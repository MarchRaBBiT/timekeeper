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
    fn validate_credentials_rejects_blank_username() {
        let result = validate_credentials("   ", "secret");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "ユーザー名を入力してください");
    }

    #[wasm_bindgen_test]
    fn validate_credentials_requires_password() {
        let result = validate_credentials("alice", "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "パスワードを入力してください");
    }

    #[wasm_bindgen_test]
    fn validate_credentials_accepts_values() {
        assert!(validate_credentials("alice", "secret").is_ok());
    }

    #[wasm_bindgen_test]
    fn normalize_totp_code_trims_and_returns_value() {
        let normalized = normalize_totp_code(" 123456 ");
        assert_eq!(normalized.as_deref(), Some("123456"));
    }

    #[wasm_bindgen_test]
    fn normalize_totp_code_drops_blank() {
        assert!(normalize_totp_code("   ").is_none());
    }
}
