mod federation_row;
pub mod rating;
mod totals;

use std::str::FromStr;

use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fmo_api_types::{FederationHealth, FederationSummary};
use leptos::{component, create_resource, view, IntoView, SignalGet};
use leptos_meta::Title;

use crate::components::federations::federation_row::FederationRow;
use crate::components::federations::totals::Totals;
use crate::BASE_URL;

#[component]
pub fn Federations() -> impl IntoView {
    let federations_res = create_resource(
        || (),
        |_| async { fetch_federations().await.map_err(|e| e.to_string()) },
    );

    let rows = move || {
        Some(
            federations_res
                .get()?
                .ok()?
                .into_iter()
                .map(|(summary, avg_txs, avg_volume)| {
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
                })
                .collect::<Vec<_>>(),
        )
    };

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
                        "List of all federations this instance is collecting statistics on"
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
                <tbody>{rows}</tbody>
            </table>
        </div>
    }
}

async fn fetch_federations() -> anyhow::Result<Vec<(FederationSummary, f64, Amount)>> {
    let url = format!("{}/federations", BASE_URL);
    let response = reqwest::get(&url).await?;
    let federations: Vec<FederationSummary> = response.json().await?;

    let federations = federations
        .into_iter()
        .filter_map(|federation_summary| {
            // Don't show offline federations for now. Eventually I'd like to only not show
            // them if they have been offline for a long time.
            if federation_summary.id
                == FederationId::from_str(
                    "4b13a146ee4ba732b2b8914a72a0a2e5873e3e942da2d4eeefd85a5fe41f27ba",
                )
                .unwrap()
            {
                return None;
            }

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
            Some((federation_summary, avg_txs, avg_volume))
        })
        .collect::<Vec<_>>();

    Ok(federations)
}
