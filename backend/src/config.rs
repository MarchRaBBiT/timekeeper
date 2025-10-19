use anyhow::anyhow;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub refresh_token_expiration_days: u64,
    pub time_zone: Tz,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./timekeeper.db".to_string());

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-this-in-production".to_string());

        let jwt_expiration_hours = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let refresh_token_expiration_days = env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse()
            .unwrap_or(7);

        let time_zone_name = env::var("APP_TIMEZONE").unwrap_or_else(|_| "UTC".to_string());
        let time_zone: Tz = time_zone_name
            .parse()
            .map_err(|_| anyhow!("Invalid APP_TIMEZONE value: {}", time_zone_name))?;

        Ok(Config {
            database_url,
            jwt_secret,
            jwt_expiration_hours,
            refresh_token_expiration_days,
            time_zone,
        })
    }
}
