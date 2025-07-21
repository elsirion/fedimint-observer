use fedimint_core::config::JsonClientConfig;
use leptos::prelude::*;

#[component]
pub fn General(config: JsonClientConfig) -> impl IntoView {
    let module_badges = get_modules(&config).into_iter().map(|module| {
        view! {
            <span class="bg-blue-100 text-blue-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-blue-900 dark:text-blue-300 inline">
                {module}
            </span>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="w-full p-4 bg-white border border-gray-200 rounded-lg shadow sm:p-8 dark:bg-gray-800 dark:border-gray-700">
            <div class="flex items-center justify-between mb-4">
                <h5 class="text-xl font-bold leading-none text-gray-900 dark:text-white">
                    Federation
                </h5>
            </div>
            <div class="flow-root">
                <div class="relative overflow-x-auto">
                    <table class="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                        <tbody>
                            <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                <th
                                    scope="row"
                                    class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                >
                                    Network
                                </th>
                                <td class="px-6 py-4">{get_network(&config)}</td>
                            </tr>
                            <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                                <th
                                    scope="row"
                                    class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                >
                                    Modules
                                </th>
                                <td class="px-6 py-4 whitespace-normal">{module_badges}</td>
                            </tr>
                            <tr class="bg-white dark:bg-gray-800">
                                <th
                                    scope="row"
                                    class="px-6 py-4 font-medium text-gray-900 dark:text-white"
                                >
                                    Confirmations
                                    <br/>
                                    Required
                                </th>
                                <td class="px-6 py-4">{get_confirmations_required(&config)}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}

fn get_network(cfg: &JsonClientConfig) -> String {
    // TODO: don't assume so much
    cfg.modules
        .iter()
        .find_map(|(_, m)| {
            if m.kind().as_str() != "wallet" {
                return None;
            }

            Some(
                m.value()["network"]
                    .as_str()
                    .expect("Network is of type string")
                    .to_owned(),
            )
        })
        .expect("Wallet module is expected to be present")
}

fn get_modules(cfg: &JsonClientConfig) -> Vec<String> {
    cfg.modules
        .values()
        .map(|m| m.kind().as_str().to_owned())
        .collect()
}

fn get_confirmations_required(cfg: &JsonClientConfig) -> u64 {
    // TODO: don't assume so much
    cfg.modules
        .iter()
        .find_map(|(_, m)| {
            if m.kind().as_str() != "wallet" {
                return None;
            }

            Some(
                m.value()["finality_delay"]
                    .as_u64()
                    .expect("finality_delay is of type integer")
                    + 1,
            )
        })
        .expect("Wallet module is expected to be present")
}
