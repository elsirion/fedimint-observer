use std::time::{Duration, Instant};

use anyhow::Context;
use fedimint_api_client::api::{DynGlobalApi, FederationApiExt, StatusResponse};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::encoding::Encodable;
use fedimint_core::endpoint_constants::STATUS_ENDPOINT;
use fedimint_core::module::ApiRequestErased;
use fedimint_wallet_common::endpoint_constants::BLOCK_COUNT_LOCAL_ENDPOINT;
use futures::future::join_all;

use crate::federation::observer::FederationObserver;

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
}
