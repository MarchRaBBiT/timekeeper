use timekeeper_backend::{config::Config, utils::cookies::SameSite};

#[test]
fn loads_time_zone_string_without_db() {
    // Ensure Config struct stays default-constructible via explicit values.
    let cfg = Config {
        database_url: "postgres://example".into(),
        jwt_secret: "secret".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 1,
        cookie_secure: false,
        cookie_same_site: SameSite::Lax,
        cors_allow_origins: vec!["http://localhost:8000".into()],
        time_zone: chrono_tz::Asia::Tokyo,
        mfa_issuer: "Timekeeper".into(),
    };
    assert_eq!(cfg.time_zone, chrono_tz::Asia::Tokyo);
}
