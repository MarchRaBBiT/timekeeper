use crate::pages::login::components::form::LoginForm;
use leptos::*;
#[component]
pub fn LoginPanel() -> impl IntoView {
    let vm = crate::pages::login::view_model::use_login_view_model();

    view! {
        <LoginForm
            form=vm.form
            error=vm.error
            login_action=vm.login_action
        />
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;
    use leptos_router::{Router, RouterIntegrationContext, ServerIntegration};

    #[test]
    fn login_panel_renders_form() {
        let html = render_to_string(move || {
            provide_context(RouterIntegrationContext::new(ServerIntegration {
                path: "http://localhost/".to_string(),
            }));
            view! {
                <Router>
                    <LoginPanel />
                </Router>
            }
        });
        assert!(html.contains("Timekeeper にログイン"));
        assert!(html.contains("Forgot your password?"));
    }
}
