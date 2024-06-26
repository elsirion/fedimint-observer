use std::collections::BTreeMap;
use std::io::Cursor;

use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::config::FederationId;
use fedimint_core::core::{DynInput, DynOutput, DynUnknown};
use fedimint_core::encoding::Encodable;
use fedimint_core::TransactionId;
use serde::Serialize;
use serde_json::json;
use sqlx::query_as;

use crate::federation::db;
use crate::federation::observer::FederationObserver;
use crate::util::get_decoders;
use crate::AppState;

pub(super) async fn list_transactions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<TransactionId>>> {
    Ok(state
        .federation_observer
        .federation_transaction_list(federation_id)
        .await?
        .into_iter()
        .map(|tx| tx.txid)
        .collect::<Vec<_>>()
        .into())
}

pub(super) async fn count_transactions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<u64>> {
    Ok(state
        .federation_observer
        .federation_transaction_count(federation_id)
        .await?
        .into())
}

pub(super) async fn transaction(
    Path((federation_id, transaction_id)): Path<(FederationId, TransactionId)>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<DebugTransaction>> {
    Ok(state
        .federation_observer
        .transaction_details(federation_id, transaction_id)
        .await?
        .into())
}

pub(super) async fn transaction_histogram(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<BTreeMap<String, serde_json::Value>>> {
    Ok(state
        .federation_observer
        .transaction_histogram(federation_id)
        .await?
        .into_iter()
        .map(|(date, count, amount)| {
            (
                date,
                json!({
                    "count": count,
                    "amount_msat": amount
                }),
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into())
}

impl FederationObserver {
    pub async fn federation_transaction_list(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<db::Transaction>> {
        self.get_federation(federation_id)
            .await
            .context("Federation doesn't exist")?;

        Ok(query_as::<_, db::Transaction>("SELECT txid, session_index, item_index, data FROM transactions WHERE federation_id = $1")
            .bind(federation_id.consensus_encode_to_vec())
            .fetch_all(self.connection().await?.as_mut())
            .await?)
    }

    pub async fn federation_transaction_count(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<u64> {
        self.get_federation(federation_id)
            .await
            .context("Federation doesn't exist")?;

        Ok(query_as::<_, (i64,)>(
            "SELECT COALESCE(COUNT(txid), 0) FROM transactions WHERE federation_id = $1",
        )
        .bind(federation_id.consensus_encode_to_vec())
        .fetch_one(self.connection().await?.as_mut())
        .await?
        .0 as u64)
    }

    pub async fn transaction_details(
        &self,
        federation_id: FederationId,
        transaction_id: TransactionId,
    ) -> anyhow::Result<DebugTransaction> {
        let cfg = self
            .get_federation(federation_id)
            .await?
            .context("Federation doesn't exist")?
            .config;

        let tx= query_as::<_, db::Transaction>("SELECT txid, session_index, item_index, data FROM transactions WHERE federation_id = $1 AND txid = $2")
            .bind(federation_id.consensus_encode_to_vec())
            .bind(transaction_id.consensus_encode_to_vec())
            .fetch_one(self.connection().await?.as_mut())
            .await?;

        let decoders = get_decoders(
            cfg.modules
                .into_iter()
                .map(|(module_instance_id, module_cfg)| (module_instance_id, module_cfg.kind)),
        );

        let inputs = tx
            .data
            .inputs
            .into_iter()
            .map(|input| {
                let module_instance_id = input.module_instance_id();
                let undecoded = input
                    .as_any()
                    .downcast_ref::<DynUnknown>()
                    .expect("Shouldn't be decoded yet");
                decoders
                    .get(module_instance_id)
                    .map(|decoder| {
                        decoder
                            .decode::<DynInput>(
                                &mut Cursor::new(&undecoded.0),
                                module_instance_id,
                                &Default::default(),
                            )
                            .expect("decoding failed")
                    })
                    .map(|input| format!("{input:?}"))
                    .unwrap_or_else(|| format!("Unknown module, instance id={module_instance_id}"))
            })
            .collect::<Vec<_>>();

        let outputs = tx
            .data
            .outputs
            .into_iter()
            .map(|output| {
                let module_instance_id = output.module_instance_id();
                let undecoded = output
                    .as_any()
                    .downcast_ref::<DynUnknown>()
                    .expect("Shouldn't be decoded yet");
                decoders
                    .get(module_instance_id)
                    .map(|decoder| {
                        decoder
                            .decode::<DynOutput>(
                                &mut Cursor::new(&undecoded.0),
                                module_instance_id,
                                &Default::default(),
                            )
                            .expect("decoding failed")
                    })
                    .map(|output| format!("{output:?}"))
                    .unwrap_or_else(|| format!("Unknown module, instance id={module_instance_id}"))
            })
            .collect::<Vec<_>>();

        Ok(DebugTransaction { inputs, outputs })
    }

    pub async fn transaction_histogram(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<(String, u64, u64)>> {
        const QUERY: &str = "
            SELECT DATE(datetime(st.estimated_session_timestamp, 'unixepoch')) AS calendar_day,
                   COUNT(DISTINCT t.txid)                                      AS transaction_count,
                   CAST(SUM(ti.total_input_amount) AS TEXT)                    AS transaction_amt
            FROM transactions t
                     JOIN
                 session_times st ON t.session_index = st.session_index AND t.federation_id = st.federation_id
                     JOIN
                 (SELECT federation_id,
                         txid,
                         SUM(amount_msat) AS total_input_amount
                  FROM transaction_inputs
                  GROUP BY txid, federation_id) ti ON t.txid = ti.txid AND t.federation_id = ti.federation_id
            WHERE t.federation_id = $1
            GROUP BY calendar_day
            ORDER BY calendar_day;
        ";

        // Check federation exists
        let _federation = self
            .get_federation(federation_id)
            .await?
            .context("Federation doesn't exist")?;

        let histogram = query_as::<_, (String, i64, String)>(QUERY)
            .bind(federation_id.consensus_encode_to_vec())
            .fetch_all(self.connection().await?.as_mut())
            .await?
            .into_iter()
            .map(|(date, cnt, amt)| (date, cnt as u64, amt.parse().expect("is a number")))
            .collect();

        Ok(histogram)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugTransaction {
    inputs: Vec<String>,
    outputs: Vec<String>,
}
