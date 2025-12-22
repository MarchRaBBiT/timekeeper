use anyhow::anyhow;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::env;

use crate::utils::cookies::SameSite;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub refresh_token_expiration_days: u64,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    pub cors_allow_origins: Vec<String>,
    pub time_zone: Tz,
    pub mfa_issuer: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://timekeeper:timekeeper@localhost:5432/timekeeper".to_string()
        });

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow!("JWT_SECRET must be set and at least 32 characters long"))?;
        if jwt_secret.len() < 32 {
            return Err(anyhow!(
                "JWT_SECRET must be at least 32 characters long (current length: {})",
                jwt_secret.len()
            ));
        }

        let jwt_expiration_hours = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let refresh_token_expiration_days = env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse()
            .unwrap_or(7);

        let cookie_secure = env::var("COOKIE_SECURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let cookie_same_site =
            parse_same_site(env::var("COOKIE_SAMESITE").unwrap_or_else(|_| "Lax".to_string()))?;

        let cors_allow_origins = env::var("CORS_ALLOW_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:8000".to_string())
            .split(',')
            .map(|origin| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect::<Vec<_>>();

        let time_zone_name = env::var("APP_TIMEZONE").unwrap_or_else(|_| "UTC".to_string());
        let time_zone: Tz = time_zone_name
            .parse()
            .map_err(|_| anyhow!("Invalid APP_TIMEZONE value: {}", time_zone_name))?;

        let mfa_issuer = env::var("MFA_ISSUER").unwrap_or_else(|_| "Timekeeper".to_string());

        Ok(Config {
            database_url,
            jwt_secret,
            jwt_expiration_hours,
            refresh_token_expiration_days,
            cookie_secure,
            cookie_same_site,
            cors_allow_origins,
            time_zone,
            mfa_issuer,
        })
    }
}

fn parse_same_site(raw: String) -> anyhow::Result<SameSite> {
    match raw.to_ascii_lowercase().as_str() {
        "lax" => Ok(SameSite::Lax),
        "strict" => Ok(SameSite::Strict),
        "none" => Ok(SameSite::None),
        _ => Err(anyhow!("Invalid COOKIE_SAMESITE value: {}", raw)),
    }
}
