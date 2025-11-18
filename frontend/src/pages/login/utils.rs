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
