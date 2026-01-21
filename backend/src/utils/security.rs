use crate::config::Config;
use crate::error::AppError;
use axum::http::HeaderMap;
use rand::Rng;

pub fn generate_token(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn verify_request_origin(headers: &HeaderMap, config: &Config) -> Result<(), AppError> {
    let origin = headers
        .get("Origin")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("Referer").and_then(|v| v.to_str().ok()));

    let origin_str = match origin {
        Some(o) => o,
        None => {
            return Err(AppError::Forbidden(
                "Missing Origin or Referer header".into(),
            ))
        }
    };

    // If config allows specific origins, check against them.
    if config
        .cors_allow_origins
        .iter()
        .any(|o| o == "*" || o == origin_str.trim_end_matches('/'))
    {
        Ok(())
    } else {
        Err(AppError::Forbidden("Invalid Origin or Referer".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;

    fn test_config(allowed: Vec<String>) -> Config {
        Config {
            database_url: "".into(),
            read_database_url: None,
            jwt_secret: "".into(),
            jwt_expiration_hours: 1,
            refresh_token_expiration_days: 1,
            audit_log_retention_days: 1825,
            audit_log_retention_forever: false,
            consent_log_retention_days: 1825,
            consent_log_retention_forever: false,
            aws_region: "ap-northeast-1".into(),
            aws_kms_key_id: "alias/timekeeper-test".into(),
            aws_audit_log_bucket: "timekeeper-audit-logs".into(),
            aws_cloudtrail_enabled: true,
            cookie_secure: false,
            cookie_same_site: crate::utils::cookies::SameSite::Lax,
            cors_allow_origins: allowed,
            time_zone: UTC,
            mfa_issuer: "".into(),
            rate_limit_ip_max_requests: 15,
            rate_limit_ip_window_seconds: 900,
            rate_limit_user_max_requests: 20,
            rate_limit_user_window_seconds: 3600,
            feature_read_replica_enabled: true,
        }
    }

    #[test]
    fn verify_origin_success() {
        let config = test_config(vec!["http://localhost:3000".into()]);
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "http://localhost:3000".parse().unwrap());
        assert!(verify_request_origin(&headers, &config).is_ok());
    }

    #[test]
    fn verify_origin_failure_mismatch() {
        let config = test_config(vec!["http://localhost:3000".into()]);
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "http://evil.com".parse().unwrap());
        assert!(verify_request_origin(&headers, &config).is_err());
    }

    #[test]
    fn verify_origin_failure_missing() {
        let config = test_config(vec!["http://localhost:3000".into()]);
        let headers = HeaderMap::new();
        assert!(verify_request_origin(&headers, &config).is_err());
    }

    #[test]
    fn verify_origin_success_wildcard() {
        let config = test_config(vec!["*".into()]);
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "http://anywhere.com".parse().unwrap());
        assert!(verify_request_origin(&headers, &config).is_ok());
    }

    #[test]
    fn verify_referer_fallback() {
        let config = test_config(vec!["http://localhost:3000".into()]);
        let mut headers = HeaderMap::new();
        headers.insert("Referer", "http://localhost:3000".parse().unwrap());
        assert!(verify_request_origin(&headers, &config).is_ok());
    }
}
