use chrono::{DateTime, Utc};
use leptos::{component, view, IntoView, RwSignal, Signal, SignalGet};
use leptos_chartistry::{
    AspectRatio, AxisMarker, Chart, Interpolation, IntoInner, Line, Period, RotatedLabel, Series,
    TickLabels, Timestamps, Tooltip, XGridLine, XGuideLine, YGridLine, YGuideLine,
};
use leptos_use::use_preferred_dark;

#[component]
pub fn TimeLineChart(
    #[prop(into)] name: RwSignal<String>,
    #[prop(into)] data: Signal<Vec<(DateTime<Utc>, f64)>>,
) -> impl IntoView {
    let prefers_dark = use_preferred_dark();

    let line = {
        let mut line = Line::new(|data: &(DateTime<Utc>, f64)| data.1)
            .with_interpolation(Interpolation::Linear);
        line.name = name;
        line
    };

    let top_label = {
        let mut label = RotatedLabel::middle("");
        label.text = name;
        label
    };

    view! {
        <div style=move || {
            if prefers_dark.get() {
                "fill: white"
            } else {
                "fill: black"
            }
        }>
            <Chart
                // Sets the width and height
                aspect_ratio=AspectRatio::from_env_width(300.0)

                // Decorate our chart
                top=top_label
                left=TickLabels::aligned_floats().with_min_chars(6)
                bottom=TickLabels::from_generator(Timestamps::from_period(Period::Month))
                inner=[
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    XGridLine::default().into_inner(),
                    YGridLine::default().into_inner(),
                    XGuideLine::over_data().into_inner(),
                    YGuideLine::over_mouse().into_inner(),
                ]
                tooltip=Tooltip::left_cursor()
                series=Series::new(|data: &(DateTime<Utc>, f64)| data.0).line(line)
                data=data
            />
        </div>
    }
}
