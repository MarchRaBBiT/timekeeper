use axum::http::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder, Row};

use crate::{
    application::http::forbidden_error,
    error::AppError,
    models::user::User,
    utils::{csv::append_csv_row, encryption::decrypt_pii, pii::mask_name},
};

use crate::admin::application::common::{parse_date_value, push_clause};

#[derive(Debug, Deserialize, Clone)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub csv_data: String,
    pub filename: String,
}

#[derive(Debug)]
struct ExportRow {
    username: String,
    full_name: String,
    date: String,
    clock_in: String,
    clock_out: String,
    total_hours: String,
    status: String,
}

pub async fn export_attendance_data(
    read_pool: &sqlx::PgPool,
    config: &crate::config::Config,
    user: &User,
    query: ExportQuery,
    filename: String,
) -> Result<(HeaderMap, ExportResponse), AppError> {
    ensure_admin(user)?;

    let (parsed_from, parsed_to) = validate_export_query(&query)?;

    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username, COALESCE(u.full_name_enc, '') as full_name, a.date, a.clock_in_time, a.clock_out_time, a.total_work_hours, a.status \
         FROM attendance a JOIN users u ON a.user_id = u.id",
    );
    let mut has_clause = false;
    if let Some(ref username) = query.username {
        push_clause(&mut builder, &mut has_clause);
        builder.push("u.username = ").push_bind(username);
    }
    if let Some(from) = parsed_from {
        push_clause(&mut builder, &mut has_clause);
        builder.push("a.date >= ").push_bind(from);
    }
    if let Some(to) = parsed_to {
        push_clause(&mut builder, &mut has_clause);
        builder.push("a.date <= ").push_bind(to);
    }
    builder.push(" ORDER BY a.date DESC, u.username");

    let data: Vec<sqlx::postgres::PgRow> = builder
        .build()
        .fetch_all(read_pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    let rows = data
        .into_iter()
        .map(|record| {
            let username = record
                .try_get::<String, &str>("username")
                .unwrap_or_default();
            let encrypted_full_name = record.try_get::<String, _>("full_name").unwrap_or_default();
            let full_name =
                decrypt_pii(&encrypted_full_name, config).unwrap_or_else(|_| "***".to_string());
            let full_name = if mask_pii {
                mask_name(&full_name)
            } else {
                full_name
            };
            let date = record
                .try_get::<chrono::NaiveDate, _>("date")
                .map(|value| value.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let clock_in = record
                .try_get::<Option<chrono::NaiveDateTime>, _>("clock_in_time")
                .ok()
                .flatten()
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default();
            let clock_out = record
                .try_get::<Option<chrono::NaiveDateTime>, _>("clock_out_time")
                .ok()
                .flatten()
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default();
            let total_hours = record
                .try_get::<f64, _>("total_work_hours")
                .map(|h| format!("{h:.2}"))
                .unwrap_or_else(|_| "0.00".to_string());
            let status = record.try_get::<String, _>("status").unwrap_or_default();

            ExportRow {
                username,
                full_name,
                date,
                clock_in,
                clock_out,
                total_hours,
                status,
            }
        })
        .collect::<Vec<_>>();

    let csv_data = tokio::task::spawn_blocking(move || {
        let mut csv = String::new();
        append_csv_row(
            &mut csv,
            &[
                "Username".to_string(),
                "Full Name".to_string(),
                "Date".to_string(),
                "Clock In".to_string(),
                "Clock Out".to_string(),
                "Total Hours".to_string(),
                "Status".to_string(),
            ],
        );

        for row in rows {
            append_csv_row(
                &mut csv,
                &[
                    row.username,
                    row.full_name,
                    row.date,
                    row.clock_in,
                    row.clock_out,
                    row.total_hours,
                    row.status,
                ],
            );
        }
        csv
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-PII-Masked",
        HeaderValue::from_static(if user.is_system_admin() {
            "false"
        } else {
            "true"
        }),
    );

    Ok((headers, ExportResponse { csv_data, filename }))
}

pub fn validate_export_query(
    query: &ExportQuery,
) -> Result<(Option<chrono::NaiveDate>, Option<chrono::NaiveDate>), AppError> {
    let parsed_from = match query.from.as_deref() {
        Some(raw) => parse_date_value(raw)
            .ok_or(AppError::BadRequest("`from` must be a valid date".into()))
            .map(Some)?,
        None => None,
    };
    let parsed_to = match query.to.as_deref() {
        Some(raw) => parse_date_value(raw)
            .ok_or(AppError::BadRequest("`to` must be a valid date".into()))
            .map(Some)?,
        None => None,
    };

    if let (Some(from), Some(to)) = (parsed_from, parsed_to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be on or before `to`".into(),
            ));
        }
    }

    Ok((parsed_from, parsed_to))
}

fn ensure_admin(user: &User) -> Result<(), AppError> {
    if user.is_admin() {
        Ok(())
    } else {
        Err(forbidden_error("Forbidden"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::{User, UserRole};
    use chrono::Utc;

    fn sample_user(is_system_admin: bool) -> User {
        let now = Utc::now();
        User {
            id: crate::types::UserId::new(),
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            is_system_admin,
            mfa_secret: None,
            mfa_enabled_at: None,
            password_changed_at: now,
            failed_login_attempts: 0,
            locked_until: None,
            lock_reason: None,
            lockout_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn validate_export_query_accepts_empty_filters() {
        let result = validate_export_query(&ExportQuery {
            username: None,
            from: None,
            to: None,
        })
        .expect("empty query should validate");
        assert!(result.0.is_none());
        assert!(result.1.is_none());
    }

    #[test]
    fn validate_export_query_rejects_inverted_dates() {
        let result = validate_export_query(&ExportQuery {
            username: None,
            from: Some("2026-03-10".to_string()),
            to: Some("2026-03-09".to_string()),
        });
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn ensure_admin_rejects_non_admin() {
        let mut user = sample_user(false);
        user.role = UserRole::Employee;
        assert!(matches!(ensure_admin(&user), Err(AppError::Forbidden(_))));
    }
}
