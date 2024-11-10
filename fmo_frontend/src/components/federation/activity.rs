use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::Mul;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fmo_api_types::FederationActivity;
use itertools::Itertools;
use leptos::{
    component, create_effect, create_resource, create_signal, event_target_value, view, IntoView,
    RwSignal, Show, SignalGet, SignalSet, SignalUpdate,
};

use super::chart::TimeLineChart;
use crate::components::alert::{Alert, AlertLevel};
use crate::util::AsBitcoin;

#[component]
pub fn ActivityChart(id: FederationId) -> impl IntoView {
    let history_resource = create_resource(
        || (),
        move |()| async move {
            fetch_federation_history(id)
                .await
                .map_err(|e| e.to_string())
        },
    );

    view! {
        {move || {
            match history_resource.get() {
                Some(Ok(history)) => view! { <ChartInner data=history/> }.into_view(),
                Some(Err(e)) => view! { <p>"Error: " {e}</p> }.into_view(),
                None => view! { <p>"Loading ..."</p> }.into_view(),
            }
        }}
    }
}

#[component]
pub fn ChartInner(data: BTreeMap<NaiveDate, FederationActivity>) -> impl IntoView {
    let (total_volume, volumes_btc) = {
        let total = Amount::from_msats(
            data.values()
                .map(|data| data.amount_transferred.msats)
                .sum::<u64>(),
        );
        let volumes_btc = data
            .iter()
            .map(|(date, data)| {
                (
                    NaiveDateTime::from(*date).and_utc(),
                    data.amount_transferred.msats as f64 / 100_000_000_000.0,
                )
            })
            .collect::<Vec<_>>();

        (total, volumes_btc)
    };

    let (total_transactions, transactions) = {
        let total = data.values().map(|data| data.num_transactions).sum::<u64>();
        let transactions = data
            .iter()
            .map(|(date, data)| {
                (
                    NaiveDateTime::from(*date).and_utc(),
                    data.num_transactions as f64,
                )
            })
            .collect::<Vec<_>>();
        (total, transactions)
    };

    let (chart_type, set_chart_type) = create_signal(ChartType::Volume);
    let (filter_outliers, set_filter_outliers) = create_signal(true);

    let chart_name_signal = RwSignal::new("".to_owned());
    create_effect(move |_| {
        let chart_name = match chart_type.get() {
            ChartType::Volume => "Daily Volume",
            ChartType::Transactions => "Daily Transactions",
        }
        .to_owned();

        chart_name_signal.set(chart_name);
    });

    let chart_data = move || match chart_type.get() {
        ChartType::Volume if filter_outliers.get() => remove_outliers(volumes_btc.clone()),
        ChartType::Volume => volumes_btc.clone(),
        ChartType::Transactions => transactions.clone(),
    };

    view! {
        <Alert
            message="Some transaction types, like Lightning transactions, cause more than one Fedimint transaction."
            level=AlertLevel::Info
            class="my-4"
        />
        <div class="w-full bg-white rounded-lg shadow dark:bg-gray-800 p-4 md:p-6">
            <div class="flex justify-between">
                <div>
                    <h5 class="leading-none text-3xl font-bold text-gray-900 dark:text-white pb-2">
                        {move || {
                            match chart_type.get() {
                                ChartType::Volume => total_volume.as_bitcoin(6).to_string(),
                                ChartType::Transactions => total_transactions.to_string(),
                            }
                        }}

                    </h5>
                    <p class="text-base font-normal text-gray-500 dark:text-gray-400">
                        {move || {
                            match chart_type.get() {
                                ChartType::Volume => "Total Volume",
                                ChartType::Transactions => "Total Transactions",
                            }
                        }}

                    </p>
                </div>
                <Show when=move || chart_type.get() == ChartType::Volume>
                    <div class="flex items-center mb-4">
                        <input
                            id="default-checkbox"
                            type="checkbox"
                            class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                            checked=move || filter_outliers.get()
                            on:change=move |_| set_filter_outliers.update(|v| *v = !*v)
                        />
                        <label
                            for="default-checkbox"
                            class="ms-2 text-sm font-medium text-gray-900 dark:text-gray-300"
                            title="Filter out values that are more than 10 times the 95th percentile"
                        >
                            Filter Extreme Outliers
                        </label>
                    </div>
                </Show>
                <div
                    class="max-w-sm"
                >
                    <select
                        class="bg-gray-50 border border-gray-300 text-gray-900 mb-6 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                        on:change=move |ev| {
                            let new_value = event_target_value(&ev);
                            set_chart_type.set(new_value.parse().unwrap());
                        }

                        prop:value=move || chart_type.get().to_string()
                    >
                        <option value="Volume">"Volume"</option>
                        <option value="Transactions">"Transactions"</option>
                    </select>
                </div>
            </div>

            <TimeLineChart name=chart_name_signal data=chart_data />

        </div>
    }
}

async fn fetch_federation_history(
    federation_id: FederationId,
) -> Result<BTreeMap<NaiveDate, FederationActivity>, String> {
    let url = format!(
        "{}/federations/{}/transactions/histogram",
        crate::BASE_URL,
        federation_id
    );
    let res = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let json = res.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChartType {
    Volume,
    Transactions,
}

impl FromStr for ChartType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Volume" => Ok(Self::Volume),
            "Transactions" => Ok(Self::Transactions),
            _ => Err(()),
        }
    }
}

impl Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Volume => write!(f, "Volume"),
            Self::Transactions => write!(f, "Transactions"),
        }
    }
}

fn remove_outliers<T>(data: Vec<(DateTime<Utc>, T)>) -> Vec<(DateTime<Utc>, T)>
where
    T: Copy + PartialOrd + Mul<Output = T> + From<u8>,
{
    let percentile_95 = data
        .iter()
        .map(|(_, val)| *val)
        .sorted_by(|a, b| a.partial_cmp(b).expect("No NaNs expected"))
        .collect::<Vec<_>>()[data.len() * 95 / 100];

    data.into_iter()
        .filter(|(_, val)| *val < percentile_95 * T::from(10u8))
        .collect()
}
