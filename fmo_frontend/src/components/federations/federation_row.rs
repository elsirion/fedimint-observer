use std::str::FromStr;

use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fmo_api_types::FederationRating;
use leptos::{component, view, IntoView};

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
) -> impl IntoView {
    let offline_federations = [FederationId::from_str(
        "4b13a146ee4ba732b2b8914a72a0a2e5873e3e942da2d4eeefd85a5fe41f27ba",
    )
    .expect("can be parsed")];

    let degraded_federations = vec![FederationId::from_str(
        "c944b2fd1e7fe04ca87f9a57d7894cb69116cec6264cb52faa71228f4ec54cd6",
    )
    .expect("can be parsed")];

    if offline_federations.contains(&id) {
        return view! {}.into_view();
    }

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
                {if degraded_federations.contains(&id) {
                    view! {
                        <span class="bg-yellow-100 text-yellow-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-gray-700 dark:text-yellow-300 border border-yellow-300">
                            "Degraded"
                        </span>
                    }
                        .into_view()
                } else {
                    view! { <Copyable text=invite/> }.into_view()
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
