pub mod db;
pub mod observer;
mod transaction;

use anyhow::Context;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use axum_auth::AuthBearer;
use fedimint_core::api::InviteCode;
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::ModuleInstanceId;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use serde_json::json;
use tracing::debug;

use crate::config::get_decoders;
use crate::federation::observer::QueryResult;
use crate::federation::transaction::{count_transactions, list_transactions};
use crate::{federation, AppState};

pub fn get_federations_routes() -> Router<AppState> {
    Router::new()
        .route("/query", post(run_query))
        .route("/query", get(|| async { Html(include_str!("query.html")) }))
        .route("/", get(list_observed_federations))
        .route("/", put(add_observed_federation))
        .route("/:federation_id", get(get_federation_overview))
        .route(
            "/:federation_id/config",
            get(federation::get_federation_config),
        )
        .route("/:federation_id/transactions", get(list_transactions))
        .route(
            "/:federation_id/transactions/count",
            get(count_transactions),
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

async fn run_query(
    AuthBearer(auth): AuthBearer,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> crate::error::Result<Json<QueryResult>> {
    let observer = state.federation_observer;

    observer.check_auth(&auth)?;

    let query = body
        .get("query")
        .context("No query provided")?
        .as_str()
        .context("Query parameter wasn't a string")?;
    debug!("Running query: {query}");
    let result = observer.run_qery(query).await?;
    debug!("Query result: {result:?}");

    Ok(result.into())
}
