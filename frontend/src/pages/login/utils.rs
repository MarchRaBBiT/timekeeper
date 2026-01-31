use crate::api::ApiError;
use leptos::*;

#[derive(Clone, Copy)]
pub struct LoginFormState {
    pub username: RwSignal<String>,
    pub password: RwSignal<String>,
    pub totp_code: RwSignal<String>,
}

impl Default for LoginFormState {
    fn default() -> Self {
        Self {
            username: create_rw_signal(String::new()),
            password: create_rw_signal(String::new()),
            totp_code: create_rw_signal(String::new()),
        }
    }
}

impl LoginFormState {
    pub fn validate(&self) -> Result<(), ApiError> {
        let username = self.username.get();
        let password = self.password.get();

        if username.trim().is_empty() {
            return Err(ApiError::validation("ユーザー名を入力してください"));
        }
        if password.is_empty() {
            return Err(ApiError::validation("パスワードを入力してください"));
        }
        Ok(())
    }

    pub fn normalize_totp(&self) -> Option<String> {
        let raw = self.totp_code.get();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::create_runtime;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn login_form_validation_requires_values() {
        with_runtime(|| {
            let state = LoginFormState::default();
            assert!(state.validate().is_err());
            state.username.set("user".into());
            assert!(state.validate().is_err());
            state.password.set("pass".into());
            assert!(state.validate().is_ok());
        });
    }

    #[test]
    fn normalize_totp_trims_value() {
        with_runtime(|| {
            let state = LoginFormState::default();
            state.totp_code.set(" 123456 ".into());
            assert_eq!(state.normalize_totp().as_deref(), Some("123456"));
            state.totp_code.set("   ".into());
            assert!(state.normalize_totp().is_none());
        });
    }
}
