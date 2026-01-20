use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Error as SqlxError, FromRow, Postgres, QueryBuilder};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::{
        holiday::{
            CreateHolidayPayload, CreateWeeklyHolidayPayload, Holiday, HolidayResponse,
            WeeklyHoliday, WeeklyHolidayResponse,
        },
        user::User,
    },
    state::AppState,
    utils::time,
};

use super::common::{parse_optional_date, push_clause};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1_000;

pub async fn list_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AdminHolidayListQuery>,
) -> Result<Json<AdminHolidayListResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let AdminHolidayQueryParams {
        page,
        per_page,
        kind,
        from,
        to,
    } = validate_admin_holiday_query(q)?;
    let offset = (page - 1) * per_page;

    let (items, total) = fetch_admin_holidays(state.read_pool(), kind, from, to, per_page, offset)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    }))
}

pub async fn create_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateHolidayPayload>,
) -> Result<Json<HolidayResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let CreateHolidayPayload {
        holiday_date,
        name,
        description,
    } = payload;

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::BadRequest("Holiday name is required".into()));
    }

    let normalized_description = description.as_deref().map(str::trim).and_then(|d| {
        if d.is_empty() {
            None
        } else {
            Some(d.to_string())
        }
    });

    let holiday = Holiday::new(
        holiday_date,
        trimmed_name.to_string(),
        normalized_description,
    );

    let insert_result = sqlx::query(
        "INSERT INTO holidays (id, holiday_date, name, description, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(holiday.id)
    .bind(holiday.holiday_date)
    .bind(&holiday.name)
    .bind(&holiday.description)
    .bind(holiday.created_at)
    .bind(holiday.updated_at)
    .execute(&state.write_pool)
    .await;

    match insert_result {
        Ok(_) => Ok(Json(HolidayResponse::from(holiday))),
        Err(SqlxError::Database(db_err))
            if db_err.constraint() == Some("holidays_holiday_date_key") =>
        {
            Err(AppError::BadRequest(
                "Holiday already exists for that date".into(),
            ))
        }
        Err(e) => Err(AppError::InternalServerError(e.into())),
    }
}

pub async fn delete_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(holiday_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let result = sqlx::query("DELETE FROM holidays WHERE id = $1")
        .bind(&holiday_id)
        .execute(&state.write_pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Holiday not found".into()));
    }

    Ok(Json(json!({"message":"Holiday deleted","id": holiday_id})))
}

pub async fn list_weekly_holidays(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<WeeklyHolidayResponse>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let holidays = sqlx::query_as::<_, WeeklyHoliday>(
        "SELECT id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at \
         FROM weekly_holidays ORDER BY enforced_from, weekday",
    )
<<<<<<< HEAD
    .fetch_all(state.read_pool())
=======
    .fetch_all(&state.write_pool)
>>>>>>> 71ecf3c (feat: add read-replica aware state)
    .await
    .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(
        holidays
            .into_iter()
            .map(WeeklyHolidayResponse::from)
            .collect(),
    ))
}

pub async fn create_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateWeeklyHolidayPayload>,
) -> Result<Json<WeeklyHolidayResponse>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    if payload.weekday > 6 {
        return Err(AppError::BadRequest(
            "Weekday must be between 0 (Sun) and 6 (Sat). (Sun=0, Mon=1, ..., Sat=6)".into(),
        ));
    }

    if let Some(ends_on) = payload.ends_on {
        if ends_on < payload.starts_on {
            return Err(AppError::BadRequest(
                "End date must be on or after the start date".into(),
            ));
        }
    }

    let today = time::today_local(&state.config.time_zone);
    let tomorrow = today + Duration::days(1);
    if !user.is_system_admin() && payload.starts_on < tomorrow {
        return Err(AppError::BadRequest(
            "Start date must be at least tomorrow".into(),
        ));
    }

    let weekly = WeeklyHoliday::new(payload.weekday, payload.starts_on, payload.ends_on, user.id);

    sqlx::query(
        "INSERT INTO weekly_holidays \
            (id, weekday, starts_on, ends_on, enforced_from, enforced_to, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(weekly.id)
    .bind(weekly.weekday)
    .bind(weekly.starts_on)
    .bind(weekly.ends_on)
    .bind(weekly.enforced_from)
    .bind(weekly.enforced_to)
    .bind(weekly.created_by)
    .bind(weekly.created_at)
    .bind(weekly.updated_at)
    .execute(&state.write_pool)
    .await?;

    Ok(Json(WeeklyHolidayResponse::from(weekly)))
}

