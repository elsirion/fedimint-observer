use std::collections::BTreeSet;

use axum::extract::{Path, State};
use axum::Json;
use fedimint_core::api::InviteCode;

use crate::config::meta::MetaOverrideCache;
use crate::config::FederationConfigCache;

pub async fn fetch_federation_module_kinds(
    Path(invite): Path<InviteCode>,
    State((config_cache, _)): State<(FederationConfigCache, MetaOverrideCache)>,
) -> crate::error::Result<Json<BTreeSet<String>>> {
    let config = config_cache.fetch_config_cached(&invite).await?;
    let module_kinds = config
        .modules
        .into_values()
        .map(|module_config| module_config.kind().as_str().to_owned())
        .collect::<BTreeSet<_>>();

    Ok(module_kinds.into())
}
