use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::user::User,
    state::AppState,
    utils::{csv::append_csv_row, encryption::decrypt_pii, pii::mask_name, time},
};

use super::common::{parse_date_value, push_clause};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>, // YYYY-MM-DD
    pub to: Option<String>,   // YYYY-MM-DD
}

struct ExportRow {
    username: String,
    full_name: String,
    date: String,
    clock_in: String,
    clock_out: String,
    total_hours: String,
    status: String,
}

pub async fn export_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    // Build filtered SQL
    let parsed_from = match q.from.as_deref() {
        Some(raw) => parse_date_value(raw)
            .ok_or(AppError::BadRequest("`from` must be a valid date".into()))
            .map(Some)?,
        None => None,
    };
    let parsed_to = match q.to.as_deref() {
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

    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.username, u.full_name_enc as full_name, a.date, a.clock_in_time, a.clock_out_time, a.total_work_hours, a.status \
         FROM attendance a JOIN users u ON a.user_id = u.id",
    );
    let mut has_clause = false;
    if let Some(ref u_name) = q.username {
        push_clause(&mut builder, &mut has_clause);
        builder.push("u.username = ").push_bind(u_name);
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
        .fetch_all(state.read_pool())
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mask_pii = !user.is_system_admin();
    let rows: Vec<ExportRow> = data
        .into_iter()
        .map(|record| {
            let username = record
                .try_get::<String, &str>("username")
                .unwrap_or_default();
            let encrypted_full_name = record.try_get::<String, _>("full_name").unwrap_or_default();
            let full_name = decrypt_pii(&encrypted_full_name, &state.config)
                .unwrap_or_else(|_| "***".to_string());
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
                .map(|h| format!("{:.2}", h))
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
        .collect();

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
    Ok((
        headers,
        Json(json!({
            "csv_data": csv_data,
            "filename": format!(
                "attendance_export_{}.csv",
                time::now_in_timezone(&state.config.time_zone).format("%Y%m%d_%H%M%S")
            )
        })),
    ))
}
