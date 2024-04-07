use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::config::FederationId;
use fedimint_core::encoding::Encodable;
use fedimint_core::TransactionId;
use sqlx::query_as;

use crate::federation::db;
use crate::federation::observer::FederationObserver;
use crate::AppState;

pub(super) async fn list_transactions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<TransactionId>>> {
    Ok(state
        .federation_observer
        .list_federation_transactions(federation_id)
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
        .count_federation_transactions(federation_id)
        .await?
        .into())
}

impl FederationObserver {
    pub async fn list_federation_transactions(
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

    pub async fn count_federation_transactions(
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
}
