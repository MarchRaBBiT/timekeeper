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
    pub fn validate(&self) -> Result<(), String> {
        let username = self.username.get();
        let password = self.password.get();

        if username.trim().is_empty() {
            return Err("ユーザー名を入力してください".into());
        }
        if password.is_empty() {
            return Err("パスワードを入力してください".into());
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

pub fn validate_credentials(username: &str, password: &str) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("ユーザー名を入力してください".into());
    }
    if password.is_empty() {
        return Err("パスワードを入力してください".into());
    }
    Ok(())
}

pub fn normalize_totp_code(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn validate_credentials_tests() {
        assert!(validate_credentials("", "pass").is_err());
        assert!(validate_credentials("user", "").is_err());
        assert!(validate_credentials("user", "pass").is_ok());
    }
}
