mod db;

use std::io::Cursor;
use std::time::{Duration, SystemTime};

use anyhow::{ensure, Context};
use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use axum_auth::AuthBearer;
use fedimint_core::api::{DynGlobalApi, InviteCode};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::{DynUnknown, ModuleInstanceId};
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::epoch::ConsensusItem;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::session_outcome::SessionOutcome;
use fedimint_core::task::TaskGroup;
use fedimint_core::util::retry;
use fedimint_core::Amount;
use fedimint_ln_common::contracts::Contract;
use fedimint_ln_common::{LightningInput, LightningOutput, LightningOutputV0};
use fedimint_mint_common::{MintInput, MintOutput};
use fedimint_wallet_common::{WalletInput, WalletOutput};
use futures::StreamExt;
use serde_json::json;
use sqlx::any::install_default_drivers;
use sqlx::pool::PoolConnection;
use sqlx::{query, query_as, Any, AnyPool, Connection, Transaction};
use tracing::log::info;
use tracing::{debug, error, warn};

use crate::config::get_decoders;
use crate::federation::db::Federation;
use crate::{federation, AppState};

pub fn get_federations_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_observed_federations))
        .route("/", put(add_observed_federation))
        .route("/:federation_id", get(get_federation_overview))
        .route(
            "/:federation_id/transactions",
            get(list_federation_transactions),
        )
        .route(
            "/:federation_id/config",
            get(federation::get_federation_config),
        )
}

