use std::borrow::Cow;
use std::collections::BTreeMap;

use anyhow::{ensure, Context};
use fedimint_core::config::JsonClientConfig;
use fedimint_core::core::ModuleKind;
use fedimint_core::invite_code::InviteCode;
use leptos::either::Either;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_router::hooks::use_query;
use leptos_router::params::{Params, ParamsError, ParamsMap};
use nostr_sdk::{EventBuilder, Kind, SingleLetterTag, Tag, TagKind};
use reqwest::StatusCode;

use crate::components::alert::{Alert, AlertLevel};
use crate::components::badge::{Badge, BadgeLevel};
use crate::components::button::{Button, SUCCESS_BUTTON};
use crate::BASE_URL;

#[derive(Debug, Clone)]
struct FederationInfo {
    federation_name: String,
    federation_config: JsonClientConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckQuery {
    check: Option<String>,
}

impl Params for CheckQuery {
    fn from_map(map: &ParamsMap) -> Result<Self, ParamsError> {
        Ok(CheckQuery {
            check: map.get("check").map(|s| s.to_string()),
        })
    }
}

#[component]
pub fn CheckFederation() -> impl IntoView {
    let invite_input_ref = NodeRef::<Input>::new();
    let query = use_query::<CheckQuery>();

    let check_federation_action =
        Action::<(), std::result::Result<FederationInfo, String>>::new_local(
            move |&()| async move {
                let check_federation_inner = move || async move {
                    let invite_code = invite_input_ref
                        .get_untracked()
                        .expect("invite_input_ref should be loaded by now")
                        .value();

                    let federation_config = {
                        let url = format!("{}/config/{invite_code}", BASE_URL);
                        let response = reqwest::get(&url).await?;
                        let config: JsonClientConfig = response.json().await?;
                        config
                    };

                    let federation_name = {
                        let url = format!("{}/config/{invite_code}/meta", BASE_URL);
                        let response = reqwest::get(&url).await?;
                        let meta: BTreeMap<String, serde_json::Value> = response.json().await?;
                        meta.get("federation_name")
                            .context("No name found")?
                            .as_str()
                            .context("Name isn't a string")?
                            .to_owned()
                    };

                    Result::<_, anyhow::Error>::Ok(FederationInfo {
                        federation_name,
                        federation_config,
                    })
                };

                check_federation_inner().await.map_err(|e| e.to_string())
            },
        );

    fn or_loading<I: IntoView>(maybe_value: Option<I>) -> impl IntoView {
        if let Some(value) = maybe_value {
            Either::Left(view! {
                <span>
                    {value}
                </span>
            })
        } else {
            Either::Right(view! {
                <div class="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
            })
        }
    }

    let federation_name = move || {
        or_loading(
            check_federation_action
                .value()
                .get()
                .and_then(|info| Some(info.ok()?.federation_name.clone())),
        )
    };
    let federation_guardians = move || {
        or_loading(
            check_federation_action
                .value()
                .get()
                .and_then(|info| Some(info.ok()?.federation_config.global.api_endpoints.len())),
        )
    };
    let federation_modules = move || {
        or_loading(check_federation_action.value().get().and_then(|info| {
            let info = info.ok()?;
            Some(
                get_modules(&info.federation_config)
                    .into_iter()
                    .map(|kind| {
                        view! {
                            <Badge
                                level=BadgeLevel::Info
                            >
                                {kind}
                            </Badge>
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        }))
    };
    let federation_network = move || {
        or_loading(check_federation_action.value().get().and_then(|info| {
            let info = info.ok()?;
            Some(get_network(&info.federation_config))
        }))
    };

    let announce_federation_action =
        Action::<(), std::result::Result<(), String>>::new_local(move |&()| async move {
            let federation_info = check_federation_action
                .value()
                .get_untracked()
                .expect("Button should only be clickable if federation info was fetched")
                .expect(
                    "Button should only be clickable if federation info fetching was successful",
                );

            sign_and_publish_federation(&federation_info.federation_config)
                .await
                .map_err(|e| e.to_string())?;

            Result::<_, String>::Ok(())
        });
    let announce_button_disabled = Signal::derive(move || {
        check_federation_action.pending().get()
            || !check_federation_action
                .value()
                .get()
                .map(|info| info.is_ok())
                .unwrap_or(false)
            || announce_federation_action.pending().get()
            || announce_federation_action
                .value()
                .get()
                .map(|res| res.is_ok())
                .unwrap_or(false)
    });

    // Handle deep-linking: auto-fill input and trigger check if 'check' parameter
    // is present
    Effect::new(move |_| {
        if let Ok(query_params) = query.get() {
            if let Some(ref check_value) = query_params.check {
                if let Some(input_element) = invite_input_ref.get() {
                    input_element.set_value(check_value);
                    check_federation_action.dispatch(());
                }
            }
        }
    });

    view! {
        <div class="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
            <h1 class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
                Inspect Federation
                <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                    "Fetch federation info by invite code"
                </p>
            </h1>

            <div class="p-5 pt-0 dark:text-white dark:bg-gray-800">
                <form
                    class="flex gap-2 items-center"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        check_federation_action.dispatch(());
                    }
                >
                    <div class="relative flex-1">
                        <input
                            node_ref=invite_input_ref
                            placeholder=" "
                            type="text"
                            class="block px-2.5 h-11 w-full text-sm text-gray-900 bg-transparent rounded-lg border-gray-300 appearance-none dark:text-white dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer border"
                        />
                        <label
                            for="floating_outlined"
                            class="absolute text-sm text-gray-500 dark:text-gray-400 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-white dark:bg-gray-800 px-2 peer-focus:px-2 peer-focus:text-blue-600 peer-focus:dark:text-blue-500 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1"
                        >
                            Invite Code
                        </label>
                    </div>
                    <Button
                        on_click=move || {
                            check_federation_action.dispatch(());
                        }
                        disabled=check_federation_action.pending()
                        class="h-11"
                    >
                        Check Federation
                    </Button>
                    <Button
                        on_click=move || {
                            announce_federation_action.dispatch(());
                        }
                        disabled=announce_button_disabled
                        color_scheme=SUCCESS_BUTTON
                        class="h-11"
                    >
                        Announce Federation
                    </Button>
                </form>
                {
                    let error_alert = move || announce_federation_action.value().get()
                        .and_then(|res| res.err())
                        .map(|e| view! {
                            <Alert
                                message=e
                                level=AlertLevel::Error
                                class="mt-4"
                            />
                        });
                    let success_alert = move || announce_federation_action.value().get()
                        .and_then(|res| res.ok())
                        .map(|_| view! {
                            <Alert
                                message="Federation announced successfully! Reload the page to see it listed"
                                level=AlertLevel::Success
                                class="mt-4"
                            />
                        });
                    view! {
                        {error_alert}
                        {success_alert}
                    }
                }

                {
                    let table_view = move || {
                        (check_federation_action.pending().get() || check_federation_action.value().get().map(|info| info.is_ok()).unwrap_or(false))
                            .then(|| view! {
                                <div class="flow-root mt-4">
                                    <div class="relative overflow-x-auto">
                                        <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                                            <tbody>
                                                <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                                    <th
                                                        scope="row"
                                                        class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                                    >
                                                        Name
                                                    </th>
                                                    <td class="px-6 py-4">{federation_name}</td>
                                                </tr>
                                                <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                                    <th
                                                        scope="row"
                                                        class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                                    >
                                                        Guardians
                                                    </th>
                                                    <td class="px-6 py-4 whitespace-normal">{federation_guardians}</td>
                                                </tr>
                                                <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                                    <th
                                                        scope="row"
                                                        class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                                    >
                                                        Modules
                                                    </th>
                                                    <td class="px-6 py-4 whitespace-normal">{federation_modules}</td>
                                                </tr>
                                                <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                                    <th
                                                        scope="row"
                                                        class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                                    >
                                                        Network
                                                    </th>
                                                    <td class="px-6 py-4 whitespace-normal">{federation_network}</td>
                                                </tr>
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            })
                    };
                    let error_alert = move || {
                        check_federation_action.value().get()
                            .and_then(|res| res.err())
                            .map(|e| view! {
                                <Alert
                                    message=e
                                    level=AlertLevel::Error
                                    class="mt-4"
                                />
                            })
                    };
                    view! {
                        {table_view}
                        {error_alert}
                    }
                }
            </div>
        </div>
    }
}

fn get_network(config: &JsonClientConfig) -> String {
    config
        .modules
        .iter()
        .find_map(|(_, module)| {
            module.is_kind(&ModuleKind::from("wallet")).then(|| {
                module
                    .value()
                    .get("network")
                    .expect("Network is of type string")
                    .as_str()
                    .expect("Network is of type string")
                    .to_owned()
            })
        })
        .expect("Wallet module is expected to be present")
}

fn get_modules(config: &JsonClientConfig) -> Vec<String> {
    config
        .modules
        .values()
        .map(|module| module.kind().as_str().to_owned())
        .collect()
}

async fn sign_and_publish_federation(config: &JsonClientConfig) -> anyhow::Result<()> {
    let signer = nostr_sdk::nostr::nips::nip07::Nip07Signer::new()?;

    let federation_id = config.global.calculate_federation_id().to_string();
    let invite_code = InviteCode::new_with_essential_num_guardians(
        &config
            .global
            .api_endpoints
            .iter()
            .map(|(&peer_id, peer_data)| (peer_id, peer_data.url.clone()))
            .collect(),
        config.global.calculate_federation_id(),
    )
    .to_string();
    let network = get_network(config);
    let modules = get_modules(config);

    let tags = vec![
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('d').unwrap()),
            [federation_id],
        ),
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('u').unwrap()),
            [invite_code],
        ),
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('n').unwrap()),
            [network],
        ),
        Tag::custom(
            TagKind::Custom(Cow::Borrowed("modules")),
            [modules.join(",")],
        ),
    ];
    let unsigned_event = EventBuilder::new(
        Kind::Custom(38173),
        // TODO: make this take into account meta announcements or leave it out
        serde_json::to_string(&config.global.meta).expect("Meta should be serializable"),
        tags,
    )
    .to_unsigned_event(signer.get_public_key().await?);

    let event = signer.sign_event(unsigned_event).await?;

    let client = reqwest::Client::new();
    let response = client
        .put(format!("{}/nostr/federations", BASE_URL))
        .json(&event)
        .send()
        .await?;

    let status = response.status();
    ensure!(
        status == StatusCode::OK,
        "Unexpected status code {}",
        status
    );

    Ok(())
}
