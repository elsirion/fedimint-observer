use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use fedimint_core::Amount;
use fedimint_core::config::FederationId;
use fmo_api_types::FederationActivity;
use leptos::{component, create_resource, create_signal, event_target_value, IntoView, SignalGet, view};
use leptos_chartistry::*;
use itertools::Itertools;
use leptos::SignalSet;
use crate::util::AsBitcoin;


#[component]
pub fn ActivityChart(
    id: FederationId
) -> impl IntoView {
    let history_resource = create_resource(
        || (),
        move|()| async move { fetch_federation_history(id).await.map_err(|e| e.to_string()) },
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
pub fn ChartInner(
    data: BTreeMap<NaiveDate, FederationActivity>
) -> impl IntoView {

    let (cleaned_total_volume, cleaned_volumes_btc) = {
        // Sometimes we have a few very large outlier that are likely some bug or attack
        let volume_95th_percentile = data.values().map(|activity| activity.amount_transferred.msats).sorted().collect::<Vec<_>>()[data.len() * 95 / 100];
        let volumes_cleaned = data.iter().filter(|(date, data)| data.amount_transferred.msats < volume_95th_percentile).collect::<Vec<_>>();
        let total = Amount::from_msats(volumes_cleaned.iter().map(|(_, data)| data.amount_transferred.msats).sum::<u64>());
        let volumes_btc = volumes_cleaned.into_iter().map(|(date, data)| (NaiveDateTime::from(*date).and_utc(), data.amount_transferred.msats as f64 / 100_000_000_000.0)).collect::<Vec<_>>();

        (total, volumes_btc)
    };

    let (total_transactions, transactions) = {
        let total = data.iter().map(|(_, data)| data.num_transactions).sum::<u64>();
        let transactions = data.iter().map(|(date, data)| (NaiveDateTime::from(*date).and_utc(), data.num_transactions as f64)).collect::<Vec<_>>();
        (total, transactions)
    };

    let (chart_type, set_chart_type) = create_signal(ChartType::Volume);

    view! {
        <div class="w-full bg-white rounded-lg shadow dark:bg-gray-800 p-4 md:p-6 m-4">
            <div class="flex justify-between">
                <div>
                    <h5 class="leading-none text-3xl font-bold text-gray-900 dark:text-white pb-2">
                        {move || {
                            match chart_type.get() {
                                ChartType::Volume => cleaned_total_volume.as_bitcoin(6).to_string(),
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
                <select
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

            {move || {
                match chart_type.get() {
                    ChartType::Volume => {
                        view! { <VolumeChart data=cleaned_volumes_btc.clone()/> }
                    }
                    ChartType::Transactions => {
                        view! { <TransactionsChart data=transactions.clone()/> }
                    }
                }
            }}

        </div>
    }
}

#[component]
fn VolumeChart(data: Vec<(DateTime<Utc>, f64)>) -> impl IntoView {
    view! {
        <Chart
            // Sets the width and height
            aspect_ratio=AspectRatio::from_env_width(300.0)

            // Decorate our chart
            top=RotatedLabel::middle("Federation Activity")
            left=TickLabels::aligned_floats()
            bottom=TickLabels::timestamps()
            inner=[
                AxisMarker::left_edge().into_inner(),
                AxisMarker::bottom_edge().into_inner(),
                XGridLine::default().into_inner(),
                YGridLine::default().into_inner(),
            ]

            // Describe the data
            series=Series::new(|data: &(DateTime<Utc>, f64)| data.0)
                .line(
                    Line::new(|data: &(DateTime<Utc>, f64)| data.1)
                        .with_name("Volume")
                        .with_interpolation(Interpolation::Linear),
                )
            data=move || data.clone()
        />
    }
}

#[component]
fn TransactionsChart(data: Vec<(DateTime<Utc>, f64)>) -> impl IntoView {
    view! {
        <Chart
            // Sets the width and height
            aspect_ratio=AspectRatio::from_env_width(300.0)

            // Decorate our chart
            top=RotatedLabel::middle("Federation Activity")
            left=TickLabels::aligned_floats()
            bottom=TickLabels::timestamps()
            inner=[
                AxisMarker::left_edge().into_inner(),
                AxisMarker::bottom_edge().into_inner(),
                XGridLine::default().into_inner(),
                YGridLine::default().into_inner(),
                XGuideLine::over_data().into_inner(),
                YGuideLine::over_mouse().into_inner(),
            ]

            // Describe the data
            series=Series::new(|data: &(DateTime<Utc>, f64)| data.0)
                .line(
                    Line::new(|data: &(DateTime<Utc>, f64)| data.1)
                        .with_name("Transactions")
                        .with_interpolation(Interpolation::Linear),
                )
            data=move || data.clone()
        />
    }
}

async fn fetch_federation_history(federation_id: FederationId) -> Result<BTreeMap<NaiveDate, FederationActivity>, String> {
    let url = format!("{}/federations/{}/transactions/histogram", crate::BASE_URL, federation_id);
    let res = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let json = res.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[derive(Debug, Clone, Copy)]
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