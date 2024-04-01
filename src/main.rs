use anyhow::Context;
use axum::routing::{get, put};
use axum::Router;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::id::fetch_federation_id;
use crate::config::meta::{fetch_federation_meta, MetaOverrideCache};
use crate::config::modules::fetch_federation_module_kinds;
use crate::config::{fetch_federation_config, FederationConfigCache};
use crate::federation::{
    add_observed_federation, list_federation_transactions, list_observed_federations,
    FederationObserver,
};

/// Fedimint config fetching service implementation
mod config;
/// `anyhow`-based error handling for axum
mod error;
mod federation;

#[derive(Debug, Clone)]
struct AppState {
    federation_config_cache: FederationConfigCache,
    meta_override_cache: MetaOverrideCache,
    federation_observer: FederationObserver,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("info".parse().unwrap())
                .from_env()
                .unwrap(),
        )
        .init();

    let bind_address = dotenv::var("FO_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    info!("Starting API server on {bind_address}");

    let app = Router::new()
        .route("/health", get(|| async { "Server is up and running!" }))
        .route("/config/:invite", get(fetch_federation_config))
        .route("/config/:invite/meta", get(fetch_federation_meta))
        .route("/config/:invite/id", get(fetch_federation_id))
        .route(
            "/config/:invite/module_kinds",
            get(fetch_federation_module_kinds),
        )
        .route("/federations", get(list_observed_federations))
        .route("/federations", put(add_observed_federation))
        .route(
            "/federations/:federation_id/transactions",
            get(list_federation_transactions),
        )
        .with_state(AppState {
            federation_config_cache: Default::default(),
            meta_override_cache: Default::default(),
            federation_observer: FederationObserver::new(
                &dotenv::var("FO_DATABASE")
                    .unwrap_or_else(|_| "sqlite://fedimint_observer.db".to_owned()),
                &dotenv::var("FO_ADMIN_AUTH").context("No FO_ADMIN_AUTH provided")?,
            )
            .await?,
        });

    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .context("Binding to port")?;

    axum::serve(listener, app)
        .await
        .context("Starting axum server")?;

    Ok(())
}
