use leptos::{component, view, Children, IntoView};

#[component]
pub fn Badge(
    level: BadgeLevel,
    #[prop(default = None)] tooltip: Option<String>,
    children: Children,
) -> impl IntoView {
    let style = match level {
        BadgeLevel::Info => "bg-blue-100 text-blue-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-blue-900 dark:text-blue-300",
        BadgeLevel::Warning => "bg-yellow-100 text-yellow-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-yellow-900 dark:text-yellow-300",
        BadgeLevel::Error => "bg-red-100 text-red-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-red-900 dark:text-red-300",
        BadgeLevel::Success => "bg-green-100 text-green-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-green-900 dark:text-green-300",
    };

    match tooltip {
        Some(tooltip_str) => {
            view! {
                <span class=style>
                    <abbr title=tooltip_str>
                        { children() }
                    </abbr>
                </span>
            }
        }
        None => {
            view! {
                <span class=style>
                    { children() }
                </span>
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum BadgeLevel {
    Info,
    Warning,
    Error,
    Success,
}
