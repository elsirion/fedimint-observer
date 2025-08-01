mod federation_row;
pub mod rating;
mod totals;

use fedimint_core::Amount;
use fmo_api_types::{FederationHealth, FederationSummary};
use itertools::Itertools;
use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::federations::federation_row::FederationRow;
use crate::components::federations::totals::Totals;
use crate::BASE_URL;

#[component]
pub fn Federations() -> impl IntoView {
    let federations_res =
        LocalResource::new(|| async { fetch_federations().await.map_err(|e| e.to_string()) });

    fn to_row((summary, avg_txs, avg_volume): (FederationSummary, f64, Amount)) -> impl IntoView {
        view! {
            <FederationRow
                id=summary.id
                name=summary.name.clone().unwrap_or_else(|| "Unnamed".to_owned())
                rating=summary.nostr_votes
                invite=summary.invite.clone()
                total_assets=summary.deposits
                avg_txs=avg_txs
                avg_volume=avg_volume
                health=summary.health
            />
        }
    }

    let active_rows = move || {
        Some(
            federations_res
                .get()?
                .ok()?
                .into_iter()
                .filter(|(summary, _, _)| summary.health != FederationHealth::Offline)
                .map(to_row)
                .collect::<Vec<_>>(),
        )
    };
    let inactive_rows = move || {
        Some(
            federations_res
                .get()?
                .ok()?
                .into_iter()
                .filter(|(summary, _, _)| summary.health == FederationHealth::Offline)
                .map(to_row)
                .collect::<Vec<_>>(),
        )
    };

    let (collapse_offline, set_collapse_offline) = signal(true);

    view! {
        <Title
            text="Fedimint Observer"
        />
        <div class="my-16">
            <Totals/>
        </div>
        <div class="relative overflow-x-auto shadow-md sm:rounded-lg">
            <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                <caption class="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
                    "Observed Federations"
                    <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                        "List of all active federations this instance is collecting statistics on"
                    </p>
                </caption>
                <thead class="text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400">
                    <tr>
                        <th scope="col" class="px-6 py-3">
                            "Name"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            <a
                                href="https://github.com/nostr-protocol/nips/pull/1110"
                                class="underline hover:no-underline"
                            >
                                "Recommendations"
                            </a>
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Invite Code"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Total Assets"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Average Activity (7d)"
                        </th>
                    </tr>
                </thead>
                <tbody>{active_rows}</tbody>
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
                    <span>"Shut Down Federations"</span>
                    <p class="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
                        "List of federations that have ceased operations but were observed in the past"
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
                            <a
                                href="https://github.com/nostr-protocol/nips/pull/1110"
                                class="underline hover:no-underline"
                            >
                                "Recommendations"
                            </a>
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Invite Code"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Total Assets"
                        </th>
                        <th scope="col" class="px-6 py-3">
                            "Average Activity (7d)"
                        </th>
                    </tr>
                </thead>
                <tbody
                    class=move || if collapse_offline.get() {
                        "hidden"
                    } else {
                        ""
                    }
                >{inactive_rows}</tbody>
            </table>
        </div>
    }
}

async fn fetch_federations() -> anyhow::Result<Vec<(FederationSummary, f64, Amount)>> {
    fn rating_index(rating: &fmo_api_types::FederationRating) -> f64 {
        rating.avg.unwrap_or(0.0) * (rating.count as f64 + 1.0).log10()
    }

    let url = format!("{}/federations", BASE_URL);
    let response = reqwest::get(&url).await?;
    let federations: Vec<FederationSummary> = response.json().await?;

    let federations = federations
        .into_iter()
        .map(|federation_summary| {
            let avg_txs = federation_summary
                .last_7d_activity
                .iter()
                .map(|tx| tx.num_transactions)
                .sum::<u64>() as f64
                / federation_summary.last_7d_activity.len() as f64;
            let avg_volume = Amount::from_msats(
                federation_summary
                    .last_7d_activity
                    .iter()
                    .map(|tx| tx.amount_transferred.msats)
                    .sum::<u64>()
                    / federation_summary.last_7d_activity.len() as u64,
            );
            (federation_summary, avg_txs, avg_volume)
        })
        .sorted_by(|(a, _, _), (b, _, _)| {
            rating_index(&b.nostr_votes).total_cmp(&rating_index(&a.nostr_votes))
        })
        .collect::<Vec<_>>();

    Ok(federations)
}
