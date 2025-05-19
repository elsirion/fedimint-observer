use deadpool_postgres::GenericClient;
use fedimint_core::config::{ClientConfig, ClientModuleConfig, JsonClientConfig, JsonWithKind};
use fedimint_core::core::{ModuleInstanceId, ModuleKind};
use fedimint_core::encoding::DynRawFallback;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::module::CommonModuleInit;
use fedimint_ln_common::LightningCommonInit;
use fedimint_mint_common::MintCommonInit;
use fedimint_wallet_common::WalletCommonInit;
use hex::ToHex;
use postgres_from_row::FromRow;
use serde_json::json;
#[cfg(feature = "stability_pool_v1")]
use stability_pool_common::StabilityPoolCommonGen;

pub fn config_to_json(cfg: ClientConfig) -> anyhow::Result<JsonClientConfig> {
    let decoders = get_decoders(
        cfg.modules
            .iter()
            .map(|(module_instance_id, module_config)| {
                (*module_instance_id, module_config.kind.clone())
            }),
    );
    let config = cfg.redecode_raw(&decoders)?;

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
                                DynRawFallback::Raw { raw, .. } => {
                                    let raw: String = ToHex::encode_hex(&raw);
                                    json!({"raw": raw})
                                }
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
                #[cfg(feature = "stability_pool_v1")]
                "stability_pool" => StabilityPoolCommonGen::decoder(),
                _ => {
                    return None;
                }
            };

            Some((module_instance_id, module_kind, decoder))
        },
    ))
    .with_fallback()
}

pub async fn execute(
    conn: &impl GenericClient,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> anyhow::Result<u64> {
    let num_rows = conn.execute(sql, params).await?;
    Ok(num_rows)
}

pub async fn query_one<T>(
    conn: &impl GenericClient,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> anyhow::Result<T>
where
    T: FromRow,
{
    let result = conn.query_one(sql, params).await?;
    Ok(T::try_from_row(&result)?)
}

pub async fn query_value<T>(
    conn: &impl GenericClient,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> anyhow::Result<T>
where
    for<'a> T: tokio_postgres::types::FromSql<'a>,
{
    let result = conn.query_one(sql, params).await?;
    Ok(result.try_get(0)?)
}

pub async fn query_opt<T>(
    conn: &impl GenericClient,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> anyhow::Result<Option<T>>
where
    T: FromRow,
{
    let result = conn.query_opt(sql, params).await?;
    Ok(result.map(|row| T::try_from_row(&row)).transpose()?)
}

pub async fn query<T>(
    conn: &impl GenericClient,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> anyhow::Result<Vec<T>>
where
    T: FromRow,
{
    let result = conn.query(sql, params).await?;
    Ok(result
        .iter()
        .map(T::try_from_row)
        .collect::<Result<_, _>>()?)
}
