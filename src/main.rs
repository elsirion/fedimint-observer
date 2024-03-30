mod error;

use anyhow::Context;
use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use error::Result;
use fedimint_core::api::InviteCode;
use fedimint_core::config::ClientConfig;

async fn fetch_federation_config(invite: Path<InviteCode>) -> Result<Json<ClientConfig>> {
    let invite = invite.0;
    let config = ClientConfig::download_from_invite_code(&invite).await?;
    Ok(config.into())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new().route("/config/:invite", get(fetch_federation_config));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("Binding to port")?;

    axum::serve(listener, app)
        .await
        .context("Starting axum server")?;

    Ok(())
}
