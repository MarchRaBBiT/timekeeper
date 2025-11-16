use chrono::{Datelike, Duration, NaiveDate};
use chrono_tz::Asia::Tokyo;
use sqlx::PgPool;
use timekeeper_backend::{
    config::Config,
    models::user::{User, UserRole},
    models::{
        leave_request::{LeaveRequest, LeaveType},
        overtime_request::OvertimeRequest,
    },
};
use uuid::Uuid;

pub fn test_config() -> Config {
    Config {
        database_url: "postgres://timekeeper:timekeeper@localhost:5432/timekeeper".into(),
        jwt_secret: "a_secure_token_that_is_long_enough_123".into(),
        jwt_expiration_hours: 1,
        refresh_token_expiration_days: 7,
        time_zone: Tokyo,
        mfa_issuer: "Timekeeper".into(),
    }
}

pub async fn seed_user(pool: &PgPool, role: UserRole, is_system_admin: bool) -> User {
    let user = User::new(
        format!("user_{}", Uuid::new_v4().to_string()),
        "hash".into(),
        "Test User".into(),
        role,
        is_system_admin,
    );

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name, role, is_system_admin, \
         mfa_secret, mfa_enabled_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.role.as_str())
    .bind(&user.is_system_admin)
    .bind(&user.mfa_secret)
    .bind(&user.mfa_enabled_at)
    .bind(&user.created_at)
    .bind(&user.updated_at)
    .execute(pool)
    .await
    .expect("insert user");

    user
}

pub async fn seed_weekly_holiday(pool: &PgPool, date: NaiveDate) {
    let weekday = date.weekday().num_days_from_monday() as i16;
    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, NULL, $4, NULL, 'test', NOW(), NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(weekday)
    .bind(date - Duration::days(7))
    .bind(date)
    .execute(pool)
    .await
    .expect("insert weekly holiday");
}

pub async fn seed_leave_request(
    pool: &PgPool,
    user_id: &str,
    leave_type: LeaveType,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> LeaveRequest {
    let request = LeaveRequest::new(
        user_id.to_string(),
        leave_type,
        start_date,
        end_date,
        Some("test".into()),
    );
    sqlx::query(
        "INSERT INTO leave_requests (id, user_id, leave_type, start_date, end_date, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(&request.id)
    .bind(&request.user_id)
    .bind(match request.leave_type {
        LeaveType::Annual => "annual",
        LeaveType::Sick => "sick",
        LeaveType::Personal => "personal",
        LeaveType::Other => "other",
    })
    .bind(&request.start_date)
    .bind(&request.end_date)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::leave_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::leave_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::leave_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::leave_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&request.approved_by)
    .bind(&request.approved_at)
    .bind(&request.decision_comment)
    .bind(&request.rejected_by)
    .bind(&request.rejected_at)
    .bind(&request.cancelled_at)
    .bind(&request.created_at)
    .bind(&request.updated_at)
    .execute(pool)
    .await
    .expect("insert leave request");
    request
}

pub async fn seed_overtime_request(
    pool: &PgPool,
    user_id: &str,
    date: NaiveDate,
    planned_hours: f64,
) -> OvertimeRequest {
    let request = OvertimeRequest::new(
        user_id.to_string(),
        date,
        planned_hours,
        Some("test OT".into()),
    );
    sqlx::query(
        "INSERT INTO overtime_requests (id, user_id, date, planned_hours, reason, status, approved_by, approved_at, decision_comment, rejected_by, rejected_at, cancelled_at, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(&request.id)
    .bind(&request.user_id)
    .bind(&request.date)
    .bind(&request.planned_hours)
    .bind(&request.reason)
    .bind(match request.status {
        timekeeper_backend::models::overtime_request::RequestStatus::Pending => "pending",
        timekeeper_backend::models::overtime_request::RequestStatus::Approved => "approved",
        timekeeper_backend::models::overtime_request::RequestStatus::Rejected => "rejected",
        timekeeper_backend::models::overtime_request::RequestStatus::Cancelled => "cancelled",
    })
    .bind(&request.approved_by)
    .bind(&request.approved_at)
    .bind(&request.decision_comment)
    .bind(&request.rejected_by)
    .bind(&request.rejected_at)
    .bind(&request.cancelled_at)
    .bind(&request.created_at)
    .bind(&request.updated_at)
    .execute(pool)
    .await
    .expect("insert overtime request");
    request
}
