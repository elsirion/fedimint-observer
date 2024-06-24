use std::time::{Duration, SystemTime};

use anyhow::{bail, ensure};
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
use sqlx::any::install_default_drivers;
use sqlx::pool::PoolConnection;
use sqlx::{query, query_as, Any, AnyPool, Connection, Row, Transaction};
use tokio::time::sleep;
use tracing::log::info;
use tracing::{debug, error, warn};

use crate::federation::db::Federation;
use crate::federation::{db, decoders_from_config, instance_to_kind};

#[derive(Debug, Clone)]
pub struct FederationObserver {
    connection_pool: AnyPool,
    admin_auth: String,
    task_group: TaskGroup,
}

impl FederationObserver {
    pub async fn new(database: &str, admin_auth: &str) -> anyhow::Result<FederationObserver> {
        install_default_drivers();
        let connection_pool = sqlx::AnyPool::connect(database).await?;

        let slf = FederationObserver {
            connection_pool,
            admin_auth: admin_auth.to_owned(),
            task_group: Default::default(),
        };

        slf.setup_schema(database).await?;

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
                if let Err(e) = slf
                    .observe_federation(federation.federation_id, federation.config)
                    .await
                {
                    error!("Observer errored: {e:?}");
                }
            },
        );
    }

    async fn setup_schema(&self, database: &str) -> anyhow::Result<()> {
        let create_schema = if database.starts_with("sqlite") {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/v0.sqlite.sql"))
        } else if database.starts_with("postgres") {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schema/v0.postgres.sql"
            ))
        } else {
            bail!("Unsupported database, use sqlite or postgres")
        };
        sqlx::raw_sql(create_schema)
            .execute(self.connection().await?.as_mut())
            .await?;
        Ok(())
    }

    pub(super) async fn connection(&self) -> anyhow::Result<PoolConnection<Any>> {
        Ok(self.connection_pool.acquire().await?)
    }

    pub async fn list_federations(&self) -> anyhow::Result<Vec<db::Federation>> {
        Ok(query_as::<_, db::Federation>("SELECT * FROM federations")
            .fetch_all(self.connection().await?.as_mut())
            .await?)
    }

    pub async fn get_federation(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Option<Federation>> {
        Ok(
            query_as::<_, db::Federation>("SELECT * FROM federations WHERE federation_id = $1")
                .bind(federation_id.consensus_encode_to_vec())
                .fetch_optional(self.connection().await?.as_mut())
                .await?,
        )
    }

    pub async fn add_federation(&self, invite: &InviteCode) -> anyhow::Result<FederationId> {
        let federation_id = invite.federation_id();

        if self.get_federation(federation_id).await?.is_some() {
            return Ok(federation_id);
        }

        let config = ClientConfig::download_from_invite_code(invite).await?;

        query("INSERT INTO federations VALUES ($1, $2)")
            .bind(federation_id.consensus_encode_to_vec())
            .bind(config.consensus_encode_to_vec())
            .execute(self.connection().await?.as_mut())
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
        let builder = esplora_client::Builder::new("https://blockstream.info/api");
        let esplora_client = builder.build_async()?;

        // TODO: find a better way to pre-seed the DB so we don't have to bother
        // blockstream.info Block 820k was mined Dec 2023, afaik there are no
        // compatible federations older than that
        let next_block_height =
            (self.last_fetched_block_height().await?.unwrap_or(820_000) + 1) as u32;
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
            query("INSERT INTO block_times VALUES ($1, $2)")
                .bind(block_height as i64)
                .bind(block.time as i64)
                .execute(self.connection().await?.as_mut())
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

    async fn last_fetched_block_height(&self) -> anyhow::Result<Option<u64>> {
        let row = query("SELECT MAX(block_height) AS max_height FROM block_times")
            .fetch_one(self.connection().await?.as_mut())
            .await?;

        Ok(row
            .try_get::<i64, _>("max_height")
            .ok()
            .map(|max_height| max_height as u64))
    }

    async fn observe_federation(
        self,
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
        self.connection()
            .await?
            .transaction(|dbtx: &mut Transaction<Any>| {
                Box::pin(async move {
                    query("INSERT INTO sessions VALUES ($1, $2, $3)")
                        .bind(federation_id.consensus_encode_to_vec())
                        .bind(session_index as i64)
                        .bind(signed_session_outcome.consensus_encode_to_vec())
                        .execute(dbtx.as_mut())
                        .await?;

                    for (item_idx, item) in signed_session_outcome.items.into_iter().enumerate() {
                        match item.item {
                            ConsensusItem::Transaction(transaction) => {
                                Self::process_transaction(
                                    dbtx,
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
                                    dbtx,
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

                    Result::<(), sqlx::Error>::Ok(())
                })
            })
            .await?;

        debug!("Processed session {session_index} of federation {federation_id}");
        Ok(())
    }

    async fn process_transaction(
        dbtx: &mut Transaction<'_, Any>,
        federation_id: FederationId,
        config: &ClientConfig,
        session_index: u64,
        item_index: u64,
        transaction: fedimint_core::transaction::Transaction,
    ) -> sqlx::Result<()> {
        let txid = transaction.tx_hash();

        query("INSERT INTO transactions VALUES ($1, $2, $3, $4, $5)")
            .bind(txid.consensus_encode_to_vec())
            .bind(federation_id.consensus_encode_to_vec())
            .bind(session_index as i64)
            .bind(item_index as i64)
            .bind(transaction.consensus_encode_to_vec())
            .execute(dbtx.as_mut())
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

            // FIXME: Currently, sqlx + postgres is unable to handle passing NULL when
            // ln_contract_id is None. This may be due to an issue with the
            // driver or library. Revisit this after future sqlx updates
            // to see if the issue has been resolved.
            if let Some(ln_contract_id) =
                maybe_ln_contract_id.map(|cid| cid.consensus_encode_to_vec())
            {
                query("INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, $5, $6)")
                    .bind(federation_id.consensus_encode_to_vec())
                    .bind(txid.consensus_encode_to_vec())
                    .bind(in_idx as i64)
                    .bind(kind.as_str())
                    .bind(ln_contract_id)
                    .bind(maybe_amount_msat.map(|amt| amt as i64))
                    .execute(dbtx.as_mut())
                    .await?;
            } else {
                query("INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, NULL, $5)")
                    .bind(federation_id.consensus_encode_to_vec())
                    .bind(txid.consensus_encode_to_vec())
                    .bind(in_idx as i64)
                    .bind(kind.as_str())
                    .bind(maybe_amount_msat.map(|amt| amt as i64))
                    .execute(dbtx.as_mut())
                    .await?;
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

                                query("INSERT INTO ln_contracts VALUES ($1, $2, $3, $4)")
                                    .bind(federation_id.consensus_encode_to_vec())
                                    .bind(contract_id.consensus_encode_to_vec())
                                    .bind(contract_type)
                                    .bind(payment_hash.consensus_encode_to_vec())
                                    .execute(dbtx.as_mut())
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

            // FIXME: Currently, sqlx + postgres is unable to handle passing NULL when
            // ln_contract_id is None. This may be due to an issue with the
            // driver or library. Revisit this after future sqlx updates
            // to see if the issue has been resolved.
            if let Some(ln_contract_id) = maybe_ln_contract.map(|cd| cd.1.consensus_encode_to_vec())
            {
                query("INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind(federation_id.consensus_encode_to_vec())
                    .bind(txid.consensus_encode_to_vec())
                    .bind(out_idx as i64)
                    .bind(kind.as_str())
                    .bind(maybe_ln_contract.map(|cd| cd.0))
                    .bind(ln_contract_id)
                    .bind(maybe_amount_msat.map(|amt| amt as i64))
                    .execute(dbtx.as_mut())
                    .await?;
            } else {
                query("INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, NULL, $6)")
                    .bind(federation_id.consensus_encode_to_vec())
                    .bind(txid.consensus_encode_to_vec())
                    .bind(out_idx as i64)
                    .bind(kind.as_str())
                    .bind(maybe_ln_contract.map(|cd| cd.0))
                    .bind(maybe_amount_msat.map(|amt| amt as i64))
                    .execute(dbtx.as_mut())
                    .await?;
            }
        }

        Ok(())
    }

    async fn process_ci(
        dbtx: &mut Transaction<'_, Any>,
        federation_id: FederationId,
        config: &ClientConfig,
        session_index: u64,
        item_index: u64,
        peer_id: PeerId,
        ci: DynModuleConsensusItem,
    ) -> sqlx::Result<()> {
        let kind = instance_to_kind(config, ci.module_instance_id());

        if kind != "wallet" {
            return Ok(());
        }

        let wallet_ci = ci
            .as_any()
            .downcast_ref::<WalletConsensusItem>()
            .expect("config says this should be a wallet CI");
        if let WalletConsensusItem::BlockCount(height_vote) = wallet_ci {
            query("INSERT INTO block_height_votes VALUES ($1, $2, $3, $4, $5)")
                .bind(federation_id.consensus_encode_to_vec())
                .bind(session_index as i64)
                .bind(item_index as i64)
                .bind(peer_id.to_usize() as i64)
                .bind(*height_vote as i64)
                .execute(dbtx.as_mut())
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
        let total_assets_msat = query_as::<_, (String,)>(
            "
        SELECT
            CAST((SELECT COALESCE(SUM(amount_msat), 0)
             FROM transaction_inputs
             WHERE kind = 'wallet' AND federation_id = $1) -
            (SELECT COALESCE(SUM(amount_msat), 0)
             FROM transaction_outputs
             WHERE kind = 'wallet' AND federation_id = $1) AS TEXT) AS net_amount_msat
        ",
        )
        .bind(federation_id.consensus_encode_to_vec())
        .fetch_one(self.connection().await?.as_mut())
        .await?
        .0;

        Ok(Amount::from_msats(
            total_assets_msat.parse().expect("DB returns valid number"),
        ))
    }
}
