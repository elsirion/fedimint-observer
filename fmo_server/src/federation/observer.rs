use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use anyhow::{ensure, Context};
use bitcoin::hashes::Hash;
use bitcoin::{Address, OutPoint, Txid};
use chrono::{DateTime, NaiveDate};
use deadpool_postgres::{GenericClient, Runtime, Transaction};
use fedimint_core::api::{DynGlobalApi, InviteCode};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::DynModuleConsensusItem;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::epoch::ConsensusItem;
use fedimint_core::session_outcome::SessionOutcome;
use fedimint_core::task::TaskGroup;
use fedimint_core::util::{retry, ConstantBackoff, FibonacciBackoff};
use fedimint_core::{Amount, PeerId};
use fedimint_ln_common::bitcoin::hashes::hex::{FromHex, ToHex};
use fedimint_ln_common::contracts::{Contract, IdentifiableContract};
use fedimint_ln_common::{LightningInput, LightningOutput, LightningOutputV0};
use fedimint_mint_common::{MintInput, MintOutput};
use fedimint_wallet_common::{WalletConsensusItem, WalletInput, WalletOutput, WalletOutputV0};
use fmo_api_types::{FederationActivity, FederationSummary, FederationUtxo, FedimintTotals};
use futures::future::join_all;
use futures::StreamExt;
use postgres_from_row::FromRow;
use tokio::time::sleep;
use tokio_postgres::NoTls;
use tracing::log::info;
use tracing::{debug, error, warn};

use crate::federation::db::Federation;
use crate::federation::{db, decoders_from_config, instance_to_kind};
use crate::util::{execute, query, query_one, query_opt, query_value};

#[derive(Debug, Clone)]
pub struct FederationObserver {
    connection_pool: deadpool_postgres::Pool,
    admin_auth: String,
    task_group: TaskGroup,
}

impl FederationObserver {
    pub async fn new(database: &str, admin_auth: &str) -> anyhow::Result<FederationObserver> {
        let connection_pool = {
            let mut pool_config = deadpool_postgres::Config::default();
            pool_config.url = Some(database.to_owned());
            pool_config.create_pool(Some(Runtime::Tokio1), NoTls)
        }?;

        let slf = FederationObserver {
            connection_pool,
            admin_auth: admin_auth.to_owned(),
            task_group: Default::default(),
        };

        slf.setup_schema().await?;

        for federation in slf.list_federations().await? {
            slf.spawn_observer(federation).await;
        }

        slf.task_group
            .spawn_cancellable("fetch block times", Self::fetch_block_times(slf.clone()));
        slf.task_group
            .spawn_cancellable("sync nostr events", Self::sync_nostr_events(slf.clone()));

        Ok(slf)
    }

    async fn spawn_observer(&self, federation: Federation) {
        let slf = self.clone();

        let federation_inner = federation.clone();
        self.task_group.spawn_cancellable(
            format!("Observer for {}", federation_inner.federation_id),
            async move {
                loop {
                    let e = slf
                        .observe_federation_history(
                            federation_inner.federation_id,
                            federation_inner.config.clone(),
                        )
                        .await
                        .expect_err("observer task exited unexpectedly");
                    error!("Observer errored, restarting in 30s: {e}");
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            },
        );

        let slf = self.clone();
        self.task_group.spawn_cancellable(
            format!("Health Monitor for {}", federation.federation_id),
            async move {
                loop {
                    let e = slf
                        .monitor_health(federation.federation_id, federation.config.clone())
                        .await
                        .expect_err("health monitor task exited unexpectedly");
                    error!("Health Monitor errored, restarting in 30s: {e}");
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            },
        );
    }

    async fn setup_schema(&self) -> anyhow::Result<()> {
        execute(
            &self.connection().await?,
            "
            CREATE OR REPLACE FUNCTION get_max_version() RETURNS INTEGER AS $$
            DECLARE
                max_version INTEGER;
            BEGIN
                IF EXISTS (
                    SELECT 1
                    FROM pg_catalog.pg_tables
                    WHERE schemaname = current_schema()
                    AND tablename = 'schema_version'
                ) THEN
                    SELECT COALESCE(MAX(version), 0) INTO max_version
                    FROM schema_version;
                ELSIF EXISTS (
                    SELECT 1
                    FROM pg_catalog.pg_tables
                    WHERE schemaname = current_schema()
                    AND tablename = 'federations'
                ) THEN
                    max_version := 0;
                ELSE
                    max_version := -1;
                END IF;

                RETURN max_version;
            END;
            $$ LANGUAGE plpgsql;
            ",
            &[],
        )
        .await?;

        let schema_version =
            query_value::<i32>(&self.connection().await?, "SELECT get_max_version();", &[]).await?;

        let migration_map = [
            (
                0,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v0.sql")),
            ),
            (
                1,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v1.sql")),
            ),
            (
                2,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v2.sql")),
            ),
            (
                3,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v3.sql")),
            ),
            (
                4,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v4.sql")),
            ),
        ];

        for (version, migration) in migration_map.iter() {
            if *version > schema_version {
                let mut conn = self.connection().await?;
                let transaction = conn.transaction().await?;
                transaction.batch_execute(migration).await?;
                self.handle_backfill(*version, &transaction).await?;
                transaction.commit().await?;
            }
        }

        if query_value::<i64>(
            &self.connection().await?,
            "SELECT COUNT(*)::bigint FROM block_times",
            &[],
        )
        .await?
            == 0
        {
            // Seed block times table
            self.connection()
                .await?
                .batch_execute(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/schema/block_times.sql"
                )))
                .await?;
        }

        Ok(())
    }

