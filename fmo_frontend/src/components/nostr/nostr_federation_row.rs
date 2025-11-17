use fedimint_core::config::FederationId;
use fedimint_core::invite_code::InviteCode;
use leptos::prelude::*;

use crate::components::badge::{Badge, BadgeLevel};
use crate::components::Copyable;

#[component]
pub fn NostrFederationRow(
    federation_id: FederationId,
    invite_code: InviteCode,
    is_observed: bool,
    federation_name: Option<String>,
) -> impl IntoView {
    view! {
        <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
            <th
                scope="row"
                class="px-6 py-4 font-medium text-gray-900 whitespace-nowrap dark:text-white"
            >
                <div class="flex items-center gap-2">
                    <span>
                        {federation_name.clone().unwrap_or_else(|| federation_id.to_string())}
                    </span>
                    {
                        // Show "Unobserved" badge if we have a name and federation is not observed
                        if !is_observed && federation_name.is_some() {
                            Some(view! {
                                <Badge level=BadgeLevel::Info tooltip=Some("Not currently observed by this instance".to_string())>
                                    "Unobserved"
                                </Badge>
                            })
                        } else {
                            None
                        }
                    }
                </div>
            </th>
            <td>
                <Copyable text=invite_code.to_string()/>
            </td>
        </tr>
    }
}
