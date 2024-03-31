use anyhow::Context;
use axum::routing::get;
use axum::Router;

use crate::config::id::fetch_federation_id;
use crate::config::meta::{fetch_federation_meta, MetaOverrideCache};
use crate::config::modules::fetch_federation_module_kinds;
use crate::config::{fetch_federation_config, FederationConfigCache};

/// Fedimint config fetching service implementation
mod config;
/// `anyhow`-based error handling for axum
mod error;

#[derive(Debug, Clone, Default)]
struct AppState {
    federation_config_cache: FederationConfigCache,
    meta_override_cache: MetaOverrideCache,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(|| async { "Server is up and running!" }))
        .route("/config/:invite", get(fetch_federation_config))
        .route("/config/:invite/meta", get(fetch_federation_meta))
        .route("/config/:invite/id", get(fetch_federation_id))
        .route(
            "/config/:invite/module_kinds",
            get(fetch_federation_module_kinds),
        )
        .with_state(AppState::default());

    let listener = tokio::net::TcpListener::bind(
        std::env::var("FO_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned()),
    )
    .await
    .context("Binding to port")?;

    axum::serve(listener, app)
        .await
        .context("Starting axum server")?;

    Ok(())
}
