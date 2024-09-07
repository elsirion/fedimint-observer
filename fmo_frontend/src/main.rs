use fmo_frontend::components::{Federation, Federations, NavBar, NavItem};
use leptos::*;
use leptos_router::{Route, Router, Routes};

fn main() {
    // set up logging
    tracing_wasm::set_as_global_default();
    console_error_panic_hook::set_once();

    mount_to_body(move || {
        view! {
            <body class="dark:bg-gray-900">
                <Router>
                    <main class="container mx-auto max-w-6xl px-4 min-h-screen">
                        <NavBar items=vec![
                            NavItem {
                                name: "Home".to_owned(),
                                href: "/".to_owned(),
                                active: true,
                            },
                            NavItem {
                                name: "About".to_owned(),
                                href: "/about".to_owned(),
                                active: false,
                            },
                        ]/>
                        <Routes>
                            <Route path="/" view=|| view! { <Federations/> }/>
                            <Route path="/federations/:id" view=|| view! { <Federation/> }/>
                            <Route path="/about" view=|| view! { <div>About</div> }/>
                        </Routes>
                    </main>
                </Router>
            </body>
        }
    })
}
