use leptos::prelude::*;

#[component]
pub fn Rating(count: u64, rating: Option<f64>) -> impl IntoView {
    view! {
        <div class="flex justify-center">
            <div>
                <div class="flex items-center">
                    <svg class="w-4 h-4 text-yellow-300 me-1" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 22 20">
                        <path d="M20.924 7.625a1.523 1.523 0 0 0-1.238-1.044l-5.051-.734-2.259-4.577a1.534 1.534 0 0 0-2.752 0L7.365 5.847l-5.051.734A1.535 1.535 0 0 0 1.463 9.2l3.656 3.563-.863 5.031a1.532 1.532 0 0 0 2.226 1.616L11 17.033l4.518 2.375a1.534 1.534 0 0 0 2.226-1.617l-.863-5.03L20.537 9.2a1.523 1.523 0 0 0 .387-1.575Z"/>
                    </svg>
                    <p class="ms-2 text-sm font-bold text-gray-900 dark:text-white">
                        {
                            match rating {
                                Some(rating) => format!("{:.1}", rating),
                                None => "N/A".to_string()
                            }
                        }
                    </p>
                </div>
                <span class="text-sm font-medium text-gray-900 dark:text-white">{count} " votes"</span>
            </div>
        </div>
    }
}
