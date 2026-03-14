mod activity;
mod chart;
mod general;
mod guardians;
pub mod nostr_vote;
pub mod stars_selector;
mod utxos;

use std::collections::BTreeMap;
use std::str::FromStr;

use fedimint_core::config::{FederationId, JsonClientConfig};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params;
use leptos_router::params::{Params, ParamsError, ParamsMap};
use utxos::Utxos;

use crate::components::federation::activity::ActivityChart;
use crate::components::federation::general::General;
use crate::components::federation::guardians::{Guardian, Guardians};
use crate::components::federation::nostr_vote::NostrVote;
use crate::components::tabs::{Tab, Tabs};
use crate::BASE_URL;

#[component]
pub fn Federation() -> impl IntoView {
    let id = move || {
        let params = use_params::<FederationParams>();
        params.with(|params| params.as_ref().map(|params| params.id).ok())
    };

    let config_resource = LocalResource::new(move || async move {
        match id() {
            Some(id) => {
                let config = fetch_federation_config(id)
                    .await
                    .map_err(|e| e.to_string())?;
                Result::<_, String>::Ok(config)
            }
            None => Err("No federation id".to_owned()),
        }
    });

    let meta_resource = LocalResource::new(move || async move {
        match id() {
            Some(id) => {
                let meta = fetch_federation_meta(id).await.map_err(|e| e.to_string())?;
                Result::<_, String>::Ok(meta)
            }
            None => Err("No federation id".to_owned()),
        }
    });

    view! {
        <Show
            when=move || { id().is_some() }
            fallback=|| {
                view! { <p>Invalid federation id</p> }
            }
        >
            <Title
                text=move || {
                        match meta_resource.get() {
                            Some(Ok(meta)) => {
                                meta.get("federation_name")
                                    .and_then(|name| name.as_str())
                                    .map(|name| name.to_owned())
                                    .unwrap_or_else(|| id().unwrap().to_string())
                            }
                            _ => "Fedimint Observer".to_owned(),
                        }
                    }
            />
            <div>
                <h2 class="text-4xl my-8 font-extrabold dark:text-white truncate">
                    {move || {
                        match meta_resource.get() {
                            Some(Ok(meta)) => {
                                meta.get("federation_name")
                                    .and_then(|name| name.as_str())
                                    .map(|name| name.to_owned())
                                    .unwrap_or_else(|| id().unwrap().to_string())
                            }
                            Some(Err(e)) => format!("Error: {}", e),
                            None => "Loading ...".to_owned(),
                        }
                    }}
                </h2>
                {move || {
                    match config_resource.get() {
                        Some(Ok(config)) => {
                            view! {
                                <div class="flex flex-wrap items-stretch gap-4 ">
                                    <div class="flex-1 min-w-[400px]">
                                        <Guardians
                                            federation_id=id().unwrap()
                                            guardians=config
                                                .global
                                                .api_endpoints.values().map(|guardian| Guardian {
                                                    name: guardian.name.clone(),
                                                    url: guardian.url.to_string(),
                                                })
                                                .collect()
                                        />
                                    </div>
                                    <div class="flex-1 min-w-[400px]">
                                        <General config=config.clone() />
                                        <div class="h-4" />
                                        <NostrVote config=config.clone() />
                                    </div>
                                </div>
                                <Tabs default="Activity">
                                    <Tab name="Activity">
                                        <ActivityChart id=id().unwrap()/>
                                    </Tab>
                                    <Tab name="UTXOs">
                                        <Utxos federation_id=id().unwrap()/>
                                    </Tab>
                                    <Tab name="Config">
                                        <div class="w-full overflow-x-scroll my-4">
                                            <pre class="dark:text-white">
                                                {serde_json::to_string_pretty(&config)
                                                    .expect("can be encoded")}
                                            </pre>
                                        </div>
                                    </Tab>
                                </Tabs>
                            }
                                .into_any()
                        }
                        Some(Err(e)) => view! { <p>{format!("Error: {}", e)}</p> }.into_any(),
                        None => view! { <p>"Loading..."</p> }.into_any(),
                    }
                }}

            </div>
        </Show>
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FederationParams {
    id: FederationId,
}

impl Params for FederationParams {
    fn from_map(map: &ParamsMap) -> Result<Self, ParamsError> {
        map.get("id")
            .and_then(|id| FederationId::from_str(id.as_str()).ok())
            .map(|id| FederationParams { id })
            .ok_or_else(|| ParamsError::MissingParam("id".into()))
    }
}

async fn fetch_federation_config(id: FederationId) -> Result<JsonClientConfig, anyhow::Error> {
    reqwest::get(format!("{}/federations/{}/config", BASE_URL, id))
        .await?
        .json()
        .await
        .map_err(Into::into)
}

async fn fetch_federation_meta(
    id: FederationId,
) -> Result<BTreeMap<String, serde_json::Value>, anyhow::Error> {
    reqwest::get(format!("{}/federations/{}/meta", BASE_URL, id))
        .await?
        .json()
        .await
        .map_err(Into::into)
}
