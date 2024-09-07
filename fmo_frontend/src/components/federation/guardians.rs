use fedimint_core::{NumPeers, NumPeersExt};
use leptos::{component, view, IntoView};

#[component]
pub fn Guardians(guardians: Vec<Guardian>) -> impl IntoView {
    let n = guardians.len();
    let t = NumPeers::from(n).threshold();

    let guardians = guardians
        .into_iter()
        .map(|guardian| {
            view! {
                <li class="py-3 sm:py-4">
                    <div class="flex items-center">
                        <div class="flex-1 min-w-0 ms-4">
                            <p class="text-sm font-medium text-gray-900 truncate dark:text-white">
                                {guardian.name}
                            </p>
                            <p class="text-sm text-gray-500 truncate dark:text-gray-400">
                                {guardian.url}
                            </p>
                        </div>
                    </div>
                </li>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div class="w-full h-full p-4 bg-white border border-gray-200 rounded-lg shadow sm:p-8 dark:bg-gray-800 dark:border-gray-700">
            <div class="flex items-center justify-between mb-4">
                <h5 class="text-xl font-bold leading-none text-gray-900 dark:text-white">
                    Guardians
                </h5>
                <p class="text-sm font-medium text-gray-500 dark:text-gray-400">
                    {format!("{} of {} Federation", t, n)}
                </p>
            </div>
            <div class="flow-root">
                <ul role="list" class="divide-y divide-gray-200 dark:divide-gray-700">
                    {guardians}
                </ul>
            </div>
        </div>
    }
}

pub struct Guardian {
    pub name: String,
    pub url: String,
}
