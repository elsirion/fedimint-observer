use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use fedimint_core::api::InviteCode;
use fedimint_core::config::{
    ClientConfig, ClientModuleConfig, FederationId, JsonClientConfig, JsonWithKind,
};
use fedimint_core::core::{ModuleInstanceId, ModuleKind};
use fedimint_core::encoding::DynRawFallback;
use fedimint_core::module::__reexports::serde_json::json;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::module::CommonModuleInit;
use fedimint_ln_common::bitcoin::hashes::hex::ToHex;
use fedimint_ln_common::LightningCommonInit;
use fedimint_mint_common::MintCommonInit;
use fedimint_wallet_common::WalletCommonInit;
use reqwest::Method;
use tower_http::cors::{Any, CorsLayer};
use tracing::warn;

use crate::config::id::fetch_federation_id;
use crate::config::meta::fetch_federation_meta;
use crate::config::modules::fetch_federation_module_kinds;
use crate::error::Result;
use crate::AppState;

/// Helper API that exposes the federation id
pub mod id;
/// Helper API that unifies config meta and override meta, applying lenient
/// parsing
pub mod meta;

/// Helper API that exposes the federation modules
pub mod modules;
pub fn get_config_routes() -> Router<AppState> {
    let router = Router::new()
        .route("/:invite", get(fetch_federation_config))
        .route("/:invite/meta", get(fetch_federation_meta))
        .route("/:invite/id", get(fetch_federation_id))
        .route("/:invite/module_kinds", get(fetch_federation_module_kinds));

    let cors_enabled = dotenv::var("ALLOW_CONFIG_CORS").map_or(false, |v| v == "true");

    if cors_enabled {
        router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET])
                .allow_credentials(true),
        )
    } else {
        router
    }
}

pub async fn fetch_federation_config(
    Path(invite): Path<InviteCode>,
    State(state): State<AppState>,
) -> Result<Json<JsonClientConfig>> {
    Ok(state
        .federation_config_cache
        .fetch_config_cached(&invite)
        .await?
        .into())
}

#[derive(Default, Debug, Clone)]
pub struct FederationConfigCache {
    federations: Arc<tokio::sync::RwLock<HashMap<FederationId, JsonClientConfig>>>,
}

impl FederationConfigCache {
    pub async fn fetch_config_cached(
        &self,
        invite: &InviteCode,
    ) -> anyhow::Result<JsonClientConfig> {
        let federation_id = invite.federation_id();

        if let Some(config) = self.federations.read().await.get(&federation_id).cloned() {
            return Ok(config);
        }

        let config = fetch_config_inner(invite).await?;
        let mut cache = self.federations.write().await;
        if let Some(replaced) = cache.insert(federation_id, config.clone()) {
            if replaced != config {
                // TODO: use tracing
                warn!("Config for federation {federation_id} changed");
            }
        }

        Ok(config)
    }
}

async fn fetch_config_inner(invite: &InviteCode) -> anyhow::Result<JsonClientConfig> {
    let raw_config = ClientConfig::download_from_invite_code(invite).await?;
    let decoders = get_decoders(raw_config.modules.iter().map(
        |(module_instance_id, module_config)| (*module_instance_id, module_config.kind.clone()),
    ));
    let config = raw_config.redecode_raw(&decoders)?;

    Ok(JsonClientConfig {
        global: config.global,
        modules: config
            .modules
            .into_iter()
            .map(
                |(
                    instance_id,
                    ClientModuleConfig {
                        kind,
                        config: module_config,
                        ..
                    },
                )| {
                    (
                        instance_id,
                        JsonWithKind::new(
                            kind.clone(),
                            match module_config {
                                DynRawFallback::Raw { raw, .. } => json!({"raw": raw.to_hex()}),
                                DynRawFallback::Decoded(decoded) => decoded.to_json().into(),
                            },
                        ),
                    )
                },
            )
            .collect(),
    })
}

pub fn get_decoders(
    modules: impl IntoIterator<Item = (ModuleInstanceId, ModuleKind)>,
) -> ModuleDecoderRegistry {
    ModuleDecoderRegistry::new(modules.into_iter().filter_map(
        |(module_instance_id, module_kind)| {
            let decoder = match module_kind.as_str() {
                "ln" => LightningCommonInit::decoder(),
                "wallet" => WalletCommonInit::decoder(),
                "mint" => MintCommonInit::decoder(),
                _ => {
                    return None;
                }
            };

            Some((module_instance_id, module_kind, decoder))
        },
    ))
    .with_fallback()
}
