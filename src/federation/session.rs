use std::collections::BTreeMap;

use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::config::FederationId;
use fedimint_core::encoding::Encodable;
use serde_json::json;
use sqlx::query_as;

use crate::federation::observer::FederationObserver;
use crate::AppState;

pub(super) async fn list_sessions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<BTreeMap<u64, serde_json::Value>>> {
    Ok(state
        .federation_observer
        .federation_session_list(federation_id)
        .await?
        .into_iter()
        .map(|session| {
            (
                session.session_index,
                json!({ "transactions": session.transaction_count }),
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into())
}

pub(super) async fn count_sessions(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<u64>> {
    Ok(state
        .federation_observer
        .federation_session_count(federation_id)
        .await?
        .into())
}

pub struct SessionData {
    pub session_index: u64,
    pub transaction_count: u64,
}

impl FederationObserver {
    pub async fn federation_session_list(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<SessionData>> {
        self.get_federation(federation_id)
            .await
            .context("Federation doesn't exist")?;

        Ok(query_as::<_, (i64, i64)>("
            SELECT s.session_index, COUNT(t.txid) AS transaction_count
            FROM sessions AS s
            LEFT JOIN transactions AS t ON s.federation_id = t.federation_id AND s.session_index = t.session_index
            WHERE s.federation_id = $1
            GROUP BY s.session_index
            ORDER BY s.session_index ASC
        ")
            .bind(federation_id.consensus_encode_to_vec())
            .fetch_all(self.connection().await?.as_mut())
            .await?.into_iter().map(|(session_index, transaction_count)| SessionData {
            session_index: session_index as u64,
            transaction_count: transaction_count as u64,
        }).collect())
    }

    pub async fn federation_session_count(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<u64> {
        let session_count =
            query_as::<_, (i64,)>("SELECT COALESCE(COUNT(session_index), 0) as max_session_index FROM sessions WHERE federation_id = $1")
                .bind(federation_id.consensus_encode_to_vec())
                .fetch_one(self.connection().await?.as_mut())
                .await?.0;
        Ok(session_count as u64)
    }
}
