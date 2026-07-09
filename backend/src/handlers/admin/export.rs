use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::{Datelike, Duration, Months, NaiveDate};
use serde::Deserialize;
use serde_json::json;
use timekeeper_app::attendance::{
    ExportAdminAttendance as ExportAdminAttendanceUseCase,
    ExportAdminAttendanceError as AppExportAdminAttendanceError,
    ExportAdminAttendanceQuery as AppExportAdminAttendanceQuery, MAX_ADMIN_EXPORT_RANGE_DAYS,
};
use timekeeper_infra_postgres::attendance::AttendanceWorkflowRepository;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::user::User,
    state::AppState,
    utils::{
        csv::{render_admin_attendance_export_csv, AdminAttendanceExportCsvRow},
        encryption::decrypt_pii,
        pii::mask_name,
        time,
    },
};

use super::common::parse_date_value;

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ExportQuery {
    pub username: Option<String>,
    pub from: Option<String>, // YYYY-MM-DD
    pub to: Option<String>,   // YYYY-MM-DD
}

pub async fn export_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    if !(user.is_manager() || user.is_system_admin()) {
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

    let today = time::now_in_timezone(&state.config.time_zone).date_naive();
    // `from`/`to` drive both the plain attendance-row query and the
    // approved-leave-days query (via generate_series). Resolving a bounded
    // default here, before either query runs, keeps both bounded uniformly.
    let (from, to) = resolve_export_date_range(parsed_from, parsed_to, today)?;

    let use_case = ExportAdminAttendanceUseCase::new(AttendanceWorkflowRepository::new(
        state.read_pool().clone(),
    ));
    let export = use_case
        .execute(AppExportAdminAttendanceQuery {
            requester_id: user.id.to_string(),
            requester_is_manager: user.is_manager(),
            requester_is_system_admin: user.is_system_admin(),
            username: q.username,
            from: Some(from),
            to: Some(to),
        })
        .await
        .map_err(export_admin_attendance_error_to_app_error)?;
    let csv_rows = export
        .rows
        .into_iter()
        .map(|row| {
            let full_name = decrypt_pii(&row.full_name_encrypted, &state.config)
                .unwrap_or_else(|_| "***".to_string());
            let full_name = if export.pii_masked {
                mask_name(&full_name)
            } else {
                full_name
            };
            AdminAttendanceExportCsvRow {
                username: row.username,
                full_name,
                date: row.date.format("%Y-%m-%d").to_string(),
                clock_in: row
                    .clock_in_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                clock_out: row
                    .clock_out_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                total_hours: row
                    .total_work_hours
                    .map(|hours| format!("{hours:.2}"))
                    .unwrap_or_else(|| "0.00".to_string()),
                status: row.status,
                leave_type: row.leave_type,
            }
        })
        .collect::<Vec<_>>();
    let csv_data = render_admin_attendance_export_csv(&csv_rows);

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-PII-Masked",
        HeaderValue::from_static(if export.pii_masked { "true" } else { "false" }),
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

fn export_admin_attendance_error_to_app_error(error: AppExportAdminAttendanceError) -> AppError {
    match error {
        AppExportAdminAttendanceError::Forbidden => AppError::Forbidden("Forbidden".into()),
        AppExportAdminAttendanceError::DateRangeTooLarge { max_days } => {
            AppError::BadRequest(format!("date range must not exceed {max_days} days"))
        }
        AppExportAdminAttendanceError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("first day of an existing month is always valid")
}

fn month_end(date: NaiveDate) -> Result<NaiveDate, AppError> {
    month_start(date)
        .checked_add_months(Months::new(1))
        .and_then(|next_month_start| next_month_start.checked_sub_signed(Duration::days(1)))
        .ok_or(AppError::BadRequest("invalid date range".into()))
}

/// Resolves the effective `from`/`to` export window and rejects windows wider
/// than `MAX_ADMIN_EXPORT_RANGE_DAYS`.
///
/// Defaulting rules when one or both bounds are omitted (M-3):
/// - both omitted: current month (`today`'s month, start to end)
/// - `from` only: `to` defaults to the end of `from`'s month
/// - `to` only: `from` defaults to the start of `to`'s month
///
/// Anchoring a missing bound to the *given* bound's month (rather than to
/// today's real calendar month) keeps the resolved range self-consistent
/// for historical queries, e.g. `to=2024-03-15` without `from` yields the
/// whole of March 2024 rather than an unrelated (and likely empty) range
/// anchored on the current month.
fn resolve_export_date_range(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    today: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    let (from, to) = match (from, to) {
        (Some(from), Some(to)) => (from, to),
        (Some(from), None) => (from, month_end(from)?),
        (None, Some(to)) => (month_start(to), to),
        (None, None) => (month_start(today), month_end(today)?),
    };

    if from > to {
        return Err(AppError::BadRequest(
            "`from` must be on or before `to`".into(),
        ));
    }

    let span_days = (to - from).num_days() + 1;
    if span_days > MAX_ADMIN_EXPORT_RANGE_DAYS {
        return Err(AppError::BadRequest(format!(
            "date range must not exceed {MAX_ADMIN_EXPORT_RANGE_DAYS} days"
        )));
    }

    Ok((from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    #[test]
    fn defaults_to_current_month_when_both_omitted() {
        let today = date(2026, 7, 9);
        let (from, to) = resolve_export_date_range(None, None, today).expect("resolves");
        assert_eq!(from, date(2026, 7, 1));
        assert_eq!(to, date(2026, 7, 31));
    }

    #[test]
    fn defaults_to_from_month_end_when_only_from_given() {
        let today = date(2026, 7, 9);
        let (from, to) =
            resolve_export_date_range(Some(date(2026, 2, 10)), None, today).expect("resolves");
        assert_eq!(from, date(2026, 2, 10));
        assert_eq!(to, date(2026, 2, 28));
    }

    #[test]
    fn defaults_to_to_month_start_when_only_to_given() {
        let today = date(2026, 7, 9);
        let (from, to) =
            resolve_export_date_range(None, Some(date(2026, 3, 15)), today).expect("resolves");
        assert_eq!(from, date(2026, 3, 1));
        assert_eq!(to, date(2026, 3, 15));
    }

    #[test]
    fn keeps_explicit_range_unchanged() {
        let today = date(2026, 7, 9);
        let (from, to) =
            resolve_export_date_range(Some(date(2025, 1, 1)), Some(date(2025, 6, 30)), today)
                .expect("resolves");
        assert_eq!(from, date(2025, 1, 1));
        assert_eq!(to, date(2025, 6, 30));
    }

    #[test]
    fn rejects_span_exceeding_max_days() {
        let today = date(2026, 7, 9);
        let result =
            resolve_export_date_range(Some(date(2025, 1, 1)), Some(date(2026, 1, 2)), today);
        assert!(result.is_err());
    }

    #[test]
    fn allows_span_exactly_at_max_days() {
        let today = date(2026, 7, 9);
        let from = date(2025, 1, 1);
        let to = from + Duration::days(MAX_ADMIN_EXPORT_RANGE_DAYS - 1);
        let (resolved_from, resolved_to) =
            resolve_export_date_range(Some(from), Some(to), today).expect("resolves");
        assert_eq!(resolved_from, from);
        assert_eq!(resolved_to, to);
    }
}