pub async fn delete_weekly_holiday(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let result = sqlx::query("DELETE FROM weekly_holidays WHERE id = $1")
        .bind(&id)
        .execute(&state.write_pool)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Weekly holiday not found".into()));
    }

    Ok(Json(json!({"message":"Weekly holiday deleted","id": id})))
}

// ============================================================================
// Internal helpers and types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminHolidayQueryParams {
    pub page: i64,
    pub per_page: i64,
    pub kind: Option<AdminHolidayKind>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub fn validate_admin_holiday_query(
    q: AdminHolidayListQuery,
) -> Result<AdminHolidayQueryParams, AppError> {
    let page = q.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let per_page = q
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let kind =
        parse_type_filter(q.r#type.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let from =
        parse_optional_date(q.from.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;
    let to = parse_optional_date(q.to.as_deref()).map_err(|e| AppError::BadRequest(e.into()))?;

    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::BadRequest(
                "`from` must be before or equal to `to`".into(),
            ));
        }
    }

    Ok(AdminHolidayQueryParams {
        page,
        per_page,
        kind,
        from,
        to,
    })
}

fn parse_type_filter(raw: Option<&str>) -> Result<Option<AdminHolidayKind>, &'static str> {
    match raw {
        Some(value) if value.eq_ignore_ascii_case("all") => Ok(None),
        Some(value) => AdminHolidayKind::from_str(value)
            .map(Some)
            .map_err(|_| "`type` must be one of public, weekly, exception, all"),
        None => Ok(None),
    }
}

fn apply_holiday_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    has_clause: &mut bool,
    kind: Option<AdminHolidayKind>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) {
    if let Some(kind) = kind {
        push_clause(builder, has_clause);
        builder.push("kind = ").push_bind(kind.as_str());
    }
    if let Some(from) = from {
        push_clause(builder, has_clause);
        builder.push("applies_from >= ").push_bind(from);
    }
    if let Some(to) = to {
        push_clause(builder, has_clause);
        builder.push("applies_from <= ").push_bind(to);
    }
}

