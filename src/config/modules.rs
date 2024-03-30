use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::api::InviteCode;
use serde_json::json;

use crate::config::meta::MetaOverrideCache;
use crate::config::FederationConfigCache;

pub async fn fetch_federation_modules(
    Path(invite): Path<InviteCode>,
    State((config_cache, _)): State<(FederationConfigCache, MetaOverrideCache)>,
) -> crate::error::Result<Json<serde_json::Value>> {
    let config = config_cache.fetch_config_cached(&invite).await?;
    let module_names = config
        .modules
        .into_values()
        .map(|module_config| module_config.kind().as_str().to_owned())
        .collect::<Vec<_>>();

    Ok(json!({ "modules": module_names }).into())
}
