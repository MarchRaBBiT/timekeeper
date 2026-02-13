use crate::error::AppError;
use crate::models::attendance_correction_request::{
    AttendanceCorrectionEffectiveValue, AttendanceCorrectionRequest, AttendanceCorrectionSnapshot,
    CorrectionBreakItem,
};
use crate::types::{AttendanceId, UserId};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Default, Clone, Copy)]
pub struct AttendanceCorrectionRequestRepository;

impl AttendanceCorrectionRequestRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(
        &self,
        db: &PgPool,
        id: &str,
        user_id: UserId,
        attendance_id: AttendanceId,
        date: chrono::NaiveDate,
        reason: &str,
        original_snapshot: &AttendanceCorrectionSnapshot,
        proposed_values: &AttendanceCorrectionSnapshot,
    ) -> Result<AttendanceCorrectionRequest, AppError> {
        let now = Utc::now();
        let query = "INSERT INTO attendance_correction_requests (
                id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8,$9)
            RETURNING id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at";

        let row = sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(id)
            .bind(user_id)
            .bind(attendance_id)
            .bind(date)
            .bind(reason)
            .bind(
                serde_json::to_value(original_snapshot)
                    .map_err(|e| AppError::InternalServerError(e.into()))?,
            )
            .bind(
                serde_json::to_value(proposed_values)
                    .map_err(|e| AppError::InternalServerError(e.into()))?,
            )
            .bind(now)
            .bind(now)
            .fetch_one(db)
            .await?;
        Ok(row)
    }

    pub async fn list_by_user(
        &self,
        db: &PgPool,
        user_id: UserId,
    ) -> Result<Vec<AttendanceCorrectionRequest>, AppError> {
        let query = "SELECT id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at
            FROM attendance_correction_requests
            WHERE user_id = $1
            ORDER BY created_at DESC";

        Ok(sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(user_id)
            .fetch_all(db)
            .await?)
    }

    pub async fn list_paginated(
        &self,
        db: &PgPool,
        status: Option<&str>,
        user_id: Option<UserId>,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<AttendanceCorrectionRequest>, AppError> {
        let offset = (page - 1).max(0) * per_page;
        let query = "SELECT id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at
            FROM attendance_correction_requests
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::uuid IS NULL OR user_id = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4";

        Ok(sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(status)
            .bind(user_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(db)
            .await?)
    }

    pub async fn find_by_id(
        &self,
        db: &PgPool,
        id: &str,
    ) -> Result<AttendanceCorrectionRequest, AppError> {
        let query = "SELECT id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at
            FROM attendance_correction_requests
            WHERE id = $1";

        sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Attendance correction request not found".into()))
    }

    pub async fn find_by_id_for_user(
        &self,
        db: &PgPool,
        id: &str,
        user_id: UserId,
    ) -> Result<AttendanceCorrectionRequest, AppError> {
        let query = "SELECT id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at
            FROM attendance_correction_requests
            WHERE id = $1 AND user_id = $2";

        sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Attendance correction request not found".into()))
    }

    pub async fn update_pending_for_user(
        &self,
        db: &PgPool,
        id: &str,
        user_id: UserId,
        reason: &str,
        proposed_values: &AttendanceCorrectionSnapshot,
    ) -> Result<AttendanceCorrectionRequest, AppError> {
        let now = Utc::now();
        let query = "UPDATE attendance_correction_requests
            SET reason = $1,
                proposed_values_json = $2,
                updated_at = $3
            WHERE id = $4 AND user_id = $5 AND status = 'pending'
            RETURNING id, user_id, attendance_id, date, status, reason,
                original_snapshot_json, proposed_values_json, decision_comment,
                approved_by, approved_at, rejected_by, rejected_at, cancelled_at,
                created_at, updated_at";

        sqlx::query_as::<_, AttendanceCorrectionRequest>(query)
            .bind(reason)
            .bind(
                serde_json::to_value(proposed_values)
                    .map_err(|e| AppError::InternalServerError(e.into()))?,
            )
            .bind(now)
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::Conflict("Only pending requests can be updated".into()))
    }

    pub async fn cancel_pending_for_user(
        &self,
        db: &PgPool,
        id: &str,
        user_id: UserId,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let query = "UPDATE attendance_correction_requests
            SET status = 'cancelled', cancelled_at = $1, updated_at = $1
            WHERE id = $2 AND user_id = $3 AND status = 'pending'";

        let affected = sqlx::query(query)
            .bind(now)
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::Conflict(
                "Only pending requests can be cancelled".into(),
            ));
        }
        Ok(())
    }

    pub async fn reject(
        &self,
        db: &PgPool,
        id: &str,
        approver_id: UserId,
        comment: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let query = "UPDATE attendance_correction_requests
            SET status = 'rejected', rejected_by = $1, rejected_at = $2,
                decision_comment = $3, updated_at = $2
            WHERE id = $4 AND status = 'pending'";

        let affected = sqlx::query(query)
            .bind(approver_id)
            .bind(now)
            .bind(comment)
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::Conflict(
                "Request not found or already processed".into(),
            ));
        }
        Ok(())
    }

    pub async fn approve_and_apply_effective_values(
        &self,
        db: &PgPool,
        id: &str,
        attendance_id: AttendanceId,
        approver_id: UserId,
        comment: &str,
        original_snapshot: &AttendanceCorrectionSnapshot,
        proposed_values: &AttendanceCorrectionSnapshot,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let breaks_json: Value = serde_json::to_value(&proposed_values.breaks)
            .map_err(|e| AppError::InternalServerError(e.into()))?;

        let mut tx = db.begin().await?;
        let latest_attendance =
            sqlx::query_as::<_, (Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>)>(
                "SELECT clock_in_time, clock_out_time
             FROM attendance
             WHERE id = $1
             FOR UPDATE",
            )
            .bind(attendance_id)
            .fetch_optional(tx.as_mut())
            .await?;

        let Some((clock_in_time, clock_out_time)) = latest_attendance else {
            return Err(AppError::NotFound("Attendance record not found".into()));
        };

        let latest_breaks =
            sqlx::query_as::<_, (chrono::NaiveDateTime, Option<chrono::NaiveDateTime>)>(
                "SELECT break_start_time, break_end_time
             FROM break_records
             WHERE attendance_id = $1
             ORDER BY break_start_time ASC
             FOR UPDATE",
            )
            .bind(attendance_id)
            .fetch_all(tx.as_mut())
            .await?;

        let latest_snapshot = AttendanceCorrectionSnapshot {
            clock_in_time,
            clock_out_time,
            breaks: latest_breaks
                .into_iter()
                .map(|(break_start_time, break_end_time)| CorrectionBreakItem {
                    break_start_time,
                    break_end_time,
                })
                .collect(),
        };

        if &latest_snapshot != original_snapshot {
            return Err(AppError::Conflict(
                "Attendance record changed after request submission. Please resubmit.".into(),
            ));
        }

        let approve_query = "UPDATE attendance_correction_requests
            SET status = 'approved', approved_by = $1, approved_at = $2,
                decision_comment = $3, updated_at = $2
            WHERE id = $4 AND status = 'pending'";
        let affected = sqlx::query(approve_query)
            .bind(approver_id)
            .bind(now)
            .bind(comment)
            .bind(id)
            .execute(tx.as_mut())
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::Conflict(
                "Request not found or already processed".into(),
            ));
        }

        let upsert_query = "INSERT INTO attendance_correction_effective_values (
                attendance_id, source_request_id,
                clock_in_time_corrected, clock_out_time_corrected,
                break_records_corrected_json, applied_by, applied_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$7)
            ON CONFLICT(attendance_id) DO UPDATE SET
                source_request_id = EXCLUDED.source_request_id,
                clock_in_time_corrected = EXCLUDED.clock_in_time_corrected,
                clock_out_time_corrected = EXCLUDED.clock_out_time_corrected,
                break_records_corrected_json = EXCLUDED.break_records_corrected_json,
                applied_by = EXCLUDED.applied_by,
                applied_at = EXCLUDED.applied_at,
                updated_at = EXCLUDED.updated_at";
        sqlx::query(upsert_query)
            .bind(attendance_id)
            .bind(id)
            .bind(proposed_values.clock_in_time)
            .bind(proposed_values.clock_out_time)
            .bind(breaks_json)
            .bind(approver_id)
            .bind(now)
            .execute(tx.as_mut())
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_effective_values(
        &self,
        db: &PgPool,
        attendance_ids: &[AttendanceId],
    ) -> Result<Vec<AttendanceCorrectionEffectiveValue>, AppError> {
        if attendance_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<String> = attendance_ids.iter().map(|id| id.to_string()).collect();
        let query = "SELECT attendance_id, source_request_id,
                clock_in_time_corrected, clock_out_time_corrected,
                break_records_corrected_json, applied_by, applied_at, updated_at
            FROM attendance_correction_effective_values
            WHERE attendance_id = ANY($1)";

        Ok(
            sqlx::query_as::<_, AttendanceCorrectionEffectiveValue>(query)
                .bind(ids)
                .fetch_all(db)
                .await?,
        )
    }
}
