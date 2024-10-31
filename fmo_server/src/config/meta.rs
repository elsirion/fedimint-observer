use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::config::FederationId;
use fedimint_core::invite_code::InviteCode;

use crate::meta::federation_meta;
use crate::AppState;

pub type MetaFields = BTreeMap<String, serde_json::Value>;

const REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub async fn fetch_federation_meta(
    Path(invite): Path<InviteCode>,
    State(state): State<AppState>,
) -> crate::error::Result<Json<MetaFields>> {
    let config = state
        .federation_config_cache
        .fetch_config_cached(&invite)
        .await?;

    federation_meta(&config, &state).await
}

#[derive(Default, Debug, Clone)]
pub struct MetaOverrideCache {
    client: reqwest::Client,
    override_files: Arc<tokio::sync::RwLock<HashMap<String, (serde_json::Value, SystemTime)>>>,
}

impl MetaOverrideCache {
    pub async fn fetch_meta_cached(
        &self,
        url: &str,
        federation_id: FederationId,
    ) -> anyhow::Result<MetaFields> {
        let current_meta_cache_entry = self.override_files.read().await.get(url).cloned();
        let meta = match current_meta_cache_entry {
            Some((meta, last_update))
                if SystemTime::now()
                    .duration_since(last_update)
                    .unwrap_or_default()
                    <= REFRESH_INTERVAL =>
            {
                meta
            }
            _ => {
                let meta = self.fetch_meta_inner(url).await?;
                let mut cache = self.override_files.write().await;
                cache.insert(url.to_owned(), (meta.clone(), SystemTime::now()));
                meta
            }
        };

        let federation_meta = parse_meta_lenient(serde_json::from_value::<MetaFields>(
            meta.get(federation_id.to_string())
                .ok_or_else(|| anyhow!("No entry for federation {federation_id} in {url}"))?
                .clone(),
        )?);
        Ok(federation_meta)
    }

    async fn fetch_meta_inner(&self, url: &str) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?)
    }
}

pub fn parse_meta_lenient(
    meta: impl IntoIterator<Item = (String, serde_json::Value)>,
) -> MetaFields {
    meta.into_iter()
        .filter_map(|(key, value)| {
            let value_string = value.as_str()?.to_owned();
            let value = serde_json::from_str(&value_string).unwrap_or_else(|_| value_string.into());
            Some((key, value))
        })
        .collect()
}
