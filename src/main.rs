use anyhow::Context;
use axum::routing::get;
use axum::Router;

use crate::config::{fetch_federation_config, FederationConfigCache};

/// Fedimint config fetching service implementation
mod config;
/// `anyhow`-based error handling for axum
mod error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/config/:invite", get(fetch_federation_config))
        .with_state(FederationConfigCache::default());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("Binding to port")?;

    axum::serve(listener, app)
        .await
        .context("Starting axum server")?;

    Ok(())
}
