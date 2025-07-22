use fedimint_core::util::backoff_util::background_backoff;
use fedimint_core::util::retry;
use fmo_api_types::FedimintTotals;
use leptos::prelude::*;
use num_format::{Locale, ToFormattedString};

#[component]
pub fn Totals() -> impl IntoView {
    let totals_res = LocalResource::new(|| async {
        retry(
            "fetching federation totals",
            background_backoff(),
            fetch_federation_totals,
        )
        .await
        .expect("Will never return Err")
    });

    view! {
        <div class="flex items-center justify-center space-x-10 dark:text-white">
            <div class="text-center">
                {move || {
                    match totals_res.get() {
                        Some(totals) => {
                            view! {
                                <div class="text-4xl font-bold mb-2">
                                    {totals.federations.to_formatted_string(&Locale::en)}
                                </div>
                            }
                        }
                        None => {
                            view! {
                                <div class="text-4xl font-bold mb-2">
                                    {"Loading...".to_string()}
                                </div>
                            }
                        }
                    }
                }}
                <div class="text-gray-500">Observed Federations</div>
            </div>
            <div class="border-l border-gray-300 h-12"></div>
            <div class="text-center">
                {move || {
                    match totals_res.get() {
                        Some(totals) => {
                            view! {
                                <div class="text-4xl font-bold mb-2 dark:text-white">
                                    {totals.tx_count.to_formatted_string(&Locale::en)}
                                </div>
                            }
                        }
                        None => {
                            view! {
                                <div class="text-4xl font-bold mb-2 dark:text-white">
                                    {"Loading...".to_string()}
                                </div>
                            }
                        }
                    }
                }}
                <div class="text-gray-500">Total Transactions</div>
            </div>
            <div class="border-l border-gray-300 h-12"></div>
            <div class="text-center">
                {move || {
                    match totals_res.get() {
                        Some(totals) => {
                            view! {
                                <div class="text-4xl font-bold dark:text-white">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="40"
                                        height="40"
                                        fill="currentColor"
                                        class="bi bi-currency-bitcoin"
                                        viewBox="0 0 16 16"
                                        class="inline mb-2"
                                    >
                                        <path d="M5.5 13v1.25c0 .138.112.25.25.25h1a.25.25 0 0 0 .25-.25V13h.5v1.25c0 .138.112.25.25.25h1a.25.25 0 0 0 .25-.25V13h.084c1.992 0 3.416-1.033 3.416-2.82 0-1.502-1.007-2.323-2.186-2.44v-.088c.97-.242 1.683-.974 1.683-2.19C11.997 3.93 10.847 3 9.092 3H9V1.75a.25.25 0 0 0-.25-.25h-1a.25.25 0 0 0-.25.25V3h-.573V1.75a.25.25 0 0 0-.25-.25H5.75a.25.25 0 0 0-.25.25V3l-1.998.011a.25.25 0 0 0-.25.25v.989c0 .137.11.25.248.25l.755-.005a.75.75 0 0 1 .745.75v5.505a.75.75 0 0 1-.75.75l-.748.011a.25.25 0 0 0-.25.25v1c0 .138.112.25.25.25zm1.427-8.513h1.719c.906 0 1.438.498 1.438 1.312 0 .871-.575 1.362-1.877 1.362h-1.28zm0 4.051h1.84c1.137 0 1.756.58 1.756 1.524 0 .953-.626 1.45-2.158 1.45H6.927z"></path>
                                    </svg>
                                    {format!(
                                        "{:.*}",
                                        5,
                                        totals.tx_volume.msats as f64 / 100_000_000_000f64,
                                    )}

                                </div>
                            }
                        }
                        None => {
                            view! {
                                <div class="text-4xl font-bold dark:text-white">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="40"
                                        height="40"
                                        fill="currentColor"
                                        class="bi bi-currency-bitcoin"
                                        viewBox="0 0 16 16"
                                        class="inline mb-2"
                                    >
                                        <path d="M5.5 13v1.25c0 .138.112.25.25.25h1a.25.25 0 0 0 .25-.25V13h.5v1.25c0 .138.112.25.25.25h1a.25.25 0 0 0 .25-.25V13h.084c1.992 0 3.416-1.033 3.416-2.82 0-1.502-1.007-2.323-2.186-2.44v-.088c.97-.242 1.683-.974 1.683-2.19C11.997 3.93 10.847 3 9.092 3H9V1.75a.25.25 0 0 0-.25-.25h-1a.25.25 0 0 0-.25.25V3h-.573V1.75a.25.25 0 0 0-.25-.25H5.75a.25.25 0 0 0-.25.25V3l-1.998.011a.25.25 0 0 0-.25.25v.989c0 .137.11.25.248.25l.755-.005a.75.75 0 0 1 .745.75v5.505a.75.75 0 0 1-.75.75l-.748.011a.25.25 0 0 0-.25.25v1c0 .138.112.25.25.25zm1.427-8.513h1.719c.906 0 1.438.498 1.438 1.312 0 .871-.575 1.362-1.877 1.362h-1.28zm0 4.051h1.84c1.137 0 1.756.58 1.756 1.524 0 .953-.626 1.45-2.158 1.45H6.927z"></path>
                                    </svg>
                                    {"Loading...".to_string()}
                                </div>
                            }
                        }
                    }
                }}
                <div class="text-gray-500">Total Volume</div>
            </div>
        </div>
    }
}

async fn fetch_federation_totals() -> anyhow::Result<FedimintTotals> {
    let url = format!("{}/federations/totals", crate::BASE_URL);
    let res = reqwest::get(&url).await?;
    Ok(res.json().await?)
}
