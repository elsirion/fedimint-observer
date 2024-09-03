use fedimint_core::config::FederationId;
use fmo_api_types::FederationUtxo;
use leptos::{component, create_resource, view, IntoView, SignalGet};

use crate::components::alert::{Alert, AlertLevel};
use crate::util::AsBitcoin;

#[component]
pub fn Utxos(federation_id: FederationId) -> impl IntoView {
    let utxo_resource = create_resource(|| (), move |()| fetch_federation_utxos(federation_id));

    view! {
        {move || {
            match utxo_resource.get() {
                Some(Ok(utxos)) => {
                    let rows = utxos
                        .iter()
                        .map(|utxo| {
                            view! {
                                <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                    <td class="px-6 py-4">
                                        <a
                                            href=format!(
                                                "https://mempool.space/address/{}",
                                                utxo.address.clone().assume_checked().to_string(),
                                            )

                                            class="text-blue-600 underline dark:text-blue-500 hover:no-underline"
                                        >
                                            <pre>
                                                <span class="truncate flex-shrink min-w-0">
                                                    {utxo.out_point.txid.to_string()}
                                                </span>
                                                <span class="flex-shrink-0">
                                                    ":" {utxo.out_point.vout.to_string()}
                                                </span>
                                            </pre>
                                        </a>
                                    </td>
                                    <td class="px-6 py-4">
                                        {utxo.amount.as_bitcoin(8).to_string()}
                                    </td>
                                </tr>
                            }
                        })
                        .collect::<Vec<_>>();
                    view! {
                        <div>
                            <Alert
                                message="The UTXO view is reconstructed from a combination of the public federation log and on-chain transactions, hence unconfirmed change UTXOs may be missing."
                                level=AlertLevel::Info
                                class="my-4"
                            />
                            <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                                <thead class="text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400">
                                    <tr>
                                        <th scope="col" class="px-6 py-3">
                                            "UTXOs ("
                                            {utxos.len()}
                                            " total)"
                                        </th>
                                        <th scope="col" class="px-6 py-3">
                                            Amount
                                        </th>
                                    </tr>
                                </thead>
                                <tbody>{rows}</tbody>
                            </table>
                        </div>
                    }
                        .into_view()
                }
                Some(Err(e)) => view! { <p>"Error: " {e}</p> }.into_view(),
                None => view! { <p>"Loading ..."</p> }.into_view(),
            }
        }}
    }
}

async fn fetch_federation_utxos(
    federation_id: FederationId,
) -> Result<Vec<FederationUtxo>, String> {
    let url = format!("{}/federations/{}/utxos", crate::BASE_URL, federation_id);
    let res = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let json = res.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}