pub async fn list_observed_federations(
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<FederationId>>> {
    Ok(state
        .federation_observer
        .list_federations()
        .await?
        .into_iter()
        .map(|federation| federation.config.calculate_federation_id())
        .collect::<Vec<_>>()
        .into())
}

pub async fn add_observed_federation(
    AuthBearer(auth): AuthBearer,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> crate::error::Result<Json<FederationId>> {
    state.federation_observer.check_auth(&auth)?;

    let invite: InviteCode = serde_json::from_value(
        body.get("invite")
            .context("Request did not contain invite field")?
            .clone(),
    )
    .context("Invalid invite code")?;
    Ok(state
        .federation_observer
        .add_federation(&invite)
        .await?
        .into())
}

pub(crate) async fn get_federation_config(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<ClientConfig>> {
    Ok(state
        .federation_observer
        .get_federation(federation_id)
        .await?
        .context("Federation not observed, you might want to try /config/:federation_invite")?
        .config
        .into())
}

async fn get_federation_overview(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<serde_json::Value>> {
    let session_count = state
        .federation_observer
        .federation_session_count(federation_id)
        .await?;
    let total_assets_msat = state
        .federation_observer
        .get_federation_assets(federation_id)
        .await?;

    Ok(json!({
        "session_count": session_count,
        "total_assets_msat": total_assets_msat
    })
    .into())
}

fn decoders_from_config(config: &ClientConfig) -> ModuleDecoderRegistry {
    get_decoders(
        config
            .modules
            .iter()
            .map(|(module_instance_id, module_config)| {
                (*module_instance_id, module_config.kind.clone())
            }),
    )
    .with_fallback()
}

fn instance_to_kind(config: &ClientConfig, module_instance_id: ModuleInstanceId) -> String {
    config
        .modules
        .get(&module_instance_id)
        .map(|module_config| module_config.kind.to_string())
        .unwrap_or_else(|| "not-in-config".to_owned())
}

pub async fn list_federation_transactions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<serde_json::Value>>> {
    let observer = state.federation_observer;

    let federation = observer
        .get_federation(federation_id)
        .await?
        .context("Federation not found")?;

    let decoders = decoders_from_config(&federation.config);

    let transactions = observer.list_federation_transactions(federation_id).await?;

    let transactions = transactions
        .into_iter()
        .map(|tx| {
            let inputs = tx
                .data
                .inputs
                .into_iter()
                .map(|input| {
                    let instance = input.module_instance_id();
                    let kind = instance_to_kind(&federation.config, instance);

                    let input = if let Some(decoder) = decoders.get(instance) {
                        let input = input.as_any().downcast_ref::<DynUnknown>().unwrap();
                        let mut input_bytes = Cursor::new(&input.0);
                        let _len = u64::consensus_decode(&mut input_bytes, &decoders).unwrap();

                        decoder
                            .decode(&mut input_bytes, instance, &decoders)
                            .expect("Invalid input")
                    } else {
                        input
                    };

                    json!({
                        "kind": kind.as_str(),
                        "input": input.to_string(),
                    })
                })
                .collect::<Vec<_>>();

            let outputs = tx
                .data
                .outputs
                .into_iter()
                .map(|output| {
                    let instance = output.module_instance_id();
                    let kind = federation
                        .config
                        .modules
                        .get(&instance)
                        .map(|module_config| module_config.kind.to_string())
                        .unwrap_or_else(|| "unknown".to_owned());

                    let output = if let Some(decoder) = decoders.get(instance) {
                        let output = output.as_any().downcast_ref::<DynUnknown>().unwrap();
                        let mut output_bytes = Cursor::new(&output.0);
                        let _len = u64::consensus_decode(&mut output_bytes, &decoders).unwrap();

                        decoder
                            .decode(&mut output_bytes, instance, &decoders)
                            .expect("Invalid input")
                    } else {
                        output
                    };

                    json!({
                        "kind": kind.as_str(),
                        "output": output.to_string(),
                    })
                })
                .collect::<Vec<_>>();

            json!({
                "session": tx.session_index,
                "item": tx.item_index,
                "transaction": {
                    "inputs": inputs,
                    "outputs": outputs,
                }
            })
        })
        .collect::<Vec<_>>();

    Ok(transactions.into())
}

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

        slf.setup_schema().await?;

        for federation in slf.list_federations().await? {
            slf.spawn_observer(federation).await;
        }

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

    async fn setup_schema(&self) -> anyhow::Result<()> {
        query(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schema/v0.sql"
        )))
        .execute(self.connection().await?.as_mut())
        .await?;
        Ok(())
    }

    async fn connection(&self) -> anyhow::Result<PoolConnection<Any>> {
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
                        || async {
                            api_fetch_single
                                .await_block(session_index, &decoders_single)
                                .await
                        },
                        Duration::from_secs(1),
                        u32::MAX,
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
                            _ => {
                                // FIXME: process module CIs
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
            let maybe_amount_msat = match kind.as_str() {
                "ln" => Some(
                    input
                        .as_any()
                        .downcast_ref::<LightningInput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats,
                ),
                "mint" => Some(
                    input
                        .as_any()
                        .downcast_ref::<MintInput>()
                        .expect("Not Mint input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .amount
                        .msats,
                ),
                "wallet" => Some(
                    input
                        .as_any()
                        .downcast_ref::<WalletInput>()
                        .expect("Not Wallet input")
                        .maybe_v0_ref()
                        .expect("Not v0")
                        .0
                        .tx_output()
                        .value
                        * 1000,
                ),
                _ => None,
            };

            // TODO: use for LN input, but needs ability to query previously created
            // contracts
            let subtype = Option::<String>::None;

            query("INSERT INTO transaction_inputs VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(federation_id.consensus_encode_to_vec())
                .bind(txid.consensus_encode_to_vec())
                .bind(in_idx as i64)
                .bind(kind.as_str())
                .bind(subtype)
                .bind(maybe_amount_msat.map(|amt| amt as i64))
                .execute(dbtx.as_mut())
                .await?;
        }

        for (out_idx, output) in transaction.outputs.into_iter().enumerate() {
            let kind = instance_to_kind(config, output.module_instance_id());
            let (maybe_amount_msat, maybe_subtype) = match kind.as_str() {
                "ln" => {
                    let ln_output = output
                        .as_any()
                        .downcast_ref::<LightningOutput>()
                        .expect("Not LN input")
                        .maybe_v0_ref()
                        .expect("Not v0");
                    let (amount_msat, maybe_subtype) = match ln_output {
                        LightningOutputV0::Contract(contract) => {
                            let subtype = match contract.contract {
                                Contract::Incoming(_) => "incoming",
                                Contract::Outgoing(_) => "outgoing",
                            };
                            (contract.amount.msats, Some(subtype))
                        }
                        // TODO: handle separately
                        LightningOutputV0::Offer(_) => (0, None),
                        LightningOutputV0::CancelOutgoing { .. } => (0, None),
                    };

                    (Some(amount_msat), maybe_subtype)
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

            query("INSERT INTO transaction_outputs VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(federation_id.consensus_encode_to_vec())
                .bind(txid.consensus_encode_to_vec())
                .bind(out_idx as i64)
                .bind(kind.as_str())
                .bind(maybe_subtype)
                .bind(maybe_amount_msat.map(|amt| amt as i64))
                .execute(dbtx.as_mut())
                .await?;
        }

        Ok(())
    }

    async fn federation_session_count(&self, federation_id: FederationId) -> anyhow::Result<u64> {
        let last_session =
            query_as::<_, (i64,)>("SELECT COALESCE(MAX(session_index), -1) as max_session_index FROM sessions WHERE federation_id = $1")
                .bind(federation_id.consensus_encode_to_vec())
                .fetch_one(self.connection().await?.as_mut())
                .await?.0;
        Ok((last_session + 1) as u64)
    }

    pub async fn list_federation_transactions(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<db::Transaction>> {
        Ok(query_as::<_, db::Transaction>("SELECT txid, session_index, item_index, data FROM transactions WHERE federation_id = $1")
            .bind(federation_id.consensus_encode_to_vec())
            .fetch_all(self.connection().await?.as_mut())
            .await?)
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
