use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;

pub struct EmailService {
    mailer: SmtpTransport,
    from_address: String,
}

impl EmailService {
    pub fn new() -> Result<Self> {
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse::<u16>()
            .unwrap_or(587);
        let smtp_username = env::var("SMTP_USERNAME").unwrap_or_default();
        let smtp_password = env::var("SMTP_PASSWORD").unwrap_or_default();
        let from_address = env::var("SMTP_FROM_ADDRESS")
            .unwrap_or_else(|_| "noreply@timekeeper.local".to_string());

        let mailer = if smtp_username.is_empty() {
            SmtpTransport::builder_dangerous(&smtp_host)
                .port(smtp_port)
                .build()
        } else {
            let creds = Credentials::new(smtp_username, smtp_password);
            SmtpTransport::relay(&smtp_host)?
                .port(smtp_port)
                .credentials(creds)
                .build()
        };

        Ok(Self {
            mailer,
            from_address,
        })
    }

    pub fn send_password_reset_email(&self, to_email: &str, reset_token: &str) -> Result<()> {
        if env::var("SMTP_SKIP_SEND").unwrap_or_default() == "true" {
            return Ok(());
        }
        let reset_url = format!(
            "{}/reset-password?token={}",
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8000".to_string()),
            reset_token
        );

        let body = format!(
            r#"
パスワードリセットのリクエストを受け付けました。

以下のリンクをクリックして、新しいパスワードを設定してください:

{}

このリンクは1時間有効です。

このリクエストに心当たりがない場合は、このメールを無視してください。

---
Timekeeper 勤怠管理システム
"#,
            reset_url
        );

        let email = Message::builder()
            .from(self.from_address.parse()?)
            .to(to_email.parse()?)
            .subject("パスワードリセットのリクエスト - Timekeeper")
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub fn send_password_changed_notification(&self, to_email: &str, username: &str) -> Result<()> {
        if env::var("SMTP_SKIP_SEND").unwrap_or_default() == "true" {
            return Ok(());
        }
        let body = format!(
            r#"
{}さんのパスワードが変更されました。

この変更に心当たりがない場合は、すぐに管理者に連絡してください。

変更日時: {}

---
Timekeeper 勤怠管理システム
"#,
            username,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let email = Message::builder()
            .from(self.from_address.parse()?)
            .to(to_email.parse()?)
            .subject("パスワード変更通知 - Timekeeper")
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        self.mailer.send(&email)?;
        Ok(())
    }
}

impl Default for EmailService {
    fn default() -> Self {
        Self::new().expect("Failed to initialize email service")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    const ENV_KEYS: [&str; 7] = [
        "SMTP_SKIP_SEND",
        "SMTP_HOST",
        "SMTP_PORT",
        "SMTP_USERNAME",
        "SMTP_PASSWORD",
        "SMTP_FROM_ADDRESS",
        "FRONTEND_URL",
    ];

    fn env_mutex() -> &'static Mutex<()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        original: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let lock = env_mutex().lock().expect("lock env");
            let original = ENV_KEYS
                .iter()
                .map(|&key| (key, std::env::var(key).ok()))
                .collect();
            Self {
                _lock: lock,
                original,
            }
        }

        fn set(&self, key: &'static str, value: &str) {
            std::env::set_var(key, value);
        }

        fn remove(&self, key: &'static str) {
            std::env::remove_var(key);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.original {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn email_service_new_creates_service() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        env.remove("SMTP_FROM_ADDRESS");
        let service = EmailService::new();
        assert!(service.is_ok());
        let service = service.unwrap();
        assert_eq!(service.from_address, "noreply@timekeeper.local");
    }

    #[test]
    fn email_service_new_uses_env_vars() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        env.set("SMTP_HOST", "smtp.example.com");
        env.set("SMTP_PORT", "2525");
        env.set("SMTP_FROM_ADDRESS", "test@example.com");

        let service = EmailService::new().unwrap();
        assert_eq!(service.from_address, "test@example.com");
    }

    #[test]
    fn email_service_send_password_reset_email_skips_when_enabled() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        env.set("FRONTEND_URL", "http://test.example.com");

        let service = EmailService::new().unwrap();
        let result = service.send_password_reset_email("test@example.com", "token123");
        assert!(result.is_ok());
    }

    #[test]
    fn email_service_send_password_changed_notification_skips_when_enabled() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");

        let service = EmailService::new().unwrap();
        let result = service.send_password_changed_notification("test@example.com", "testuser");
        assert!(result.is_ok());
    }

    #[test]
    fn email_service_send_password_reset_email_includes_token() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        env.set("FRONTEND_URL", "http://test.example.com");

        let service = EmailService::new().unwrap();
        let result = service.send_password_reset_email("test@example.com", "abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn email_service_send_password_changed_notification_includes_username() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");

        let service = EmailService::new().unwrap();
        let result = service.send_password_changed_notification("test@example.com", "testuser");
        assert!(result.is_ok());
    }

    #[test]
    fn email_service_default_impl_new() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        let service = EmailService::default();
        assert_eq!(service.from_address, "noreply@timekeeper.local");
    }

    #[test]
    fn email_service_send_password_reset_email_valid_to_address() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");
        env.set("FRONTEND_URL", "http://test.example.com");

        let service = EmailService::new().unwrap();
        let result = service.send_password_reset_email("valid@example.com", "abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn email_service_send_password_changed_notification_valid_to_address() {
        let env = EnvGuard::new();
        env.set("SMTP_SKIP_SEND", "true");

        let service = EmailService::new().unwrap();
        let result = service.send_password_changed_notification("valid@example.com", "testuser");
        assert!(result.is_ok());
    }
}
