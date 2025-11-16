mod check_federation;
mod nostr_federation_row;

use std::collections::BTreeMap;

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

    view! {
        <Title
            text="Fedimint Observer"
        />

        <CheckFederation />

        <div class="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
            <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                <caption class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
                    "Nostr Federations"
                    <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                        "Other federations announced via Nostr"
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

                        nostr_federations_res.get().unwrap_or_default()
                            .into_iter()
                            .map(|(federation_id, invite_code)| {
                                let is_observed = observed_ids.contains(&federation_id);
                                view! {
                                    <NostrFederationRow
                                        federation_id=federation_id
                                        invite_code=invite_code
                                        is_observed=is_observed
                                    />
                                }
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
