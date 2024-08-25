use leptos::{component, view, IntoView};

#[component]
pub fn Alert(
    #[prop(into, optional)] title: Option<String>,
    #[prop(into)] message: String,
    level: AlertLevel,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let style = match level {
        AlertLevel::Info => "p-4 mb-4 text-sm text-blue-800 rounded-lg bg-blue-50 dark:bg-gray-800 dark:text-blue-400",
        AlertLevel::Warning => "p-4 mb-4 text-sm text-yellow-800 rounded-lg bg-yellow-50 dark:bg-gray-800 dark:text-yellow-300",
        AlertLevel::Error => "p-4 mb-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400",
        AlertLevel::Success => "p-4 mb-4 text-sm text-green-800 rounded-lg bg-green-50 dark:bg-gray-800 dark:text-green-400",
    };

    let class = format!("{} {}", style, class);

    let title = title.unwrap_or_else(|| match level {
        AlertLevel::Info => "Info: ".to_string(),
        AlertLevel::Warning => "Warning: ".to_string(),
        AlertLevel::Error => "Error: ".to_string(),
        AlertLevel::Success => "Success: ".to_string(),
    });

    view! {
        <div class=class role="alert">
          <span class="font-bold">{title}</span> {message}
        </div>
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Success,
}
