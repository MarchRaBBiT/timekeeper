use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SameSite {
    Lax,
    Strict,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct CookieOptions {
    pub secure: bool,
    pub same_site: SameSite,
}

pub const ACCESS_COOKIE_NAME: &str = "access_token";
pub const REFRESH_COOKIE_NAME: &str = "refresh_token";
pub const ACCESS_COOKIE_PATH: &str = "/";
pub const REFRESH_COOKIE_PATH: &str = "/api/auth";

pub fn build_auth_cookie(
    name: &str,
    value: &str,
    max_age: Duration,
    path: &str,
    options: CookieOptions,
) -> String {
    let mut cookie = format!(
        "{}={}; Path={}; Max-Age={}; HttpOnly; SameSite={}",
        name,
        value,
        path,
        max_age.as_secs(),
        same_site_value(options.same_site)
    );
    if options.secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn build_clear_cookie(name: &str, path: &str, options: CookieOptions) -> String {
    let mut cookie = format!(
        "{}=; Path={}; Max-Age=0; HttpOnly; SameSite={}",
        name,
        path,
        same_site_value(options.same_site)
    );
    if options.secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn extract_cookie_value(header: &str, name: &str) -> Option<String> {
    header.split(';').map(str::trim).find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn same_site_value(same_site: SameSite) -> &'static str {
    match same_site {
        SameSite::Lax => "Lax",
        SameSite::Strict => "Strict",
        SameSite::None => "None",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_auth_cookie_includes_security_attributes() {
        let opts = CookieOptions {
            secure: true,
            same_site: SameSite::Lax,
        };
        let cookie = build_auth_cookie("access_token", "abc", Duration::from_secs(3600), "/", opts);
        assert!(cookie.contains("access_token=abc"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=3600"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn build_clear_cookie_sets_max_age_zero() {
        let opts = CookieOptions {
            secure: false,
            same_site: SameSite::Strict,
        };
        let cookie = build_clear_cookie("refresh_token", "/api/auth", opts);
        assert!(cookie.contains("refresh_token="));
        assert!(cookie.contains("Path=/api/auth"));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn extract_cookie_value_finds_matching_name() {
        let header = "a=1; access_token=token-value; b=2";
        assert_eq!(
            extract_cookie_value(header, "access_token").as_deref(),
            Some("token-value")
        );
        assert!(extract_cookie_value(header, "missing").is_none());
    }
}
