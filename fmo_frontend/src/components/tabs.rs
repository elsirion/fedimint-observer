use leptos::leptos_dom::Transparent;
use leptos::{
    component, create_signal, view, Children, ChildrenFn, CollectView, IntoView, SignalGet,
    SignalSet, ToChildren, View,
};
use reqwest::header::WARNING;
use tracing::warn;

#[component]
pub fn Tabs(#[prop(into)] default: String, children: Children) -> impl IntoView {
    let (active_tab, set_active_tab) = create_signal(default);

    let tab_names = children()
        .as_children()
        .iter()
        .filter_map(|child| {
            let Some(tab) = child
                .as_transparent()
                .and_then(Transparent::downcast_ref::<TabView>)
            else {
                warn!(
                    ?child,
                    "Unexpected child in <Tabs>, only <Tab> components are allowed"
                );
                return None;
            };
            Some((tab.name.clone(), tab.children.clone()))
        })
        .collect::<Vec<_>>();

    let tabs = tab_names.iter().map(|(tab_name, _)| {
        let tab_name_c = tab_name.clone();
        let tab_name_a = tab_name.clone();
        const INACTIVE_CLASSES: &str = "inline-block p-4 border-b-2 border-transparent rounded-t-lg hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300";
        const ACTIVE_CLASSES: &str = "inline-block p-4 text-blue-600 border-b-2 border-blue-600 rounded-t-lg active dark:text-blue-500 dark:border-blue-500";

        view! {
            <li class="me-2">
                <a
                    href="#"
                    class=move || { if tab_name_a == active_tab.get() { ACTIVE_CLASSES } else { INACTIVE_CLASSES } }
                    on:click=move |_| set_active_tab.set(tab_name_c.clone())
                >
                    { tab_name }
                </a>
            </li>
        }
    }).collect_view();

    let get_tab_content = move |name: String| {
        tab_names
            .iter()
            .find_map(|(tab_name, children)| {
                if *tab_name == name {
                    Some(children.clone())
                } else {
                    None
                }
            })
            .expect("Tab not found")
    };

    view! {
        <div class="text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:text-gray-400 dark:border-gray-700">
            <ul class="flex flex-wrap -mb-px">
                { tabs }
            </ul>
        </div>
        {move || (get_tab_content(active_tab.get()))()}
    }
}

#[component(transparent)]
pub fn Tab(#[prop(into)] name: String, children: ChildrenFn) -> impl IntoView {
    TabView { name, children }
}

struct TabView {
    name: String,
    children: ChildrenFn,
}

impl IntoView for TabView {
    fn into_view(self) -> View {
        Transparent::new(self).into_view()
    }
}
