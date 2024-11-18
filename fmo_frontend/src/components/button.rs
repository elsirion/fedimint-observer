use std::borrow::Cow;

use leptos::{component, view, Children, IntoView, MaybeSignal, SignalGet};

#[component]
pub fn Button<F: FnOnce() + Copy + 'static>(
    #[prop(default = PRIMARY_BUTTON)] color_scheme: ButtonColors,
    #[prop(into, optional)] class: String,
    on_click: F,
    #[prop(into, optional)] disabled: MaybeSignal<bool>,
    children: Children,
) -> impl IntoView {
    const COMMON: &str = "whitespace-nowrap font-medium rounded-lg text-sm px-5";

    let class = move || {
        let color_scheme_class = if disabled.get() {
            &color_scheme.disabled
        } else {
            &color_scheme.enabled
        };

        format!("{} {} {}", COMMON, color_scheme_class, class)
    };

    view! {
        <button
            class=class
            on:click=move |_| on_click()
            disabled=disabled
            type="button"
        >
            { children() }
        </button>
    }
}

pub const PRIMARY_BUTTON: ButtonColors = ButtonColors::new(
    "text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800",
    "text-white bg-blue-400 dark:bg-blue-500 cursor-not-allowed font-medium rounded-lg"
);
pub const SECONDARY_BUTTON: ButtonColors = ButtonColors::new(
    "text-gray-900 focus:outline-none bg-white border border-gray-200 hover:bg-gray-100 hover:text-blue-700 focus:z-10 focus:ring-4 focus:ring-gray-100 dark:focus:ring-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600 dark:hover:text-white dark:hover:bg-gray-700",
    "text-gray-400 dark:text-gray-600 cursor-not-allowed"

);
pub const SUCCESS_BUTTON: ButtonColors = ButtonColors::new(
    "text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 dark:bg-green-600 dark:hover:bg-green-700 focus:outline-none",
    "text-white bg-green-400 dark:bg-green-500 cursor-not-allowed font-medium rounded-lg"
);

#[derive(Debug, Clone)]
pub struct ButtonColors {
    enabled: Cow<'static, str>,
    disabled: Cow<'static, str>,
}

impl ButtonColors {
    pub const fn new(enabled: &'static str, disabled: &'static str) -> Self {
        Self {
            enabled: Cow::Borrowed(enabled),
            disabled: Cow::Borrowed(disabled),
        }
    }
}
