use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveDateTime;
use fedimint_core::config::FederationId;
use fedimint_core::encoding::Encodable;
use fmo_api_types::{FederationGateways, GatewayFees, GatewayInfo};
use postgres_from_row::FromRow;

use crate::util::query;
use crate::AppState;

#[derive(Debug, Clone, FromRow)]
struct GatewayRow {
    gateway_id: Vec<u8>,
    node_pub_key: Vec<u8>,
    api_endpoint: String,
    base_fee_msat: i64,
    proportional_fee_millionths: i32,
    supports_private_payments: bool,
    registered_at: NaiveDateTime,
    expires_at: NaiveDateTime,
    seconds_until_expiry: i32,
}

impl From<GatewayRow> for GatewayInfo {
    fn from(row: GatewayRow) -> Self {
        GatewayInfo {
            gateway_id: hex::encode(&row.gateway_id),
            node_pub_key: hex::encode(&row.node_pub_key),
            api_endpoint: row.api_endpoint,
            fees: GatewayFees {
                base_msat: row.base_fee_msat as u64,
                proportional_millionths: row.proportional_fee_millionths as u32,
            },
            supports_private_payments: row.supports_private_payments,
            registered_at: row.registered_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            expires_at: row.expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            seconds_until_expiry: row.seconds_until_expiry,
        }
    }
}

pub async fn get_federation_gateways(
    Path(federation_id): Path<FederationId>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<FederationGateways>> {
    let gateways = get_current_gateways(&state, federation_id).await?;

    Ok(Json(FederationGateways {
        federation_id,
        total_count: gateways.len(),
        gateways,
    }))
}

async fn get_current_gateways(
    state: &AppState,
    federation_id: FederationId,
) -> anyhow::Result<Vec<GatewayInfo>> {
    let conn = state.federation_observer.connection().await?;

    let rows: Vec<GatewayRow> = query(
        &conn,
        "SELECT 
            gateway_id,
            node_pub_key,
            api_endpoint,
            base_fee_msat,
            proportional_fee_millionths,
            supports_private_payments,
            registered_at,
            expires_at,
            seconds_until_expiry
         FROM ln_current_gateways
         WHERE federation_id = $1
         ORDER BY base_fee_msat ASC, proportional_fee_millionths ASC",
        &[&federation_id.consensus_encode_to_vec()],
    )
    .await?;

    Ok(rows.into_iter().map(GatewayInfo::from).collect())
}

pub async fn get_all_gateways(
    State(state): State<AppState>,
) -> crate::error::Result<Json<Vec<GatewayInfo>>> {
    let conn = state.federation_observer.connection().await?;

    let rows: Vec<GatewayRow> = query(
        &conn,
        "SELECT 
            gateway_id,
            node_pub_key,
            api_endpoint,
            base_fee_msat,
            proportional_fee_millionths,
            supports_private_payments,
            registered_at,
            expires_at,
            seconds_until_expiry
         FROM ln_current_gateways
         ORDER BY expires_at DESC
         LIMIT 1000",
        &[],
    )
    .await?;

    Ok(Json(rows.into_iter().map(GatewayInfo::from).collect()))
}
