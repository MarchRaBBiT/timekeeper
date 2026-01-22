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

#[cfg(test)]
mod tests {
    use super::*;
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
}
