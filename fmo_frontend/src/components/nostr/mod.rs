mod check_federation;
mod nostr_federation_row;

use std::collections::BTreeMap;

use anyhow::Context;
use check_federation::CheckFederation;
use fedimint_core::config::FederationId;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::util::backoff_util::background_backoff;
use fedimint_core::util::retry;
use fmo_api_types::FederationSummary;
use leptos::prelude::*;
use leptos_meta::Title;
use nostr_federation_row::NostrFederationRow;

use crate::BASE_URL;

#[component]
pub fn NostrFederations() -> impl IntoView {
    let nostr_federations_res = LocalResource::new(fetch_nostr_federations);
    let observed_federations_res = LocalResource::new(fetch_observed_federations);

    // Signal to store federation names as they are fetched
    let (federation_names, set_federation_names) = signal(BTreeMap::<FederationId, String>::new());

    let (collapse_offline, set_collapse_offline) = signal(true);

    // Spawn tasks to fetch each federation name independently
    Effect::new(move || {
        if let Some(federations) = nostr_federations_res.get() {
            for (federation_id, invite_code) in federations {
                // Spawn independent task for each federation
                leptos::task::spawn_local(async move {
                    if let Some(name) = fetch_federation_name(invite_code).await {
                        set_federation_names.update(|names| {
                            names.insert(federation_id, name);
                        });
                    }
                });
            }
        }
    });

    view! {
        <Title
            text="Fedimint Observer"
        />

        <CheckFederation />

        <div class="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
            <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                <caption class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
                    "Online Nostr Federations"
                    <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                        "Federations announced via Nostr that are currently online"
                    </p>
                </caption>
                <thead class="text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400">
                    <tr>
                        <th scope="col" class="px-6 py-3">
                            "Name"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Invite Code"
                        </th>
                    </tr>
                </thead>
                <tbody>
                    { move || {
                        let observed_ids = observed_federations_res.get()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|summary| summary.id)
                            .collect::<std::collections::HashSet<_>>();

                        let names = federation_names.get();

                        nostr_federations_res.get().unwrap_or_default()
                            .into_iter()
                            .filter_map(|(federation_id, invite_code)| {
                                let name = names.get(&federation_id).cloned();
                                // Only show if we have a name (online)
                                name.as_ref()?;
                                let is_observed = observed_ids.contains(&federation_id);
                                Some(view! {
                                    <NostrFederationRow
                                        federation_id=federation_id
                                        invite_code=invite_code
                                        is_observed=is_observed
                                        federation_name=name
                                    />
                                })
                            })
                            .collect::<Vec<_>>()
                    }}
                </tbody>
            </table>
        </div>

        <div class="relative overflow-x-auto shadow-md sm:rounded-lg mt-6">
            <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                <caption
                    class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800"
                    on:click=move |_| set_collapse_offline.set(!collapse_offline.get_untracked())
                >
                    <svg
                        class=move || if collapse_offline.get() {
                            "w-3 h-3 shrink-0 inline mr-2 rotate-180"
                        } else {
                            "w-3 h-3 shrink-0 inline mr-2"
                        }
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 10 6"
                    >
                        <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5 5 1 1 5"/>
                    </svg>
                    <span>"Offline Nostr Federations"</span>
                    <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                        "Federations announced via Nostr that are currently offline or unreachable"
                    </p>
                </caption>
                <thead
                    class=move || if collapse_offline.get() {
                        "hidden"
                    } else {
                        "text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400"
                    }
                >
                    <tr>
                        <th scope="col" class="px-6 py-3">
                            "Name"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Invite Code"
                        </th>
                    </tr>
                </thead>
                <tbody
                    class=move || if collapse_offline.get() {
                        "hidden"
                    } else {
                        ""
                    }
                >
                    { move || {
                        let observed_ids = observed_federations_res.get()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|summary| summary.id)
                            .collect::<std::collections::HashSet<_>>();

                        let names = federation_names.get();

                        nostr_federations_res.get().unwrap_or_default()
                            .into_iter()
                            .filter_map(|(federation_id, invite_code)| {
                                let name = names.get(&federation_id).cloned();
                                // Only show if we don't have a name (offline)
                                if name.is_some() {
                                    return None;
                                }
                                let is_observed = observed_ids.contains(&federation_id);
                                Some(view! {
                                    <NostrFederationRow
                                        federation_id=federation_id
                                        invite_code=invite_code
                                        is_observed=is_observed
                                        federation_name=name
                                    />
                                })
                            })
                            .collect::<Vec<_>>()
                    }}
                </tbody>
            </table>
        </div>
    }
}
async fn fetch_nostr_federations() -> BTreeMap<FederationId, InviteCode> {
    let url = format!("{}/nostr/federations", BASE_URL);

    let fetch_nostr_federations_impl = || {
        let url_inner = url.clone();
        async move {
            let response = reqwest::get(&url_inner).await?;
            let federations: BTreeMap<FederationId, InviteCode> = response.json().await?;
            Ok(federations)
        }
    };

    retry(
        "Fetching Nostr federations",
        background_backoff(),
        fetch_nostr_federations_impl,
    )
    .await
    .expect("Will never return Err")
}

async fn fetch_observed_federations() -> Vec<FederationSummary> {
    let url = format!("{}/federations", BASE_URL);

    let fetch_observed_federations_impl = || {
        let url_inner = url.clone();
        async move {
            let response = reqwest::get(&url_inner).await?;
            let federations: Vec<FederationSummary> = response.json().await?;
            Ok(federations)
        }
    };

    retry(
        "Fetching observed federations",
        background_backoff(),
        fetch_observed_federations_impl,
    )
    .await
    .expect("Will never return Err")
}

async fn fetch_federation_name(invite_code: InviteCode) -> Option<String> {
    let url = format!("{}/config/{invite_code}/meta", BASE_URL);

    let response = reqwest::get(&url).await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let federation: BTreeMap<String, serde_json::Value> = response.json().await.ok()?;
    federation
        .get("federation_name")
        .context("No name found")
        .ok()?
        .as_str()
        .context("Name isn't a string")
        .ok()
        .map(|s| s.to_owned())
}
