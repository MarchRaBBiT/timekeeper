use leptos::*;

pub mod components;
pub mod repository;
pub mod utils;
pub mod view_model;

mod panel;

pub use panel::LoginPanel;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! { <LoginPanel /> }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;
    use leptos_router::{Router, RouterIntegrationContext, ServerIntegration};

    #[test]
    fn login_page_renders_panel() {
        let html = render_to_string(move || {
            provide_context(RouterIntegrationContext::new(ServerIntegration {
                path: "http://localhost/".to_string(),
            }));
            view! {
                <Router>
                    <LoginPage />
                </Router>
            }
        });
        assert!(html.contains("Timekeeper にログイン"));
    }
}
