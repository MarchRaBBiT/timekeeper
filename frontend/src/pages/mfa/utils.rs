pub fn validate_totp_code(code: &str) -> Result<String, String> {
    let trimmed = code.trim();
    if trimmed.len() < 6 {
        Err("6桁の確認コードを入力してください".into())
    } else {
        Ok(trimmed.to_string())
    }
}
