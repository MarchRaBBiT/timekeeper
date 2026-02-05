use super::repository::ResetPasswordRepository;
use crate::api::{ApiClient, ApiError, MessageResponse};
use leptos::*;
use leptos_router::use_query_map;
use std::rc::Rc;

#[derive(Clone)]
pub struct ResetPasswordViewModel {
    pub password: RwSignal<String>,
    pub error: RwSignal<Option<String>>,
    pub success: RwSignal<Option<String>>,
    pub submit_action: Action<String, Result<MessageResponse, ApiError>>,
}

pub fn use_reset_password_view_model() -> ResetPasswordViewModel {
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repository = ResetPasswordRepository::new_with_client(Rc::new(api));
    let query = use_query_map();
    let token = Signal::derive(move || query.get().get("token").cloned().unwrap_or_default());

    let password = create_rw_signal(String::new());
    let error = create_rw_signal(None);
    let success = create_rw_signal(None);

    let repo_for_submit = repository.clone();
    let token_for_submit = token.clone();
    let submit_action = create_action(move |value: &String| {
        let repo = repo_for_submit.clone();
        let token = token_for_submit.get();
        let value = value.clone();
        async move {
            let (token, new_password) = validate_reset_input(&token, &value)?;
            repo.reset_password(token, new_password).await
        }
    });

    create_effect(move |_| {
        if let Some(result) = submit_action.value().get() {
            apply_submit_result(&success, &error, result);
        }
    });

    ResetPasswordViewModel {
        password,
        error,
        success,
        submit_action,
    }
}

fn validate_reset_input(token: &str, new_password: &str) -> Result<(String, String), ApiError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(ApiError::validation("Invalid token"));
    }
    let new_password = new_password.trim();
    if new_password.is_empty() {
        return Err(ApiError::validation("Password is required"));
    }
    Ok((token.to_string(), new_password.to_string()))
}

fn apply_submit_result(
    success: &RwSignal<Option<String>>,
    error: &RwSignal<Option<String>>,
    result: Result<MessageResponse, ApiError>,
) {
    match result {
        Ok(resp) => {
            success.set(Some(resp.message));
            error.set(None);
        }
        Err(err) => {
            error.set(Some(err.to_string()));
            success.set(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_submit_result, *};
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn validate_reset_input_rejects_empty_token() {
        let err = validate_reset_input("", "password").expect_err("should fail");
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[wasm_bindgen_test]
    fn validate_reset_input_rejects_empty_password() {
        let err = validate_reset_input("token", " ").expect_err("should fail");
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[wasm_bindgen_test]
    fn validate_reset_input_trims_values() {
        let (token, password) =
            validate_reset_input("  token  ", "  NewPass123!  ").expect("valid");
        assert_eq!(token, "token");
        assert_eq!(password, "NewPass123!");
    }

    #[wasm_bindgen_test]
    fn apply_submit_result_sets_success_or_error() {
        let success = create_rw_signal(None::<String>);
        let error = create_rw_signal(None::<String>);

        apply_submit_result(
            &success,
            &error,
            Ok(MessageResponse {
                message: "updated".into(),
            }),
        );
        assert_eq!(success.get(), Some("updated".into()));
        assert!(error.get().is_none());

        apply_submit_result(&success, &error, Err(ApiError::validation("invalid token")));
        assert!(success.get().is_none());
        assert!(error.get().unwrap_or_default().contains("invalid token"));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::{apply_submit_result, *};
    use crate::test_support::ssr::with_runtime;

    #[test]
    fn validate_reset_input_rejects_empty_on_host() {
        with_runtime(|| {
            let err = validate_reset_input("", "password").expect_err("should fail");
            assert_eq!(err.code, "VALIDATION_ERROR");
        });
    }

    #[test]
    fn validate_reset_input_host_covers_trim_and_empty_password() {
        with_runtime(|| {
            let err = validate_reset_input("token", "   ").expect_err("should fail");
            assert_eq!(err.code, "VALIDATION_ERROR");

            let (token, password) =
                validate_reset_input("  token ", " pass1234 ").expect("valid input");
            assert_eq!(token, "token");
            assert_eq!(password, "pass1234");
        });
    }

    #[test]
    fn apply_submit_result_updates_signals() {
        with_runtime(|| {
            let success = create_rw_signal(None::<String>);
            let error = create_rw_signal(None::<String>);

            apply_submit_result(
                &success,
                &error,
                Ok(MessageResponse {
                    message: "パスワードを変更しました".into(),
                }),
            );
            assert_eq!(success.get(), Some("パスワードを変更しました".into()));
            assert!(error.get().is_none());

            apply_submit_result(&success, &error, Err(ApiError::validation("Invalid token")));
            assert!(success.get().is_none());
            assert!(error.get().unwrap_or_default().contains("Invalid token"));
        });
    }
}