    async fn backfill_v2_migration_wallet_data(
        &self,
        dbtx: &Transaction<'_>,
    ) -> anyhow::Result<()> {
        info!("Beginning backfill for v2 wallet migration data, this may take a long time");

        let num_cpus = std::thread::available_parallelism()
            .map(|non_zero_cpus| non_zero_cpus.get())
            .unwrap_or(12);

        for fed in self.list_federations().await? {
            info!(
                "Parsing all session outcomes for fed: {}",
                fed.federation_id
            );
            let decoders = decoders_from_config(&fed.config);
            let session_outcome_rows = dbtx
                .query(
                    "SELECT * FROM sessions WHERE federation_id = $1",
                    &[&fed.federation_id.consensus_encode_to_vec()],
                )
                .await?;

            let rows_count = session_outcome_rows.len();

            // take advantage of all cores, otherwise backfilling can take a long time
            let mut parsing_stream =
                futures::stream::iter(session_outcome_rows.into_iter().enumerate())
                    .map(|(row_idx, row)| {
                        let decoders_clone = decoders.clone();
                        tokio::task::spawn(async move {
                            if row_idx % 1000 == 0 {
                                let percentage = (row_idx as f64) / (rows_count as f64);
                                let percentage_str = format!("{:.2}%", percentage * 100.0);
                                info!(
                                    "parsing session index: {:?}/{:?} ({})",
                                    row_idx, rows_count, percentage_str
                                );
                            }
                            db::SessionOutcome::from_row_with_decoders(
                                &row,
                                &decoders_clone.clone(),
                            )
                        })
                    })
                    .buffered(num_cpus)
                    .boxed();

            while let Some(outcome) = parsing_stream.next().await.transpose()? {
                self.process_session(
                    fed.federation_id,
                    fed.config.clone(),
                    outcome.session_index as u64,
                    outcome.data,
                    &dbtx,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn handle_backfill(&self, version: i32, dbtx: &Transaction<'_>) -> anyhow::Result<()> {
        match version {
            2 => Ok(self.backfill_v2_migration_wallet_data(dbtx).await?),
            _ => Ok(()),
        }
    }

    pub(super) async fn connection(&self) -> anyhow::Result<deadpool_postgres::Object> {
        Ok(self.connection_pool.get().await?)
    }

    pub async fn list_federations(&self) -> anyhow::Result<Vec<db::Federation>> {
        Ok(query(&self.connection().await?, "SELECT * FROM federations", &[]).await?)
    }

    pub async fn list_federation_summaries(&self) -> anyhow::Result<Vec<FederationSummary>> {
        let federations =
            query::<Federation>(&self.connection().await?, "SELECT * FROM federations", &[])
                .await?;

        join_all(federations.into_iter().map(|federation| async move {
            let deposits = self.get_federation_assets(federation.federation_id).await?;
            let name = federation
                .config
                .global
                .meta
                .get("federation_name")
                .cloned();

            let last_7d_activity = self
                .federation_activity(federation.federation_id, 7)
                .await?;

            let invite = federation
                .config
                .invite_code(&PeerId::from(0))
                .expect("There should always be a peer 0")
                .to_string();

            Ok(FederationSummary {
                id: federation.federation_id,
                name,
                last_7d_activity,
                deposits,
                invite,
                nostr_votes: self.federation_rating(federation.federation_id).await?,
            })
        }))
        .await
        .into_iter()
        .collect()
    }

    async fn federation_activity(
        &self,
        federation_id: FederationId,
        days: u32,
    ) -> anyhow::Result<Vec<FederationActivity>> {
        #[derive(Debug, FromRow)]
        struct FederationActivityRow {
            date: NaiveDate,
            tx_count: i64,
            total_amount: i64,
        }

        let now = chrono::offset::Utc::now();

        // language=postgresql
        let activity = query::<FederationActivityRow>(&self.connection().await?, "
            SELECT DATE(st.estimated_session_timestamp) AS date,
                   COUNT(DISTINCT t.txid)::bigint       AS tx_count,
                   COALESCE(SUM((SELECT SUM(amount_msat)
                        FROM transaction_inputs
                        WHERE transaction_inputs.txid = t.txid AND transaction_inputs.federation_id = t.federation_id))::bigint, 0)   AS total_amount
            FROM transactions t
                     JOIN
                 session_times st ON t.session_index = st.session_index AND t.federation_id = st.federation_id
            WHERE t.federation_id = $1  AND st.estimated_session_timestamp >= $2
            GROUP BY date
            ORDER BY date;
        ", &[&federation_id.consensus_encode_to_vec(), &(now - chrono::Duration::days(8)).naive_utc()]).await?;

        Ok(last_n_day_iter(now.date_naive(), days)
            .map(|date| {
                let (tx_count, total_amt) = activity
                    .iter()
                    .find(|row| row.date == date)
                    .map(|row| (row.tx_count, row.total_amount))
                    .unwrap_or((0, 0));
                FederationActivity {
                    num_transactions: tx_count as u64,
                    amount_transferred: Amount::from_msats(total_amt as u64),
                }
            })
            .collect())
    }

    pub async fn get_federation(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Option<Federation>> {
        Ok(query_opt(
            &self.connection().await?,
            "SELECT * FROM federations WHERE federation_id = $1",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?)
    }

    pub async fn add_federation(&self, invite: &InviteCode) -> anyhow::Result<FederationId> {
        let federation_id = invite.federation_id();

        if self.get_federation(federation_id).await?.is_some() {
            return Ok(federation_id);
        }

        let config = ClientConfig::download_from_invite_code(invite).await?;

        self.connection()
            .await?
            .execute(
                "INSERT INTO federations VALUES ($1, $2)",
                &[
                    &federation_id.consensus_encode_to_vec(),
                    &config.consensus_encode_to_vec(),
                ],
            )
            .await?;

        self.spawn_observer(Federation {
            federation_id,
            config,
        })
        .await;

        Ok(federation_id)
    }

    // FIXME: use middleware for auth and get it out of here
    pub fn check_auth(&self, bearer_token: &str) -> anyhow::Result<()> {
        ensure!(self.admin_auth == bearer_token, "Invalid bearer token");
        Ok(())
    }

    async fn fetch_block_times(self) {
        const SLEEP_SECS: u64 = 60;
        loop {
            if let Err(e) = self.fetch_block_times_inner().await {
                warn!("Error while fetching block times: {e:?}");
            }
            info!("Block sync finished, waiting {SLEEP_SECS} seconds");
            sleep(Duration::from_secs(SLEEP_SECS)).await;
        }
    }

    async fn fetch_block_times_inner(&self) -> anyhow::Result<()> {
        let builder = esplora_client::Builder::new("https://mempool.space/api");
        let esplora_client = builder.build_async()?;

        // TODO: find a better way to pre-seed the DB so we don't have to bother
        // blockstream.info Block 820k was mined Dec 2023, afaik there are no
        // compatible federations older than that
        let next_block_height = self.last_fetched_block_height().await?.unwrap_or(820_000) + 1;
        let current_block_height = esplora_client.get_height().await?;

        info!("Fetching block times for block {next_block_height} to {current_block_height}");

        let mut block_stream = futures::stream::iter(next_block_height..=current_block_height)
            .map(move |block_height| {
                let esplora_client_inner = esplora_client.clone();
                async move {
                    let block_hash = esplora_client_inner.get_block_hash(block_height).await?;
                    let block = esplora_client_inner.get_header_by_hash(&block_hash).await?;

                    Result::<_, anyhow::Error>::Ok((block_height, block))
                }
            })
            .buffered(4);

        let mut timer = SystemTime::now();
        let mut last_log_height = next_block_height;
        while let Some((block_height, block)) = block_stream.next().await.transpose()? {
            self.connection()
                .await?
                .execute(
                    "INSERT INTO block_times VALUES ($1, $2)",
                    &[
                        &(block_height as i32),
                        &DateTime::from_timestamp(block.time as i64, 0)
                            .expect("Invalid timestamp")
                            .naive_utc(),
                    ],
                )
                .await?;

            // TODO: write abstraction
            let elapsed = timer.elapsed().unwrap_or_default();
            if elapsed >= Duration::from_secs(5) {
                let blocks_synced = block_height - last_log_height;
                let rate = (blocks_synced as f64) / elapsed.as_secs_f64();
                info!("Synced up to block {block_height}, processed {blocks_synced} blocks at a rate of {rate:.2} blocks/s");
                timer = SystemTime::now();
                last_log_height = block_height;
            }
        }

        Ok(())
    }

    async fn last_fetched_block_height(&self) -> anyhow::Result<Option<u32>> {
        let max_height = query_value::<Option<i32>>(
            &self.connection().await?,
            "SELECT MAX(block_height) AS max_height FROM block_times",
            &[],
        )
        .await?;

        Ok(max_height.map(|max_height| max_height as u32))
    }

    async fn observe_federation_history(
        &self,
        federation_id: FederationId,
        config: ClientConfig,
    ) -> anyhow::Result<()> {
        let api = DynGlobalApi::from_config(&config);
        let decoders = decoders_from_config(&config);

        info!("Starting background job for {federation_id}");
        let next_session = self.federation_session_count(federation_id).await?;
        debug!("Next session {next_session}");
        let api_fetch = api.clone();
        let mut session_stream = futures::stream::iter(next_session..)
            .map(move |session_index| {
                debug!("Starting fetch job for session {session_index}");
                let api_fetch_single = api_fetch.clone();
                let decoders_single = decoders.clone();
                async move {
                    let signed_session_outcome = retry(
                        format!("Waiting for session {session_index}"),
                        ConstantBackoff::default()
                            .with_delay(Duration::from_secs(1))
                            .with_max_times(usize::MAX),
                        || async {
                            api_fetch_single
                                .await_block(session_index, &decoders_single)
                                .await
                        },
                    )
                    .await
                    .expect("Will fail after 136 years");
                    debug!("Finished fetch job for session {session_index}");
                    (session_index, signed_session_outcome)
                }
            })
            .buffered(32);

        let mut timer = SystemTime::now();
        let mut last_session = next_session;
        while let Some((session_index, signed_session_outcome)) = session_stream.next().await {
            let mut connection = self.connection().await?;
            let dbtx = connection.transaction().await?;
            self.process_session(
                federation_id,
                config.clone(),
                session_index,
                signed_session_outcome,
                &dbtx,
            )
            .await?;
            dbtx.commit().await?;

            let elapsed = timer.elapsed().unwrap_or_default();
            if elapsed >= Duration::from_secs(5) {
                let sessions_synced = session_index - last_session;
                let rate = (sessions_synced as f64) / elapsed.as_secs_f64();
                info!("Synced up to session {session_index}, processed {sessions_synced} sessions at a rate of {rate:.2} sessions/s");
                timer = SystemTime::now();
                last_session = session_index;

                // If we are syncing up initially we don't want to refresh the views every
                // session, later we do
                if rate < 1f64 {
                    self.refresh_views().await?;
                }
            }
        }

        unreachable!("Session stream should never end")
    }

    async fn process_session(
        &self,
        federation_id: FederationId,
        config: ClientConfig,
        session_index: u64,
        signed_session_outcome: SessionOutcome,
        dbtx: &Transaction<'_>,
    ) -> anyhow::Result<()> {
        dbtx.execute(
            "INSERT INTO sessions VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            &[
                &federation_id.consensus_encode_to_vec(),
                &(session_index as i32),
                &signed_session_outcome.consensus_encode_to_vec(),
            ],
        )
        .await?;

        for (item_idx, item) in signed_session_outcome.items.into_iter().enumerate() {
            match item.item {
                ConsensusItem::Transaction(transaction) => {
                    Self::process_transaction(
                        &dbtx,
                        federation_id,
                        &config,
                        session_index,
                        item_idx as u64,
                        transaction,
                    )
                    .await?;
                }
                ConsensusItem::Module(module_ci) => {
                    Self::process_ci(
                        &dbtx,
                        federation_id,
                        &config,
                        session_index,
                        item_idx as u64,
                        item.peer,
                        module_ci,
                    )
                    .await?;
                }
                _ => {
                    // Ignore unknown CIs
                }
            }
        }

        debug!("Processed session {session_index} of federation {federation_id}");
        Ok(())
    }

    async fn refresh_views(&self) -> anyhow::Result<()> {
        info!("Refreshing views");
        self.connection()
            .await?
            .batch_execute(
                "
                REFRESH MATERIALIZED VIEW CONCURRENTLY session_times;
                REFRESH MATERIALIZED VIEW CONCURRENTLY utxos;
                ",
            )
            .await?;

        info!("Refresh complete");

        Ok(())
    }

    async fn process_transaction(
        dbtx: &Transaction<'_>,
        federation_id: FederationId,
        config: &ClientConfig,
        session_index: u64,
        item_index: u64,
        transaction: fedimint_core::transaction::Transaction,
    ) -> Result<(), tokio_postgres::Error> {
        let fedimint_txid = transaction.tx_hash();

        dbtx.execute(
            "INSERT INTO transactions VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            &[
                &fedimint_txid.consensus_encode_to_vec(),
                &federation_id.consensus_encode_to_vec(),
                &(session_index as i32),
                &(item_index as i32),
                &transaction.consensus_encode_to_vec(),
            ],
        )
        .await?;

        for (in_idx, input) in transaction.inputs.into_iter().enumerate() {
            let kind = instance_to_kind(config, input.module_instance_id());
            let (maybe_amount_msat, maybe_ln_contract_id) = match kind.as_str() {
                "ln" => {
                    let input = input
                        .as_any()
                        .downcast_ref::<LightningInput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0");

                    (Some(input.amount.msats), Some(input.contract_id))
                }
                "mint" => {
                    let amount_msat = input
                        .as_any()
                        .downcast_ref::<MintInput>()
                        .expect("Not Mint input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats;

                    (Some(amount_msat), None)
                }
                "wallet" => {
                    let amount_msat = input
                        .as_any()
                        .downcast_ref::<WalletInput>()
                        .expect("Not Wallet input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .0
                        .tx_output()
                        .value
                        * 1000;
                    (Some(amount_msat), None)
                }
                _ => (None, None),
            };

            dbtx.execute(
                "INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
                &[
                    &federation_id.consensus_encode_to_vec(),
                    &fedimint_txid.consensus_encode_to_vec(),
                    &(in_idx as i32),
                    &kind,
                    &maybe_ln_contract_id.map(|cid| cid.consensus_encode_to_vec()),
                    &maybe_amount_msat.map(|amt| amt as i64),
                ],
            )
            .await?;

            if kind.as_str() == "wallet" {
                let peg_in_proof = &input
                    .as_any()
                    .downcast_ref::<WalletInput>()
                    .expect("Not Wallet input")
                    .maybe_v0_ref()
                    .expect("Not v0")
                    .0;

                let outpoint = peg_in_proof.outpoint();
                let on_chain_txid = fedimint_core::TransactionId::from_hex(&outpoint.txid.to_hex())
                    .expect("Invalid data in DB")
                    .consensus_encode_to_vec();

                let address = bitcoin::Address::from_script(
                    bitcoin::Script::from_bytes(peg_in_proof.tx_output().script_pubkey.as_bytes()),
                    bitcoin::Network::Bitcoin,
                )
                .expect("Invalid output address");

                dbtx.execute(
                        "INSERT INTO wallet_peg_ins VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
                        &[
                            &on_chain_txid,
                            &(outpoint.vout as i32),
                            &address.to_string(),
                            &maybe_amount_msat.map(|amt| amt as i64).expect("Wallet input must have amount"),
                            &federation_id.consensus_encode_to_vec(),
                            &fedimint_txid.consensus_encode_to_vec(),
                            &(in_idx as i32),
                        ]
                    ).await?;
            }
        }

        for (out_idx, output) in transaction.outputs.into_iter().enumerate() {
            let kind = instance_to_kind(config, output.module_instance_id());
            let (maybe_amount_msat, maybe_ln_contract) = match kind.as_str() {
                "ln" => {
                    let ln_output = output
                        .as_any()
                        .downcast_ref::<LightningOutput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0");
                    let (maybe_amount_msat, ln_contract_interaction_kind, contract_id) =
                        match ln_output {
                            LightningOutputV0::Contract(contract) => {
                                let contract_id = contract.contract.contract_id();
                                let (contract_type, payment_hash) = match &contract.contract {
                                    Contract::Incoming(c) => ("incoming", c.hash),
                                    Contract::Outgoing(c) => ("outgoing", c.hash),
                                };

                                dbtx.execute(
                                    "INSERT INTO ln_contracts VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                                    &[
                                        &federation_id.consensus_encode_to_vec(),
                                        &contract_id.consensus_encode_to_vec(),
                                        &contract_type,
                                        &payment_hash.consensus_encode_to_vec(),
                                    ],
                                )
                                .await?;

                                (Some(contract.amount.msats), "fund", contract_id)
                            }
                            LightningOutputV0::Offer(offer) => {
                                // For incoming contracts payment has == cotnract id
                                (Some(0), "offer", offer.hash.into())
                            }
                            LightningOutputV0::CancelOutgoing { contract, .. } => {
                                (Some(0), "cancel", *contract)
                            }
                        };

                    (
                        maybe_amount_msat,
                        Some((ln_contract_interaction_kind, contract_id)),
                    )
                }
                "mint" => {
                    let amount_msat = output
                        .as_any()
                        .downcast_ref::<MintOutput>()
                        .expect("Not Mint input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats;
                    (Some(amount_msat), None)
                }
                "wallet" => {
                    let amount_msat = output
                        .as_any()
                        .downcast_ref::<WalletOutput>()
                        .expect("Not Wallet input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount()
                        .to_sat()
                        * 1000;
                    (Some(amount_msat), None)
                }
                _ => (None, None),
            };

            dbtx.execute(
                "INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
                &[
                    &federation_id.consensus_encode_to_vec(),
                    &fedimint_txid.consensus_encode_to_vec(),
                    &(out_idx as i32),
                    &kind,
                    &maybe_ln_contract.map(|(kind, _id)| kind),
                    &maybe_ln_contract.map(|(_kind, id)| id.consensus_encode_to_vec()),
                    &maybe_amount_msat.map(|amt| amt as i64),
                ],
            )
            .await?;

            if kind.as_str() == "wallet" {
                let wallet_v0_output = output
                    .as_any()
                    .downcast_ref::<WalletOutput>()
                    .expect("Not Wallet input")
                    .maybe_v0_ref()
                    .expect("Not v0");

                match wallet_v0_output {
                    WalletOutputV0::PegOut(peg_out) => {
                        let withdrawal_address = peg_out.recipient.clone();
                        dbtx.execute(
                            "INSERT INTO wallet_withdrawal_addresses VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
                            &[
                                &withdrawal_address.to_string(),
                                &federation_id.consensus_encode_to_vec(),
                                &(session_index as i32),
                                &(item_index as i32),
                                &fedimint_txid.consensus_encode_to_vec(),
                                &(out_idx as i32),
                            ]
                        ).await?;
                    }
                    WalletOutputV0::Rbf(_) => {
                        // panic, since the benefits may outweigh the annoyance of removing and
                        // restarting
                        panic!(
                            r#"
                            You've discovered a terribly unfortunate situation: an RBF wallet output

                            Federation ID: {}
                            Name: {}

                            If you know any of the guardians of the federation, please give them a heads up
                            that they should expect failures re-syncing, or worse. They can reach out to the
                            core dev team on Discord (chat.fedimint.org).

                            For more context, see: https://github.com/fedimint/fedimint/pull/5496
                        "#,
                            federation_id,
                            config.global.federation_name().unwrap_or("no name defined"),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_ci(
        dbtx: &Transaction<'_>,
        federation_id: FederationId,
        config: &ClientConfig,
        session_index: u64,
        item_index: u64,
        peer_id: PeerId,
        ci: DynModuleConsensusItem,
    ) -> Result<(), tokio_postgres::Error> {
        let kind = instance_to_kind(config, ci.module_instance_id());

        if kind != "wallet" {
            return Ok(());
        }

        let wallet_ci = ci
            .as_any()
            .downcast_ref::<WalletConsensusItem>()
            .expect("config says this should be a wallet CI");
        match wallet_ci {
            WalletConsensusItem::BlockCount(height_vote) => {
                dbtx.execute(
                    "INSERT INTO block_height_votes VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
                    &[
                        &federation_id.consensus_encode_to_vec(),
                        &(session_index as i32),
                        &(item_index as i32),
                        &(peer_id.to_usize() as i32),
                        &(*height_vote as i32),
                    ],
                )
                .await?;
            }
            WalletConsensusItem::PegOutSignature(peg_out_sig) => {
                let peg_out_txid = peg_out_sig.txid.to_string();
                let peg_out_txid_encoded =
                    fedimint_core::TransactionId::from_str(peg_out_txid.as_str())
                        .expect("Invalid on chain txid")
                        .consensus_encode_to_vec();

                dbtx.execute(
                    "INSERT INTO wallet_withdrawal_transactions VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    &[
                        &peg_out_txid_encoded,
                        &federation_id.consensus_encode_to_vec(),
                    ],
                )
                .await?;

                dbtx.execute(
                    "INSERT INTO wallet_withdrawal_signatures VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    &[
                        &peg_out_txid_encoded,
                        &(session_index as i32),
                        &(item_index as i32),
                        &(peer_id.to_usize() as i32),
                    ],
                )
                .await?;

                let num_sigs = dbtx
                    .query_one(
                        "
                        SELECT COUNT(peer_id)::INT num_sigs
                        FROM wallet_withdrawal_signatures
                        WHERE on_chain_txid = $1
                        GROUP BY on_chain_txid
                        ",
                        &[&peg_out_txid_encoded],
                    )
                    .await?
                    .get::<_, i32>("num_sigs") as usize;

                // 3n + 1 <= num_peers
                // n <= (num_peers - 1) / 3
                // threshold = num_peers - floor((num_peers - 1) / 3)
                let threshold = {
                    let num_peers = config.global.api_endpoints.len();
                    num_peers - (num_peers - 1) / 3
                };

                if num_sigs < threshold {
                    return Ok(());
                }

                // at this point, the transaction reached threshold and should broadcast

                let esplora_txid = esplora_client::Txid::from_str(peg_out_txid.as_str())
                    .expect("Couldn't create esplora txid");

                let builder = esplora_client::Builder::new("https://mempool.space/api");
                let client = builder
                    .build_async()
                    .expect("Failed to build esplora client");

                let fetched_tx = retry(
                    format!("fetching tx from esplora"),
                    FibonacciBackoff::default()
                        .with_min_delay(Duration::from_secs(30))
                        .with_max_delay(Duration::from_secs(60 * 30))
                        .with_max_times(usize::MAX),
                    || async {
                        client.get_tx_no_opt(&esplora_txid).await.map_err(|e| {
                            warn!("failed to fetch tx: {e:?}");
                            anyhow::anyhow!("failed fetching tx from esplora")
                        })
                    },
                )
                .await
                .expect("Reached usize::MAX retries");

                for input in fetched_tx.input {
                    let prev_out_txid = fedimint_core::TransactionId::from_str(
                        input.previous_output.txid.to_string().as_str(),
                    )
                    .expect("Invalid txid")
                    .consensus_encode_to_vec();

                    dbtx.execute(
                        "INSERT INTO wallet_withdrawal_transaction_inputs VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                        &[
                            &prev_out_txid,
                            &(input.previous_output.vout as i32),
                            &peg_out_txid_encoded,
                        ],
                    )
                    .await?;
                }

                for (out_idx, output) in fetched_tx.output.iter().enumerate() {
                    let address = bitcoin::Address::from_script(
                        bitcoin::Script::from_bytes(output.script_pubkey.as_bytes()),
                        bitcoin::Network::Bitcoin,
                    )
                    .expect("Invalid bitcoin address");

                    dbtx.execute(
                        "INSERT INTO wallet_withdrawal_transaction_outputs VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                        &[
                            &peg_out_txid_encoded,
                            &(out_idx as i32),
                            &address.to_string(),
                            &((output.value.to_sat() as i64) * 1000),

                        ],
                    )
                    .await?;

                    // update federation_txid if we found a matching withdrawal address
                    dbtx.execute(
                        "
                        UPDATE wallet_withdrawal_transactions
                        SET federation_txid = (
                            SELECT txid
                            FROM wallet_withdrawal_addresses wwa
                            WHERE address = $1
                              AND NOT EXISTS (
                                SELECT *
                                FROM wallet_withdrawal_transactions wwt
                                WHERE wwa.txid = wwt.federation_txid
                              )
                            -- if address reuse, assume earliest withdrawal request first
                            ORDER BY session_index, item_index
                            LIMIT 1
                        )
                        WHERE on_chain_txid = $2
                          AND federation_txid IS NULL
                        ",
                        &[&address.to_string(), &peg_out_txid_encoded],
                    )
                    .await?;
                }
            }
            _ => {
                // other WalletConsesnsusItems are not needed yet
            }
        }

        Ok(())
    }

    pub async fn get_federation_assets(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Amount> {
        // Unfortunately SQLx has a bug where the integer parsing logic of the Any DB
        // type always uses signed 32bit integer decoding when receiving integer values
        // from SQLite. This is probably due to SQLite lacking the distinction between
        // integer types and just calling everything INTEGER and always using 64bit
        // representations while any other DBMS will call 64bit integers BIGINT or
        // something similar. That's why we serialize the number to a string and the
        // deserialize again in rust.
        let total_assets_msat = query_value::<i64>(
            &self.connection().await?,
            "
        SELECT
            CAST((SELECT COALESCE(SUM(amount_msat), 0)
             FROM transaction_inputs
             WHERE kind = 'wallet' AND federation_id = $1) -
            (SELECT COALESCE(SUM(amount_msat), 0)
             FROM transaction_outputs
             WHERE kind = 'wallet' AND federation_id = $1) AS BIGINT) AS net_amount_msat
        ",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?;

        Ok(Amount::from_msats(total_assets_msat as u64))
    }

    pub async fn federation_utxos(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<FederationUtxo>> {
        self.get_federation(federation_id).await?;

        #[derive(Debug, FromRow)]
        struct FederationUtxoRaw {
            on_chain_txid: Vec<u8>,
            on_chain_vout: i32,
            address: String,
            amount_msat: i64,
        }

        query::<FederationUtxoRaw>(
            &self.connection().await?,
            // language=postgresql
            "SELECT on_chain_txid, on_chain_vout, address, amount_msat FROM utxos WHERE federation_id = $1 ORDER BY amount_msat DESC",
            &[&federation_id.consensus_encode_to_vec()],
        ).await?.into_iter().map(|utxo| {
            Result::<_, anyhow::Error>::Ok(FederationUtxo {
                address: Address::from_str(&utxo.address)?,
                out_point: OutPoint {
                    txid: Txid::from_slice(&utxo.on_chain_txid)?,
                    vout: utxo.on_chain_vout.try_into()?,
                },
                amount: Amount::from_msats(utxo.amount_msat.try_into()?),
            })
        }).collect()
    }

    pub async fn totals(&self) -> anyhow::Result<FedimintTotals> {
        #[derive(Debug, FromRow)]
        struct FedimintTotalsResult {
            federations: i64,
            tx_count: i64,
            tx_volume: i64,
        }

        let totals = query_one::<FedimintTotalsResult>(
            &self.connection().await?,
            // language=postgresql
            "
                SELECT (SELECT count(*) from federations)::bigint               as federations,
                       (SELECT count(*) from transactions)::bigint               as tx_count,
                       (SELECT sum(amount_msat) from transaction_inputs)::bigint as tx_volume
            ",
            &[],
        )
        .await?;

        Ok(FedimintTotals {
            federations: totals.federations as u64,
            tx_count: totals.tx_count as u64,
            tx_volume: Amount::from_msats(totals.tx_volume as u64),
        })
    }

    pub async fn duplicate_ecash(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<BTreeMap<[u8; 48], Vec<(u64, u64, Amount)>>> {
        #[derive(FromRow)]
        struct TransactionRow {
            session_index: i32,
            item_index: i32,
            data: Vec<u8>,
        }

        let federation = self
            .get_federation(federation_id)
            .await?
            .context("Federation not found")?;
        let decoders = decoders_from_config(&federation.config);
        let mint_module_id = federation
            .config
            .modules
            .iter()
            .find_map(|(module_instance_id, module_cfg)| {
                if module_cfg.kind.as_str() == "mint" {
                    Some(*module_instance_id)
                } else {
                    None
                }
            })
            .expect("Mint module not found");

        let raw_transactions: Vec<TransactionRow> = query(
            &self.connection().await?,
            "SELECT session_index, item_index, data FROM transactions WHERE federation_id = $1",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?;

        let notes_iter = raw_transactions.into_iter().flat_map(|row| {
            let transaction: fedimint_core::transaction::Transaction =
                Decodable::consensus_decode_vec(row.data, &decoders)
                    .expect("Our DB should only contain valid transactions");
            transaction.outputs.into_iter().filter_map(move |output| {
                if output.module_instance_id() == mint_module_id {
                    let mint_output = output
                        .as_any()
                        .downcast_ref::<MintOutput>()
                        .expect("Not mint output");
                    let mint_output_v0 = mint_output.maybe_v0_ref().expect("Not v0");
                    let blind_nonce = mint_output_v0.blind_nonce;
                    let amount = mint_output_v0.amount;
                    Some((
                        row.session_index as u64,
                        row.item_index as u64,
                        blind_nonce,
                        amount,
                    ))
                } else {
                    None
                }
            })
        });

        let mut ecash_map = BTreeMap::<[u8; 48], Vec<(u64, u64, Amount)>>::new();
        for (session_index, item_index, blind_nonce, amount) in notes_iter {
            ecash_map
                .entry(blind_nonce.0 .0.to_compressed())
                .or_default()
                .push((session_index, item_index, amount));
        }

        Ok(ecash_map
            .into_iter()
            .filter(|(_, issuances)| issuances.len() != 1)
            .collect())
    }
}

fn last_n_day_iter(now: NaiveDate, days: u32) -> impl Iterator<Item = NaiveDate> {
    (0..days)
        .rev()
        .map(move |day| now - chrono::Duration::days(day as i64))
}

#[cfg(test)]
mod tests {
    use crate::federation::observer::last_n_day_iter;

    #[test]
    fn test_day_iter() {
        let now = chrono::offset::Utc::now().date_naive();
        let days = 7;
        let last_7_days = last_n_day_iter(now, days).collect::<Vec<_>>();
        assert_eq!(last_7_days.len(), days as usize);
        assert_eq!(last_7_days[6], now);
        assert_eq!(last_7_days[0], now - chrono::Duration::days(6));
    }
}
