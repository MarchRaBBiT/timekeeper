use super::repository::ForgotPasswordRepository;
use crate::api::{ApiClient, ApiError, MessageResponse};
use leptos::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct ForgotPasswordViewModel {
    pub email: RwSignal<String>,
    pub error: RwSignal<Option<String>>,
    pub success: RwSignal<Option<String>>,
    pub submit_action: Action<String, Result<MessageResponse, ApiError>>,
}

pub fn use_forgot_password_view_model() -> ForgotPasswordViewModel {
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repository = ForgotPasswordRepository::new_with_client(Rc::new(api));

    let email = create_rw_signal(String::new());
    let error = create_rw_signal(None);
    let success = create_rw_signal(None);

    let repo_for_submit = repository.clone();
    let submit_action = create_action(move |value: &String| {
        let repo = repo_for_submit.clone();
        let value = value.clone();
        async move {
            let email = normalize_email(&value)?;
            repo.request_reset(email).await
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

    ForgotPasswordViewModel {
        email,
        error,
        success,
        submit_action,
    }
}

fn normalize_email(value: &str) -> Result<String, ApiError> {
    let email = value.trim();
    if email.is_empty() {
        return Err(ApiError::validation("Email is required"));
    }
    Ok(email.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn normalize_email_rejects_empty() {
        let err = normalize_email("").expect_err("should fail");
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[wasm_bindgen_test]
    fn normalize_email_trims_whitespace() {
        let email = normalize_email("  test@example.com  ").expect("valid");
        assert_eq!(email, "test@example.com");
    }
}
