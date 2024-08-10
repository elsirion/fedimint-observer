use std::time::{Duration, SystemTime};

use anyhow::ensure;
use chrono::DateTime;
use deadpool_postgres::{Runtime, Transaction};
use fedimint_core::api::{DynGlobalApi, InviteCode};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::DynModuleConsensusItem;
use fedimint_core::encoding::Encodable;
use fedimint_core::epoch::ConsensusItem;
use fedimint_core::session_outcome::SessionOutcome;
use fedimint_core::task::TaskGroup;
use fedimint_core::util::{retry, ConstantBackoff};
use fedimint_core::{Amount, PeerId};
use fedimint_ln_common::contracts::{Contract, IdentifiableContract};
use fedimint_ln_common::{LightningInput, LightningOutput, LightningOutputV0};
use fedimint_mint_common::{MintInput, MintOutput};
use fedimint_wallet_common::{WalletConsensusItem, WalletInput, WalletOutput};
use futures::StreamExt;
use tokio::time::sleep;
use tokio_postgres::NoTls;
use tracing::log::info;
use tracing::{debug, error, warn};

use crate::federation::db::Federation;
use crate::federation::{db, decoders_from_config, instance_to_kind};
use crate::util::{execute, query, query_opt, query_value};

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

        Ok(slf)
    }

    async fn spawn_observer(&self, federation: Federation) {
        let slf = self.clone();
        self.task_group.spawn_cancellable(
            format!("Observer for {}", federation.federation_id),
            async move {
                loop {
                    let e = slf
                        .observe_federation(federation.federation_id, federation.config.clone())
                        .await
                        .expect_err("observer task exited unexpectedly");
                    error!("Observer errored, restarting in 30s: {e:?}");
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
                    WHERE schemaname = 'public' 
                    AND tablename = 'schema_version'
                ) THEN
                    SELECT COALESCE(MAX(version), 0) INTO max_version
                    FROM schema_version;
                ELSE
                    max_version := 0;
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
        ];

        for (version, migration) in migration_map.iter() {
            if *version > schema_version {
                let mut conn = self.connection().await?;
                let transaction = conn.transaction().await?;
                transaction.batch_execute(migration).await?;
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

    pub(super) async fn connection(&self) -> anyhow::Result<deadpool_postgres::Object> {
        Ok(self.connection_pool.get().await?)
    }

    pub async fn list_federations(&self) -> anyhow::Result<Vec<db::Federation>> {
        Ok(query(&self.connection().await?, "SELECT * FROM federations", &[]).await?)
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

    async fn observe_federation(
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
            self.process_session(
                federation_id,
                config.clone(),
                session_index,
                signed_session_outcome,
            )
            .await?;

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
    ) -> anyhow::Result<()> {
        let mut connection = self.connection().await?;
        let dbtx = connection.transaction().await?;

        dbtx.execute(
            "INSERT INTO sessions VALUES ($1, $2, $3)",
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

        dbtx.commit().await?;

        debug!("Processed session {session_index} of federation {federation_id}");
        Ok(())
    }

    async fn refresh_views(&self) -> anyhow::Result<()> {
        info!("Refreshing views");
        self.connection()
            .await?
            .batch_execute("REFRESH MATERIALIZED VIEW CONCURRENTLY session_times;")
            .await?;

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
        let txid = transaction.tx_hash();

        dbtx.execute(
            "INSERT INTO transactions VALUES ($1, $2, $3, $4, $5)",
            &[
                &txid.consensus_encode_to_vec(),
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
                "INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &federation_id.consensus_encode_to_vec(),
                    &txid.consensus_encode_to_vec(),
                    &(in_idx as i32),
                    &kind,
                    &maybe_ln_contract_id.map(|cid| cid.consensus_encode_to_vec()),
                    &maybe_amount_msat.map(|amt| amt as i64),
                ],
            )
            .await?;
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
                                    "INSERT INTO ln_contracts VALUES ($1, $2, $3, $4)",
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
                "INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &federation_id.consensus_encode_to_vec(),
                    &txid.consensus_encode_to_vec(),
                    &(out_idx as i32),
                    &kind,
                    &maybe_ln_contract.map(|(kind, _id)| kind),
                    &maybe_ln_contract.map(|(_kind, id)| id.consensus_encode_to_vec()),
                    &maybe_amount_msat.map(|amt| amt as i64),
                ],
            )
            .await?;
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
        if let WalletConsensusItem::BlockCount(height_vote) = wallet_ci {
            dbtx.execute(
                "INSERT INTO block_height_votes VALUES ($1, $2, $3, $4, $5)",
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
}
