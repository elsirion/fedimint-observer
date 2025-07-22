use leptos::prelude::*;

#[component]
pub fn Tabs(#[prop(into)] default: String, children: Children) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(default.clone());

    // Extract tab names from children - this is a simplified approach
    // In Leptos 0.8.x, we'll use a more direct approach
    let tabs = ["Activity", "UTXOs", "Config"];

    let tab_buttons = tabs.iter().map(|tab_name| {
        let tab_name_owned = tab_name.to_string();
        let tab_name_click = tab_name_owned.clone();

        const INACTIVE_CLASSES: &str = "inline-block p-4 border-b-2 border-transparent rounded-t-lg hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300";
        const ACTIVE_CLASSES: &str = "inline-block p-4 text-blue-600 border-b-2 border-blue-600 rounded-t-lg active dark:text-blue-500 dark:border-blue-500";

        view! {
            <li class="me-2">
                <a
                    href="#"
                    class=move || {
                        if tab_name_owned == active_tab.get() {
                            ACTIVE_CLASSES
                        } else {
                            INACTIVE_CLASSES
                        }
                    }
                    on:click=move |_| set_active_tab.set(tab_name_click.clone())
                >
                    {*tab_name}
                </a>
            </li>
        }
    }).collect_view();

    provide_context(active_tab);

    view! {
        <div class="text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:text-gray-400 dark:border-gray-700">
            <ul class="flex flex-wrap -mb-px">{tab_buttons}</ul>
        </div>
        <div>
            {children()}
        </div>
    }
}

#[component]
pub fn Tab(#[prop(into)] name: String, children: Children) -> impl IntoView {
    let active_tab = use_context::<ReadSignal<String>>().expect("Tab must be used within Tabs");

    view! {
        <div style:display=move || if active_tab.get() == name { "block" } else { "none" }>
            {children()}
        </div>
    }
}
