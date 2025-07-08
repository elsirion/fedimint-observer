use axum::Json;
use fedimint_core::config::JsonClientConfig;
use fedimint_core_v3::config::META_OVERRIDE_URL_KEY;
use tracing::debug;
use tracing::log::warn;

use crate::config::meta::{parse_meta_lenient, MetaFields};
use crate::AppState;

pub async fn federation_meta(
    cfg: &JsonClientConfig,
    state: &AppState,
) -> crate::error::Result<Json<MetaFields>> {
    let meta_fields_config = parse_meta_lenient(
        cfg.global
            .meta
            .iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned().into())),
    );

    let meta_fields = if let Some(override_url) = meta_fields_config
        .get(META_OVERRIDE_URL_KEY)
        .or_else(|| meta_fields_config.get("meta_external_url")) // Fedi legacy field
        .and_then(|url| url.as_str().map(ToOwned::to_owned))
    {
        debug!("fetching {override_url}");
        let meta_override = match state
            .meta_override_cache
            .fetch_meta_cached(&override_url, cfg.global.calculate_federation_id())
            .await
        {
            Ok(meta) => meta,
            Err(e) => {
                warn!("Failed to fetch meta fields from {override_url}: {e:?}");
                return Ok(meta_fields_config.into());
            }
        };

        meta_fields_config
            .into_iter()
            .chain(meta_override)
            .collect::<MetaFields>()
    } else {
        meta_fields_config
    };

    Ok(meta_fields.into())
}
