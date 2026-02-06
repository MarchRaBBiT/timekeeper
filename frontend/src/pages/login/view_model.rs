use super::utils::LoginFormState;
use crate::api::{ApiError, LoginRequest};
use crate::state::auth;
use leptos::*;

#[derive(Clone)]
pub struct LoginViewModel {
    pub form: LoginFormState,
    pub error: RwSignal<Option<ApiError>>,
    pub login_action: Action<LoginRequest, Result<(), ApiError>>,
}

pub fn use_login_view_model() -> LoginViewModel {
    let form = LoginFormState::default();
    let error = create_rw_signal(None::<ApiError>);
    let login_action = auth::use_login_action();

    let form_copy = form;
    create_effect(move |_| {
        if let Some(result) = login_action.value().get() {
            match result {
                Ok(_) => {
                    error.set(None);
                    form_copy.totp_code.set(String::new());
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/dashboard");
                    }
                }
                Err(err) => error.set(Some(err)),
            }
        }
    });

    LoginViewModel {
        form,
        error,
        login_action,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::with_runtime;

    #[test]
    fn login_view_model_defaults_empty() {
        with_runtime(|| {
            let vm = use_login_view_model();
            assert!(vm.error.get().is_none());
            assert!(vm.form.username.get().is_empty());
        });
    }
}
