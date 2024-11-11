use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fmo_api_types::{FederationHealth, FederationRating};
use leptos::{component, view, IntoView};

use crate::components::badge::{Badge, BadgeLevel};
use crate::components::federations::rating::Rating;
use crate::components::Copyable;
use crate::util::AsBitcoin;

#[component]
pub fn FederationRow(
    id: FederationId,
    name: String,
    rating: FederationRating,
    invite: String,
    total_assets: Amount,
    avg_txs: f64,
    avg_volume: Amount,
    health: FederationHealth,
) -> impl IntoView {
    view! {
        <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
            <th
                scope="row"
                class="px-6 py-4 font-medium text-gray-900 whitespace-nowrap dark:text-white"
            >
                <a
                    href=format!("/federations/{id}")
                    class="font-medium text-blue-600 dark:text-blue-500 hover:underline"
                >
                    {name}
                </a>
            </th>
            <td>
                <Rating
                    count=rating.count
                    rating=rating.avg
                />
            </td>
            <td class="px-6 py-4">
                { match health {
                    FederationHealth::Online => {
                        view! { <Copyable text=invite/> }.into_view()
                    },
                    FederationHealth::Degraded => {
                        view! {
                            <Badge level=BadgeLevel::Warning>
                                "Degraded"
                            </Badge>
                        }.into_view()
                    }
                    FederationHealth::Offline => {
                        view! {
                            <Badge level=BadgeLevel::Error>
                                "Offline"
                            </Badge>
                        }.into_view()
                    },
                }}
            </td>
            <td class="px-6 py-4">{total_assets.as_bitcoin(6).to_string()}</td>
            <td class="px-6 py-4">
                <ul>
                    <li>{format!("#tx: {:.1}", avg_txs)}</li>
                    <li>{format!("volume: {}", avg_volume.as_bitcoin(6))}</li>
                </ul>
            </td>
        </tr>
    }
    .into_view()
}
