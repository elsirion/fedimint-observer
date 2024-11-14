use std::collections::BTreeMap;

use anyhow::Context;
use fedimint_core::config::FederationId;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::util::backon::FibonacciBuilder;
use fedimint_core::util::retry;
use leptos::{component, create_resource, view, IntoView, SignalGet};

use crate::components::Copyable;
use crate::BASE_URL;

#[component]
pub fn NostrFederationRow(federation_id: FederationId, invite_code: InviteCode) -> impl IntoView {
    let invite_code_inner = invite_code.clone();
    let federation_name_res = create_resource(
        || (),
        move |_| fetch_federation_name(invite_code_inner.clone()),
    );

    view! {
        <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
            <th
                scope="row"
                class="px-6 py-4 font-medium text-gray-900 whitespace-nowrap dark:text-white"
            >
                { move || {
                    match federation_name_res.get() {
                        Some(name) => name,
                        None => federation_id.to_string(),
                    }
                }}
            </th>
            <td>
                <Copyable text=invite_code.to_string()/>
            </td>
        </tr>
    }
}

async fn fetch_federation_name(invite_code: InviteCode) -> String {
    let url = format!("{}/config/{invite_code}/meta", BASE_URL);

    let fetch_federation_name_impl = || {
        let url_inner = url.clone();
        async move {
            let response = reqwest::get(&url_inner).await?;
            let federation: BTreeMap<String, serde_json::Value> = response.json().await?;
            Ok(federation
                .get("federation_name")
                .context("No name found")?
                .as_str()
                .context("Name isn't a string")?
                .to_owned())
        }
    };

    retry(
        "Fetching federation name",
        FibonacciBuilder::default().with_max_times(usize::MAX),
        fetch_federation_name_impl,
    )
    .await
    .expect("Won't terminate")
}
