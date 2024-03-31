mod db;

use anyhow::{ensure, Context};
use axum::extract::State;
use axum::Json;
use axum_auth::AuthBearer;
use fedimint_core::api::InviteCode;
use fedimint_core::config::{ClientConfig, FederationId};
use sqlx::any::install_default_drivers;
use sqlx::pool::PoolConnection;
use sqlx::{query, query_as, Any, AnyConnection, AnyPool};

use crate::federation::db::Federation;
use crate::AppState;

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

#[derive(Debug, Clone)]
pub struct FederationObserver {
    connection_pool: AnyPool,
    admin_auth: String,
}

impl FederationObserver {
    pub async fn new(database: &str, admin_auth: &str) -> anyhow::Result<FederationObserver> {
        install_default_drivers();

        let connection_pool = sqlx::AnyPool::connect(database).await?;
        Self::setup_schema(connection_pool.acquire().await?.as_mut()).await?;

        Ok(FederationObserver {
            connection_pool,
            admin_auth: admin_auth.to_owned(),
        })
    }

    async fn setup_schema(connection: &mut AnyConnection) -> anyhow::Result<()> {
        query(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schema/v0.sql"
        )))
        .execute(connection)
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
                .bind(federation_id.to_string())
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
        let config_json =
            serde_json::to_string(&config).expect("Config could be decoded, so can be encoded");

        query("INSERT INTO federations VALUES ($1, $2)")
            .bind(federation_id.to_string())
            .bind(config_json)
            .execute(self.connection().await?.as_mut())
            .await?;

        Ok(federation_id)
    }

    // FIXME: use middleware for auth and get it out of here
    pub fn check_auth(&self, bearer_token: &str) -> anyhow::Result<()> {
        ensure!(&self.admin_auth == bearer_token, "Invalid bearer token");
        Ok(())
    }
}
