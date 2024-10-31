use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_api_client::api::{DynGlobalApi, FederationApiExt, StatusResponse};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::encoding::Encodable;
use fedimint_core::endpoint_constants::STATUS_ENDPOINT;
use fedimint_core::module::ApiRequestErased;
use fedimint_core::PeerId;
use fedimint_wallet_common::endpoint_constants::BLOCK_COUNT_LOCAL_ENDPOINT;
use fmo_api_types::{GuardianHealth, GuardianHealthLatest};
use futures::future::join_all;
use postgres_from_row::FromRow;

use crate::federation::observer::FederationObserver;
use crate::util::query;

impl FederationObserver {
    pub async fn monitor_health(
        &self,
        federation_id: FederationId,
        config: ClientConfig,
    ) -> anyhow::Result<()> {
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
        const REQUEST_INTERVAL: Duration = Duration::from_secs(60);

        let mut interval = tokio::time::interval(REQUEST_INTERVAL);
        let api = DynGlobalApi::from_endpoints(
            config
                .global
                .api_endpoints
                .iter()
                .map(|(&peer_id, peer_url)| (peer_id, peer_url.url.clone())),
            &None,
        );
        let wallet_module = config
            .modules
            .iter()
            .find_map(|(&module_instance_id, module)| {
                (module.kind.as_str() == "wallet").then_some(module_instance_id)
            })
            .context("Wallet module not found")?;

        loop {
            interval.tick().await;

            let peer_status_responses =
                join_all(config.global.api_endpoints.keys().map(|&peer_id| {
                    let api = api.clone();
                    async move {
                        // We don't time the first request, there might be a reconnect happening in
                        // the background
                        let status = api
                            .request_single_peer(
                                Some(REQUEST_TIMEOUT),
                                STATUS_ENDPOINT.to_owned(),
                                ApiRequestErased::default(),
                                peer_id,
                            )
                            .await
                            .ok()
                            .and_then(|json| serde_json::from_value::<StatusResponse>(json).ok());

                        // Second request is used to determine ping
                        // TODO: how much time does bitcoind take to answer if at all (caching?)?
                        let start_time = Instant::now();
                        let block_height = api
                            .with_module(wallet_module)
                            .request_single_peer(
                                Some(REQUEST_TIMEOUT),
                                BLOCK_COUNT_LOCAL_ENDPOINT.to_owned(),
                                ApiRequestErased::default(),
                                peer_id,
                            )
                            .await
                            .ok()
                            .and_then(|json| {
                                serde_json::from_value::<Option<u32>>(json).ok().flatten()
                            })
                            .map(|block_count| {
                                // Fedimint uses 1-based block heights, while bitcoind uses 0-based
                                // heights
                                block_count - 1
                            });
                        let api_latency = start_time.elapsed();

                        (peer_id, status, block_height, api_latency)
                    }
                }))
                .await;

            let mut conn = self.connection().await?;
            let dbtx = conn.transaction().await?;
            let timestamp = chrono::Utc::now().naive_utc();
            for (peer_id, status, block_height, api_latency) in peer_status_responses {
                dbtx.execute(
                    "INSERT INTO guardian_health VALUES ($1, $2, $3, $4, $5, $6)",
                    &[
                        &federation_id.consensus_encode_to_vec(),
                        &timestamp,
                        &(peer_id.to_usize() as i32),
                        &status.map(|s| serde_json::to_value(s).expect("Can be serialized")),
                        &block_height.map(|bh| bh as i32),
                        &(api_latency.as_millis() as i32),
                    ],
                )
                .await?;
            }
            dbtx.commit().await?;
        }
    }

    pub async fn get_guardian_health(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<BTreeMap<PeerId, GuardianHealth>> {
        let _federation = self
            .get_federation(federation_id)
            .await
            .context("Unknown federation")?;

        let health_rows = query::<GuardianHealthRow>(
            &self.connection().await?,
            "WITH RankedRows AS (
                    SELECT
                        *,
                        ROW_NUMBER() OVER  (PARTITION BY guardian_id ORDER BY time DESC) AS rn
                    FROM
                        guardian_health
                    WHERE
                        federation_id = $1
                ),
                     Last30d AS (
                         SELECT
                             guardian_id,
                             (count(status)::decimal / count(*)::decimal * 100)::real as uptime,
                             avg(latency_ms)::real as latency_ms
                         FROM
                             RankedRows
                         WHERE
                             time > NOW() - INTERVAL '30 days' and
                             federation_id = $1
                         group by
                             guardian_id
                     )
                SELECT
                    RankedRows.guardian_id,
                    RankedRows.block_height,
                    (RankedRows.status -> 'federation'  ->> 'session_count')::integer AS session_count,
                    Last30d.uptime,
                    Last30d.latency_ms
                FROM
                    RankedRows join Last30d on RankedRows.guardian_id = Last30d.guardian_id
                WHERE
                    rn = 1;
                ",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?;

        let our_block_height = self.get_block_height().await?;
        let max_session = health_rows
            .iter()
            .filter_map(|row| row.session_count)
            .max()
            .unwrap_or_default() as u32;

        Ok(health_rows
            .into_iter()
            .map(|row| {
                let latest = if row.session_count.is_some() && row.block_height.is_some() {
                    let block_height = row.block_height.expect("checked above") as u32;
                    let session_count = row.session_count.expect("checked above") as u32;
                    Some(GuardianHealthLatest {
                        block_height,
                        block_outdated: our_block_height.saturating_sub(block_height) > 6,
                        session_count,
                        session_outdated: max_session.saturating_sub(session_count) > 1,
                    })
                } else {
                    None
                };

                let health = GuardianHealth {
                    avg_uptime: row.uptime,
                    avg_latency: row.latency_ms,
                    latest,
                };

                (PeerId::new(row.guardian_id as u16), health)
            })
            .collect())
    }
}

#[derive(FromRow)]
struct GuardianHealthRow {
    guardian_id: i32,
    block_height: Option<i32>,
    session_count: Option<i32>,
    uptime: f32,
    latency_ms: f32,
}

pub(super) async fn get_federation_health(
    Path(federation_id): Path<FederationId>,
    State(state): State<crate::AppState>,
) -> crate::error::Result<Json<BTreeMap<PeerId, GuardianHealth>>> {
    let guardian_health = state
        .federation_observer
        .get_guardian_health(federation_id)
        .await?;

    Ok(Json(guardian_health))
}
