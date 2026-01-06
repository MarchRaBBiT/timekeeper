use base64::{engine::general_purpose, Engine as _};
use leptos::*;
use crate::api::ApiError;

#[derive(Clone, Copy, Default)]
pub struct MessageState {
    pub error: RwSignal<Option<ApiError>>,
    pub success: RwSignal<Option<String>>,
}

impl MessageState {
    pub fn set_error(&self, msg: ApiError) {
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

pub fn validate_totp_code(code: &str) -> Result<String, ApiError> {
    let trimmed = code.trim();
    if trimmed.len() < 6 {
        Err(ApiError::validation("6桁の確認コードを入力してください"))
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn svg_to_data_url(svg: &str) -> String {
    let encoded = general_purpose::STANDARD.encode(svg.as_bytes());
    format!("data:image/svg+xml;base64,{}", encoded)
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

    #[wasm_bindgen_test]
    fn svg_to_data_url_encodes_svg() {
        let result = svg_to_data_url("<svg></svg>");
        assert_eq!(result, "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=");
    }

    #[wasm_bindgen_test]
    fn svg_to_data_url_handles_empty_input() {
        let result = svg_to_data_url("");
        assert_eq!(result, "data:image/svg+xml;base64,");
    }
}
