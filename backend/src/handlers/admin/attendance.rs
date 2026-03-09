use crate::error::AppError;
use crate::models::{PaginatedResponse, PaginationQuery};
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    admin::application::attendance as application,
    models::{attendance::AttendanceResponse, break_record::BreakRecordResponse, user::User},
    state::AppState,
};

pub async fn get_all_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<AttendanceResponse>>, AppError> {
    Ok(Json(
        application::get_all_attendance(
            state.read_pool(),
            &user,
            pagination.limit(),
            pagination.offset(),
        )
        .await?,
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct AdminAttendanceUpsert {
    pub user_id: String,
    pub date: String,
    pub clock_in_time: String,
    pub clock_out_time: Option<String>,
    pub breaks: Option<Vec<AdminBreakItem>>,
}

#[derive(Deserialize, ToSchema)]
pub struct AdminBreakItem {
    pub break_start_time: String,
    pub break_end_time: Option<String>,
}

pub async fn upsert_attendance(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<AdminAttendanceUpsert>,
) -> Result<Json<AttendanceResponse>, AppError> {
    Ok(Json(
        application::upsert_attendance(
            &state.write_pool,
            &state.config.time_zone,
            &user,
            application::AdminAttendanceUpsertInput {
                user_id: body.user_id,
                date: body.date,
                clock_in_time: body.clock_in_time,
                clock_out_time: body.clock_out_time,
                breaks: body.breaks.map(|items| {
                    items
                        .into_iter()
                        .map(|item| application::AdminBreakItemInput {
                            break_start_time: item.break_start_time,
                            break_end_time: item.break_end_time,
                        })
                        .collect()
                }),
            },
        )
        .await?,
    ))
}

pub async fn force_end_break(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(break_id): Path<String>,
) -> Result<Json<BreakRecordResponse>, AppError> {
    Ok(Json(
        application::force_end_break(&state.write_pool, &state.config.time_zone, &user, &break_id)
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_attendance_upsert_structure_holds_values() {
        let body = AdminAttendanceUpsert {
            user_id: "user-1".to_string(),
            date: "2026-03-09".to_string(),
            clock_in_time: "2026-03-09T09:00:00".to_string(),
            clock_out_time: Some("2026-03-09T18:00:00".to_string()),
            breaks: Some(vec![AdminBreakItem {
                break_start_time: "2026-03-09T12:00:00".to_string(),
                break_end_time: Some("2026-03-09T13:00:00".to_string()),
            }]),
        };

        assert_eq!(body.user_id, "user-1");
        assert_eq!(body.date, "2026-03-09");
        assert_eq!(body.breaks.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn admin_break_item_allows_open_breaks() {
        let item = AdminBreakItem {
            break_start_time: "2026-03-09T12:00:00".to_string(),
            break_end_time: None,
        };

        assert_eq!(item.break_start_time, "2026-03-09T12:00:00");
        assert!(item.break_end_time.is_none());
    }
}
