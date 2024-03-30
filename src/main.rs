mod error;

use anyhow::Context;
use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use error::Result;
use fedimint_core::api::InviteCode;
use fedimint_core::config::{ClientConfig, ClientModuleConfig, JsonClientConfig, JsonWithKind};
use fedimint_core::core::{ModuleInstanceId, ModuleKind};
use fedimint_core::encoding::DynRawFallback;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::module::CommonModuleInit;
use fedimint_ln_common::bitcoin::hashes::hex::ToHex;
use fedimint_ln_common::LightningCommonInit;
use fedimint_mint_common::MintCommonInit;
use fedimint_wallet_common::WalletCommonInit;

fn get_decoders(
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

async fn fetch_federation_config(invite: Path<InviteCode>) -> Result<Json<JsonClientConfig>> {
    let invite = invite.0;
    let raw_config = ClientConfig::download_from_invite_code(&invite).await?;
    let decoders = get_decoders(raw_config.modules.iter().map(
        |(module_instance_id, module_config)| (*module_instance_id, module_config.kind.clone()),
    ));
    let config = raw_config.redecode_raw(&decoders)?;

    let json_config = JsonClientConfig {
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
                                DynRawFallback::Raw { raw, .. } => raw.to_hex().into(),
                                DynRawFallback::Decoded(decoded) => decoded.to_json().into(),
                            },
                        ),
                    )
                },
            )
            .collect(),
    };

    Ok(json_config.into())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new().route("/config/:invite", get(fetch_federation_config));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("Binding to port")?;

    axum::serve(listener, app)
        .await
        .context("Starting axum server")?;

    Ok(())
}
