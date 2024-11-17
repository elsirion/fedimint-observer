use std::collections::BTreeMap;

use anyhow::Context;
use fedimint_core::config::JsonClientConfig;
use leptos::html::Input;
use leptos::{
    component, create_action, create_node_ref, create_signal, view, IntoView, SignalGet, SignalSet,
};

use crate::components::alert::{Alert, AlertLevel};
use crate::components::badge::{Badge, BadgeLevel};
use crate::BASE_URL;

#[derive(Debug, Clone)]
struct FederationInfo {
    federation_name: String,
    federation_config: JsonClientConfig,
}

#[component]
pub fn CheckFederation() -> impl IntoView {
    let (check_button_disabled, set_check_button_disabled) = create_signal(false);
    let invite_input_ref = create_node_ref::<Input>();
    let check_federation_action = create_action(move |&()| async move {
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

        let info = check_federation_inner().await.map_err(|e| e.to_string());
        set_check_button_disabled.set(false);
        info
    });

    fn or_loading<I: IntoView>(maybe_value: Option<I>) -> impl IntoView {
        if let Some(value) = maybe_value {
            view! {
                <span>
                    {value}
                </span>
            }
            .into_view()
        } else {
            view! {
                <div class="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
            }
            .into_view()
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
                info.federation_config
                    .modules
                    .values()
                    .map(|v| {
                        let kind = v.kind().as_str().to_owned();
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
            Some(
                info.federation_config
                    .modules
                    .iter()
                    .find_map(|(_, m)| {
                        if m.kind().as_str() != "wallet" {
                            return None;
                        }
                        Some(
                            m.value()["network"]
                                .as_str()
                                .expect("Network is of type string")
                                .to_owned(),
                        )
                    })
                    .expect("Wallet module is expected to be present"),
            )
        }))
    };

    view! {
        <div class="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
            <h1 class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
                Inspect Federation
                <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                    "Fetch federation info by invite code"
                </p>
            </h1>

            <div class="p-5 pt-0">
                <form class="flex gap-2 items-center">
                    <div class="relative flex-1">
                        <input
                            _ref=invite_input_ref
                            placeholder=" "
                            type="text"
                            class="block px-2.5 h-11 w-full text-sm text-gray-900 bg-transparent rounded-lg border-gray-300 appearance-none dark:text-white dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer border"
                        />
                        <label
                            for="floating_outlined"
                            class="absolute text-sm text-gray-500 dark:text-gray-400 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-white dark:bg-gray-900 px-2 peer-focus:px-2 peer-focus:text-blue-600 peer-focus:dark:text-blue-500 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1"
                        >
                            Invite Code
                        </label>
                    </div>
                    <button
                        on:click=move |_| {
                            set_check_button_disabled.set(true);
                            check_federation_action.dispatch(());
                        }
                        enabled=move || !check_button_disabled.get()
                        type="button"
                        class="h-11 whitespace-nowrap text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800"
                    >
                        Check Federation
                    </button>
                </form>

                { move || if check_federation_action.pending().get() || check_federation_action.value().get().map(|info| info.is_ok()).unwrap_or(false) {
                    view! {
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
                    }.into_view()
                } else if let Some(Err(e)) = check_federation_action.value().get() {
                    view! {
                        <Alert
                            message=e
                            level=AlertLevel::Error
                            class="mt-4"
                        />
                    }.into_view()
                } else {
                    view!().into_view()
                }}
            </div>
        </div>
    }
}
