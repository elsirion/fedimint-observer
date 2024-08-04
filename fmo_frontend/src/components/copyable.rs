use leptos::{component, view, IntoView};
use web_sys::window;

// TODO: on_success/on_failure callbacks
#[component]
pub fn Copyable(text: String) -> impl IntoView {
    let text_inner = text.clone();
    let copy_action = move |_| {
        if let Some(clipboard) = window().and_then(|w| w.navigator().clipboard()) {
            let _ = clipboard.write_text(&text_inner);
        } else {
            // handle the lack of clipboard!
        }
    };

    view! {
        <input value=text/>
        <button
            on:click=copy_action
            class="ml-2 text-gray-900 bg-white hover:bg-gray-100 border border-gray-200 focus:ring-4 focus:outline-none focus:ring-gray-100 rounded-lg text-sm px-1 py-1 text-center inline-flex items-center dark:focus:ring-gray-600 dark:bg-gray-800 dark:border-gray-700 dark:text-white dark:hover:bg-gray-700 me-1 mb-1"
        >
            <svg
                class="w-[18px] h-[18px] text-gray-800 dark:text-white"
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                fill="none"
                viewBox="0 0 24 24"
            >
                <path
                    stroke="currentColor"
                    stroke-linejoin="round"
                    stroke-width="2.3"
                    d="M9 8v3a1 1 0 0 1-1 1H5m11 4h2a1 1 0 0 0 1-1V5a1 1 0 0 0-1-1h-7a1 1 0 0 0-1 1v1m4 3v10a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-7.13a1 1 0 0 1 .24-.65L7.7 8.35A1 1 0 0 1 8.46 8H13a1 1 0 0 1 1 1Z"
                ></path>
            </svg>
        </button>
    }
}
