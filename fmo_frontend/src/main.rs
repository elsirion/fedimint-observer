use fmo_frontend::components::nostr::NostrFederations;
use fmo_frontend::components::{Federation, Federations, NavBar, NavItem};
use leptos::*;
use leptos_meta::{provide_meta_context, Link};
use leptos_router::{Route, Router, Routes};

fn main() {
    // set up logging
    tracing_wasm::set_as_global_default();
    console_error_panic_hook::set_once();
    provide_meta_context();

    mount_to_body(move || {
        view! {
            <Link
                rel="icon"
                type_="image/x-icon"
                href="/fedimint.png"
            />
            <body class="dark:bg-gray-900">
                <Router>
                    <main class="container mx-auto max-w-6xl px-4 min-h-screen pb-4">
                        <NavBar items=vec![
                            NavItem {
                                name: "Home".to_owned(),
                                href: "/".to_owned(),
                                // TODO: make this actually work
                                active: false,
                            },
                            NavItem {
                                name: "Nostr".to_owned(),
                                href: "/nostr".to_owned(),
                                active: false,
                            },
                        ]/>
                        <Routes>
                            <Route path="/" view=|| view! { <Federations/> }/>
                            <Route path="/federations/:id" view=|| view! { <Federation/> }/>
                            <Route path="/nostr" view=|| view! { <NostrFederations/> }/>
                            <Route path="/about" view=|| view! { <div>About</div> }/>
                        </Routes>
                    </main>
                </Router>
            </body>
        }
    })
}