async fn fetch_admin_holidays(
    pool: &sqlx::PgPool,
    kind: Option<AdminHolidayKind>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<AdminHolidayListItem>, i64), SqlxError> {
    let mut data_builder = QueryBuilder::new(
        r#"
        WITH unioned AS (
            SELECT id,
                   'public'::text AS kind,
                   holiday_date AS applies_from,
                   holiday_date AS applies_to,
                   holiday_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   name,
                   description,
                   NULL::text AS user_id,
                   description AS reason,
                   NULL::text AS created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM holidays
            UNION ALL
            SELECT id,
                   'weekly'::text AS kind,
                   enforced_from AS applies_from,
                   enforced_to AS applies_to,
                   NULL::date AS date,
                   weekday,
                   starts_on,
                   ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   NULL::text AS user_id,
                   NULL::text AS reason,
                   created_by,
                   created_at,
                   NULL::boolean AS is_override
            FROM weekly_holidays
            UNION ALL
            SELECT id,
                   'exception'::text AS kind,
                   exception_date AS applies_from,
                   exception_date AS applies_to,
                   exception_date AS date,
                   NULL::smallint AS weekday,
                   NULL::date AS starts_on,
                   NULL::date AS ends_on,
                   NULL::text AS name,
                   NULL::text AS description,
                   user_id,
                   reason,
                   created_by,
                   created_at,
                   override AS is_override
            FROM holiday_exceptions
        )
        SELECT id, kind, applies_from, applies_to, date, weekday, starts_on, ends_on,
               name, description, user_id, reason, created_by, created_at, is_override
        FROM unioned
        "#,
    );

    let mut data_has_clause = false;
    apply_holiday_filters(&mut data_builder, &mut data_has_clause, kind, from, to);

    data_builder
        .push(" ORDER BY applies_from DESC, kind ASC, created_at DESC")
        .push(" LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(offset);

    let mut count_builder = QueryBuilder::new(
        r#"
        SELECT COUNT(*) FROM (
            WITH unioned AS (
                SELECT id,
                       'public'::text AS kind,
                       holiday_date AS applies_from,
                       holiday_date AS applies_to,
                       holiday_date AS date,
                       NULL::smallint AS weekday,
                       NULL::date AS starts_on,
                       NULL::date AS ends_on,
                       name,
                       description,
                       NULL::text AS user_id,
                       description AS reason,
                       NULL::text AS created_by,
                       created_at,
                       NULL::boolean AS is_override
                FROM holidays
            UNION ALL
                SELECT id,
                       'weekly'::text AS kind,
                       enforced_from AS applies_from,
                       enforced_to AS applies_to,
                       NULL::date AS date,
                       weekday,
                       starts_on,
                       ends_on,
                       NULL::text AS name,
                       NULL::text AS description,
                       NULL::text AS user_id,
                       NULL::text AS reason,
                       created_by,
                       created_at,
                       NULL::boolean AS is_override
                FROM weekly_holidays
            UNION ALL
                SELECT id,
                       'exception'::text AS kind,
                       exception_date AS applies_from,
                       exception_date AS applies_to,
                       exception_date AS date,
                       NULL::smallint AS weekday,
                       NULL::date AS starts_on,
                       NULL::date AS ends_on,
                       NULL::text AS name,
                       NULL::text AS description,
                       user_id,
                       reason,
                       created_by,
                       created_at,
                       override AS is_override
                FROM holiday_exceptions
            )
            SELECT 1
            FROM unioned
        "#,
    );

    let mut count_has_clause = false;
    apply_holiday_filters(&mut count_builder, &mut count_has_clause, kind, from, to);
    count_builder.push(") AS counted");

    let rows = data_builder
        .build_query_as::<AdminHolidayRow>()
        .fetch_all(pool)
        .await?;

    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    let items = rows
        .into_iter()
        .map(AdminHolidayListItem::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SqlxError::Protocol("invalid holiday kind".into()))?;

    Ok((items, total))
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AdminHolidayListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminHolidayListResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub items: Vec<AdminHolidayListItem>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminHolidayKind {
    Public,
    Weekly,
    Exception,
}

impl AdminHolidayKind {
    fn as_str(&self) -> &'static str {
        match self {
            AdminHolidayKind::Public => "public",
            AdminHolidayKind::Weekly => "weekly",
            AdminHolidayKind::Exception => "exception",
        }
    }
}

impl FromStr for AdminHolidayKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Ok(AdminHolidayKind::Public),
            "weekly" => Ok(AdminHolidayKind::Weekly),
            "exception" => Ok(AdminHolidayKind::Exception),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminHolidayListItem {
    pub id: String,
    pub kind: AdminHolidayKind,
    pub applies_from: NaiveDate,
    pub applies_to: Option<NaiveDate>,
    pub date: Option<NaiveDate>,
    pub weekday: Option<i16>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_override: Option<bool>,
}

#[derive(Debug, FromRow)]
struct AdminHolidayRow {
    id: String,
    kind: String,
    applies_from: NaiveDate,
    applies_to: Option<NaiveDate>,
    date: Option<NaiveDate>,
    weekday: Option<i16>,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    name: Option<String>,
    description: Option<String>,
    user_id: Option<String>,
    reason: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    is_override: Option<bool>,
}

impl TryFrom<AdminHolidayRow> for AdminHolidayListItem {
    type Error = ();

    fn try_from(row: AdminHolidayRow) -> Result<Self, Self::Error> {
        let kind = AdminHolidayKind::from_str(&row.kind)?;
        Ok(Self {
            id: row.id,
            kind,
            applies_from: row.applies_from,
            applies_to: row.applies_to,
            date: row.date,
            weekday: row.weekday,
            starts_on: row.starts_on,
            ends_on: row.ends_on,
            name: row.name,
            description: row.description,
            user_id: row.user_id,
            reason: row.reason,
            created_by: row.created_by,
            created_at: row.created_at,
            is_override: row.is_override,
        })
    }
}
