use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_api_client::api::{DynGlobalApi, FederationApiExt};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::ModuleInstanceId;
use fedimint_core::encoding::Encodable;
use fedimint_core::module::ApiRequestErased;
use fedimint_ln_common::federation_endpoint_constants::LIST_GATEWAYS_ENDPOINT;
use fedimint_ln_common::LightningGatewayAnnouncement;
use fmo_api_types::GatewayInfo;
use tracing::{info, warn};

use crate::federation::observer::FederationObserver;
use crate::util::query;

impl FederationObserver {
    /// Background task: poll the LN module on this federation for currently
    /// registered gateways and persist them. Runs in a loop until cancelled.
    pub async fn monitor_gateways(
        &self,
        federation_id: FederationId,
        config: ClientConfig,
    ) -> anyhow::Result<()> {
        const POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

        let api = DynGlobalApi::from_endpoints(
            config
                .global
                .api_endpoints
                .iter()
                .map(|(&peer_id, peer_url)| (peer_id, peer_url.url.clone())),
            &None,
        )
        .await?;

        let ln_instance_id = config
            .modules
            .iter()
            .find_map(|(&instance_id, module)| {
                (module.kind.as_str() == "ln").then_some(instance_id)
            })
            .context("No LN module found in federation config")?;

        let peer_ids: Vec<fedimint_core::PeerId> =
            config.global.api_endpoints.keys().copied().collect();

        let mut interval = tokio::time::interval(POLL_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) =
                Self::fetch_and_store_gateways(self, federation_id, &api, ln_instance_id, &peer_ids)
                    .await
            {
                warn!(
                    "Failed to fetch gateways for federation {}: {:?}",
                    federation_id, e
                );
            }
        }
    }

    async fn fetch_and_store_gateways(
        &self,
        federation_id: FederationId,
        api: &DynGlobalApi,
        ln_instance_id: ModuleInstanceId,
        peer_ids: &[fedimint_core::PeerId],
    ) -> anyhow::Result<()> {
        // Query all peers and merge by gateway_id — each guardian has their own
        // registry so we take the union across all peers.
        let mut merged: HashMap<String, LightningGatewayAnnouncement> = HashMap::new();

        for &peer_id in peer_ids {
            let result: anyhow::Result<Vec<LightningGatewayAnnouncement>> = api
                .with_module(ln_instance_id)
                .request_single_peer(
                    LIST_GATEWAYS_ENDPOINT.to_owned(),
                    ApiRequestErased::default(),
                    peer_id,
                )
                .await
                .map_err(anyhow::Error::from)
                .and_then(|v| serde_json::from_value(v).map_err(anyhow::Error::from));

            match result {
                Ok(gateways) => {
                    for gw in gateways {
                        merged.entry(gw.info.gateway_id.to_string()).or_insert(gw);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch gateways from peer {} for {}: {:?}",
                        peer_id, federation_id, e
                    );
                }
            }
        }

        let conn = self.connection().await?;
        let now = chrono::Utc::now();
        let federation_id_bytes = federation_id.consensus_encode_to_vec();

        for (gateway_id, gw) in &merged {
            let node_pub_key = gw.info.node_pub_key.to_string();
            let api_endpoint = gw.info.api.to_string();
            let lightning_alias = &gw.info.lightning_alias;
            let vetted = gw.vetted;
            let raw = serde_json::to_value(gw)?;

            conn.execute(
                "INSERT INTO gateways
                     (federation_id, gateway_id, node_pub_key, api_endpoint,
                      lightning_alias, vetted, raw, first_seen, last_seen)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
                 ON CONFLICT (federation_id, gateway_id) DO UPDATE
                     SET node_pub_key    = EXCLUDED.node_pub_key,
                         api_endpoint    = EXCLUDED.api_endpoint,
                         lightning_alias = EXCLUDED.lightning_alias,
                         vetted          = EXCLUDED.vetted,
                         raw             = EXCLUDED.raw,
                         last_seen       = EXCLUDED.last_seen",
                &[
                    &federation_id_bytes,
                    gateway_id,
                    &node_pub_key,
                    &api_endpoint,
                    lightning_alias,
                    &vetted,
                    &raw,
                    &now,
                ],
            )
            .await?;
        }

        info!(
            "Stored {} gateway(s) for federation {}",
            merged.len(),
            federation_id
        );
        Ok(())
    }

    pub async fn list_federation_gateways(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<Vec<GatewayInfo>> {
        #[derive(postgres_from_row::FromRow)]
        struct GatewayRow {
            gateway_id: String,
            node_pub_key: String,
            lightning_alias: String,
            api_endpoint: String,
            vetted: bool,
            raw: serde_json::Value,
        }

        let rows = query::<GatewayRow>(
            &self.connection().await?,
            "SELECT gateway_id, node_pub_key, lightning_alias, api_endpoint, vetted, raw
             FROM gateways
             WHERE federation_id = $1
             ORDER BY last_seen DESC",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GatewayInfo {
                gateway_id: r.gateway_id,
                node_pub_key: r.node_pub_key,
                lightning_alias: r.lightning_alias,
                api_endpoint: r.api_endpoint,
                vetted: r.vetted,
                raw: Some(r.raw),
            })
            .collect())
    }
}

pub(super) async fn get_federation_gateways(
    Path(federation_id): Path<FederationId>,
    State(state): State<crate::AppState>,
) -> crate::error::Result<Json<Vec<GatewayInfo>>> {
    Ok(state
        .federation_observer
        .list_federation_gateways(federation_id)
        .await?
        .into())
}
