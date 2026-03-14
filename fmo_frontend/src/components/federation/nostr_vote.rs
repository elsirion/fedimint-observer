use anyhow::ensure;
use fedimint_core::config::{FederationId, JsonClientConfig};
use leptos::prelude::*;
use nostr_sdk::{EventBuilder, Kind, SingleLetterTag, Tag, TagKind};
use reqwest::StatusCode;

use crate::components::alert::{Alert, AlertLevel};
use crate::components::federation::stars_selector::StarsSelector;
use crate::BASE_URL;

#[component]
pub fn NostrVote(config: JsonClientConfig) -> impl IntoView {
    let federation_id = config.global.calculate_federation_id();

    let (in_progress, set_in_progress) = signal(false);
    let sign_rating_action = Action::<(u8, String), std::result::Result<(), String>>::new_local(
        move |(rating, comment): &(u8, String)| {
            let comment_inner = comment.clone();
            let rating_inner = *rating;
            async move {
                let res = sign_and_publish_rating(federation_id, rating_inner, &comment_inner)
                    .await
                    .map_err(|e| e.to_string());
                set_in_progress.set(false);
                res
            }
        },
    );

    let (rating, set_rating) = signal(5u8);
    let (comment, st_comment) = signal("".to_owned());
    view! {
        <div class="w-full p-4 bg-white border border-gray-200 rounded-lg shadow sm:p-8 dark:bg-gray-800 dark:border-gray-700">
            <div class="flex items-center justify-between mb-4">
                <h5 class="text-xl font-bold leading-none text-gray-900 dark:text-white">
                    Recommend
                </h5>
            </div>
            <div class="flow-root">
                <div class="relative overflow-x-auto">
                    <form
                        on:submit=move |ev| {
                            ev.prevent_default();
                            set_in_progress.set(true);
                            sign_rating_action.dispatch((rating.get(), comment.get()));
                        }
                    >
                        { move || {
                            match sign_rating_action.value().get() {
                                Some(Ok(())) => {
                                    view! {
                                        <Alert
                                            level=AlertLevel::Success
                                            message="Your recommendation was published!"
                                        />
                                    }.into_any()
                                },
                                Some(Err(e)) => {
                                    view! {
                                        <Alert
                                            level=AlertLevel::Error
                                            message=e
                                        />
                                    }.into_any()
                                },
                                None => {
                                    view! { <div style="display: none;"><Alert message="" level=AlertLevel::Info /></div> }.into_any()
                                }
                            }
                        }}
                        <div class="mb-6">
                            <div>
                                <StarsSelector
                                    default_value=5
                                    selected_stars=set_rating
                                />
                                <span class="mx-4 inline-block text-xl dark:text-white">
                                    {move || {rating.get()}} "/5"
                                </span>
                            </div>
                        </div>
                        <div class="mb-6">
                            <input
                                class="block w-full p-4 text-gray-900 border border-gray-300 rounded-lg bg-gray-50 text-base focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                                type="text"
                                placeholder="Comment"
                                on:input=move |ev| {
                                    st_comment.set(event_target_value(&ev));
                                }
                                prop:value=comment
                            />
                        </div>
                        { move || {
                            let is_disabled = in_progress.get();
                            view! {
                                <input
                                    type="submit"
                                    value="Rate"
                                    disabled=is_disabled
                                    class=move || if is_disabled {
                                        "text-white bg-blue-400 dark:bg-blue-500 cursor-not-allowed font-medium rounded-lg text-sm px-5 py-2.5 text-center"
                                    } else {
                                        "text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800"
                                    }
                                />
                            }
                        }}
                    </form>
                </div>
            </div>
        </div>
    }
}

async fn sign_and_publish_rating(
    federation_id: FederationId,
    rating: u8,
    comment: &str,
) -> anyhow::Result<()> {
    let signer = nostr_sdk::nostr::nips::nip07::Nip07Signer::new()?;

    let tags = vec![
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('d').unwrap()),
            [federation_id.to_string()],
        ),
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('n').unwrap()),
            ["mainnet"],
        ),
        Tag::custom(
            TagKind::SingleLetter(SingleLetterTag::from_char('k').unwrap()),
            ["38173"],
        ),
    ];
    let unsigned_event =
        EventBuilder::new(Kind::Custom(38000), format!("[{rating}/5] {comment}"), tags)
            .to_unsigned_event(signer.get_public_key().await?);

    let event = signer.sign_event(unsigned_event).await?;

    let client = reqwest::Client::new();
    let response = client
        .put(format!("{}/federations/nostr/rating", BASE_URL))
        .json(&event)
        .send()
        .await?;

    let status = response.status();
    ensure!(
        status == StatusCode::OK,
        "Unexpected status code {}",
        status
    );

    Ok(())
}
