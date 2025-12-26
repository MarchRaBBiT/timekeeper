use leptos::*;

#[derive(Clone, Copy, Default)]
pub struct MessageState {
    pub error: RwSignal<Option<String>>,
    pub success: RwSignal<Option<String>>,
}

impl MessageState {
    pub fn set_error(&self, msg: String) {
        self.error.set(Some(msg));
        self.success.set(None);
    }

    pub fn set_success(&self, msg: String) {
        self.success.set(Some(msg));
        self.error.set(None);
    }

    pub fn clear(&self) {
        self.error.set(None);
        self.success.set(None);
    }
}

pub fn validate_totp_code(code: &str) -> Result<String, String> {
    let trimmed = code.trim();
    if trimmed.len() < 6 {
        Err("6桁の確認コードを入力してください".into())
    } else {
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn validate_totp_code_rejects_short_values() {
        let result = validate_totp_code("123");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn validate_totp_code_trims_and_accepts() {
        let result = validate_totp_code(" 987654 ");
        assert_eq!(result.unwrap(), "987654");
    }
}
