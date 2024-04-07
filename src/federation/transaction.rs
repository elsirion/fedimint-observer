use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::config::FederationId;
use fedimint_core::TransactionId;

use crate::AppState;

pub(super) async fn list_transactions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<TransactionId>>> {
    Ok(state
        .federation_observer
        .list_federation_transactions(federation_id)
        .await
        .context("Federation not observed, you might want to try /config/:federation_invite")?
        .into_iter()
        .map(|tx| tx.txid)
        .collect::<Vec<_>>()
        .into())
}
