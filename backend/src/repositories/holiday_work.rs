use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use timekeeper_app::holiday_work::{compensatory_expiry, HolidayWorkError};
use timekeeper_contract::holiday_work::{
    HolidayWorkBenefit, HolidayWorkRequestResponse, HolidayWorkRequestStatus,
    SubmitHolidayWorkRequest,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum HolidayWorkRepositoryError {
    #[error("holiday work request not found")]
    NotFound,
    #[error("request is not pending")]
    NotPending,
    #[error("resolved workday is missing")]
    WorkdayNotResolved,
    #[error("origin must be a non-working day and substitute must be a scheduled workday")]
    InvalidDayKinds,
    #[error("one or both workdays are locked")]
    Locked,
    #[error("approved compensatory leave has already been consumed")]
    CompensatoryAlreadyConsumed,
    #[error("actor is not authorized to decide this request")]
    Unauthorized,
    #[error(transparent)]
    Domain(#[from] HolidayWorkError),
    #[error("repository error: {0}")]
    Repository(String),
}

#[derive(Debug, Clone)]
pub struct HolidayWorkRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, FromRow)]
struct HolidayWorkRow {
    id: Uuid,
    user_id: String,
    work_date: NaiveDate,
    benefit: String,
    substitute_date: Option<NaiveDate>,
    compensatory_minutes: Option<i32>,
    status: String,
    reason: String,
    decision_comment: Option<String>,
    decided_by: Option<String>,
    decided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ResolvedDayRow {
    work_date: NaiveDate,
    day_kind: String,
    work_schedule_id: Uuid,
    locked_at: Option<DateTime<Utc>>,
}

const COLUMNS: &str = "id, user_id, work_date, benefit, substitute_date,
    compensatory_minutes, status, reason, decision_comment, decided_by,
    decided_at, created_at, updated_at";

impl HolidayWorkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn submit(
        &self,
        user_id: &str,
        input: SubmitHolidayWorkRequest,
    ) -> Result<HolidayWorkRequestResponse, HolidayWorkRepositoryError> {
        timekeeper_app::holiday_work::validate_submission(&input)?;
        let id = Uuid::new_v4();
        let sql = format!(
            "INSERT INTO holiday_work_requests
             (id, user_id, work_date, benefit, substitute_date,
              compensatory_minutes, reason)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {COLUMNS}"
        );
        let row = sqlx::query_as::<_, HolidayWorkRow>(&sql)
            .bind(id)
            .bind(user_id)
            .bind(input.work_date)
            .bind(benefit_str(input.benefit))
            .bind(input.substitute_date)
            .bind(input.compensatory_minutes)
            .bind(input.reason)
            .fetch_one(&self.pool)
            .await
            .map_err(repository_error)?;
        row.try_into()
    }

    pub async fn list_for_user(
        &self,
        user_id: &str,
        status: Option<HolidayWorkRequestStatus>,
    ) -> Result<Vec<HolidayWorkRequestResponse>, HolidayWorkRepositoryError> {
        let status = status.map(status_str);
        let sql = format!(
            "SELECT {COLUMNS} FROM holiday_work_requests
             WHERE user_id = $1 AND ($2::TEXT IS NULL OR status = $2)
             ORDER BY created_at DESC, id"
        );
        sqlx::query_as::<_, HolidayWorkRow>(&sql)
            .bind(user_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub async fn list_pending_in_scope(
        &self,
        actor_id: &str,
        system_admin: bool,
    ) -> Result<Vec<HolidayWorkRequestResponse>, HolidayWorkRepositoryError> {
        let sql = "WITH RECURSIVE subordinate_depts AS (
                       SELECT dm.department_id
                       FROM department_managers dm
                       WHERE dm.user_id = $1
                       UNION
                       SELECT d.id
                       FROM departments d
                       JOIN subordinate_depts parent ON d.parent_id = parent.department_id
                   )
                   SELECT h.id, h.user_id, h.work_date, h.benefit, h.substitute_date,
                    h.compensatory_minutes, h.status, h.reason, h.decision_comment,
                    h.decided_by, h.decided_at, h.created_at, h.updated_at
             FROM holiday_work_requests h
             JOIN users u ON u.id = h.user_id
             WHERE h.status = 'pending' AND
               ($2 OR u.department_id IN (SELECT department_id FROM subordinate_depts))
             ORDER BY h.created_at, h.id";
        sqlx::query_as::<_, HolidayWorkRow>(sql)
            .bind(actor_id)
            .bind(system_admin)
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub async fn reject(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_is_system_admin: bool,
        comment: &str,
    ) -> Result<HolidayWorkRequestResponse, HolidayWorkRepositoryError> {
        self.decide_without_effect(id, actor_id, actor_is_system_admin, comment, "rejected")
            .await
    }

    pub async fn cancel(
        &self,
        id: Uuid,
        user_id: &str,
    ) -> Result<HolidayWorkRequestResponse, HolidayWorkRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(repository_error)?;
        let select_sql = format!(
            "SELECT {COLUMNS} FROM holiday_work_requests
             WHERE id = $1 AND user_id = $2 FOR UPDATE"
        );
        let row = sqlx::query_as::<_, HolidayWorkRow>(&select_sql)
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(repository_error)?
            .ok_or(HolidayWorkRepositoryError::NotFound)?;
        let current: HolidayWorkRequestResponse = row.try_into()?;
        match current.status {
            HolidayWorkRequestStatus::Pending => {}
            HolidayWorkRequestStatus::Approved => match current.benefit {
                HolidayWorkBenefit::Substitution => {
                    reverse_substitution(&mut tx, &current).await?;
                }
                HolidayWorkBenefit::Compensatory => {
                    reverse_compensatory(&mut tx, &current, user_id).await?;
                }
            },
            _ => return Err(HolidayWorkRepositoryError::NotPending),
        }
        let sql = format!(
            "UPDATE holiday_work_requests
             SET status = 'cancelled', decision_comment = NULL, decided_by = NULL,
                 decided_at = NOW(), updated_at = NOW()
             WHERE id = $1 RETURNING {COLUMNS}"
        );
        let cancelled = sqlx::query_as::<_, HolidayWorkRow>(&sql)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(repository_error)?;
        tx.commit().await.map_err(repository_error)?;
        cancelled.try_into()
    }

    pub async fn approve(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_is_system_admin: bool,
        comment: &str,
    ) -> Result<HolidayWorkRequestResponse, HolidayWorkRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(repository_error)?;
        let sql = format!(
            "SELECT {COLUMNS} FROM holiday_work_requests
             WHERE id = $1 FOR UPDATE"
        );
        let row = sqlx::query_as::<_, HolidayWorkRow>(&sql)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(repository_error)?
            .ok_or(HolidayWorkRepositoryError::NotFound)?;
        let current: HolidayWorkRequestResponse = row.clone().try_into()?;
        ensure_decision_authorized(&mut tx, actor_id, &current.user_id, actor_is_system_admin)
            .await?;
        timekeeper_app::holiday_work::validate_decision(
            current.status,
            &current.user_id,
            actor_id,
        )?;
        match current.benefit {
            HolidayWorkBenefit::Substitution => {
                approve_substitution(&mut tx, &current, actor_id).await?
            }
            HolidayWorkBenefit::Compensatory => {
                approve_compensatory(&mut tx, &current, actor_id).await?
            }
        }
        let update_sql = format!(
            "UPDATE holiday_work_requests
             SET status = 'approved', decision_comment = $2, decided_by = $3,
                 decided_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND status = 'pending' RETURNING {COLUMNS}"
        );
        let approved = sqlx::query_as::<_, HolidayWorkRow>(&update_sql)
            .bind(id)
            .bind(comment)
            .bind(actor_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(repository_error)?
            .ok_or(HolidayWorkRepositoryError::NotPending)?;
        tx.commit().await.map_err(repository_error)?;
        approved.try_into()
    }

    async fn decide_without_effect(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_is_system_admin: bool,
        comment: &str,
        status: &str,
    ) -> Result<HolidayWorkRequestResponse, HolidayWorkRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(repository_error)?;
        let select_sql =
            format!("SELECT {COLUMNS} FROM holiday_work_requests WHERE id = $1 FOR UPDATE");
        let current = sqlx::query_as::<_, HolidayWorkRow>(&select_sql)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(repository_error)?
            .ok_or(HolidayWorkRepositoryError::NotFound)?;
        ensure_decision_authorized(&mut tx, actor_id, &current.user_id, actor_is_system_admin)
            .await?;
        let sql = format!(
            "UPDATE holiday_work_requests SET status = $2, decision_comment = $3,
             decided_by = $4, decided_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND status = 'pending' RETURNING {COLUMNS}"
        );
        let row = sqlx::query_as::<_, HolidayWorkRow>(&sql)
            .bind(id)
            .bind(status)
            .bind(comment)
            .bind(actor_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(repository_error)?
            .ok_or(HolidayWorkRepositoryError::NotPending)?;
        tx.commit().await.map_err(repository_error)?;
        row.try_into()
    }
}

async fn ensure_decision_authorized(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    applicant_id: &str,
    actor_is_system_admin: bool,
) -> Result<(), HolidayWorkRepositoryError> {
    if actor_id == applicant_id {
        return Err(HolidayWorkRepositoryError::Unauthorized);
    }
    if actor_is_system_admin {
        return Ok(());
    }

    // Lock the applicant row so department reassignment cannot race this
    // authorization decision. Lock the manager's current root assignments so
    // revocation cannot race it either.
    let applicant_department = sqlx::query_scalar::<_, Option<String>>(
        "SELECT department_id FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(applicant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(repository_error)?
    .flatten()
    .ok_or(HolidayWorkRepositoryError::Unauthorized)?;
    let managed_roots = sqlx::query_scalar::<_, String>(
        "SELECT department_id FROM department_managers
         WHERE user_id = $1 ORDER BY department_id FOR SHARE",
    )
    .bind(actor_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(repository_error)?;
    if managed_roots.is_empty() {
        return Err(HolidayWorkRepositoryError::Unauthorized);
    }
    // Lock every hierarchy row used by the recursive scope decision. A
    // concurrent department reparent must wait until this decision commits,
    // preventing an approval based on a hierarchy that has already changed.
    let scoped_departments = sqlx::query_scalar::<_, String>(
        "WITH RECURSIVE subordinate_depts AS (
             SELECT UNNEST($1::TEXT[]) AS department_id
             UNION
             SELECT d.id FROM departments d
             JOIN subordinate_depts parent ON d.parent_id = parent.department_id
         )
         SELECT d.id
         FROM departments d
         JOIN subordinate_depts scoped ON scoped.department_id = d.id
         ORDER BY d.id
         FOR SHARE OF d",
    )
    .bind(managed_roots)
    .fetch_all(&mut **tx)
    .await
    .map_err(repository_error)?;
    if scoped_departments
        .iter()
        .any(|department| department == &applicant_department)
    {
        Ok(())
    } else {
        Err(HolidayWorkRepositoryError::Unauthorized)
    }
}

async fn reverse_substitution(
    tx: &mut Transaction<'_, Postgres>,
    request: &HolidayWorkRequestResponse,
) -> Result<(), HolidayWorkRepositoryError> {
    let substitute_date = request
        .substitute_date
        .ok_or(HolidayWorkError::InvalidSubstitution)?;
    // Serialize cancellation with monthly closing. Merely checking locked_at
    // leaves a window in which closing can lock the rows before the overrides
    // are deleted.
    let lock_states = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT locked_at
         FROM resolved_workdays
         WHERE user_id = $1 AND work_date = ANY($2)
         FOR UPDATE",
    )
    .bind(&request.user_id)
    .bind(vec![request.work_date, substitute_date])
    .fetch_all(&mut **tx)
    .await
    .map_err(repository_error)?;
    if lock_states.into_iter().any(|locked_at| locked_at.is_some()) {
        return Err(HolidayWorkRepositoryError::Locked);
    }
    let reason = format!("holiday substitution request {}", request.id);
    sqlx::query(
        "DELETE FROM workday_overrides
         WHERE user_id = $1 AND work_date = ANY($2) AND reason = $3",
    )
    .bind(&request.user_id)
    .bind(vec![request.work_date, substitute_date])
    .bind(reason)
    .execute(&mut **tx)
    .await
    .map_err(repository_error)?;
    Ok(())
}

async fn reverse_compensatory(
    tx: &mut Transaction<'_, Postgres>,
    request: &HolidayWorkRequestResponse,
    actor_id: &str,
) -> Result<(), HolidayWorkRepositoryError> {
    let request_id = Uuid::parse_str(&request.id)
        .map_err(|error| HolidayWorkRepositoryError::Repository(error.to_string()))?;
    let grant = sqlx::query_as::<_, (Uuid, i32)>(
        "SELECT lot_id, day_equivalent_minutes FROM leave_ledger_entries
         WHERE holiday_work_request_id = $1 AND kind = 'grant' FOR UPDATE",
    )
    .bind(request_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(repository_error)?
    .ok_or(HolidayWorkRepositoryError::NotFound)?;
    let remaining = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(amount_minutes), 0)::BIGINT
         FROM leave_ledger_entries WHERE lot_id = $1",
    )
    .bind(grant.0)
    .fetch_one(&mut **tx)
    .await
    .map_err(repository_error)?;
    let granted = i64::from(
        request
            .compensatory_minutes
            .ok_or(HolidayWorkError::InvalidCompensatoryMinutes)?,
    );
    if remaining != granted {
        return Err(HolidayWorkRepositoryError::CompensatoryAlreadyConsumed);
    }
    sqlx::query(
        "INSERT INTO leave_ledger_entries
         (id,user_id,leave_type,kind,lot_id,amount_minutes,obligation_minutes,
          day_equivalent_minutes,reason,created_by,effective_at)
         VALUES ($1,$2,'compensatory','adjust',$3,$4,0,$5,$6,$7,NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(&request.user_id)
    .bind(grant.0)
    .bind(-granted)
    .bind(grant.1)
    .bind(format!("cancel holiday work request {}", request.id))
    .bind(actor_id)
    .execute(&mut **tx)
    .await
    .map_err(repository_error)?;
    Ok(())
}

async fn approve_substitution(
    tx: &mut Transaction<'_, Postgres>,
    request: &HolidayWorkRequestResponse,
    actor_id: &str,
) -> Result<(), HolidayWorkRepositoryError> {
    let substitute_date = request
        .substitute_date
        .ok_or(HolidayWorkError::InvalidSubstitution)?;
    let rows = sqlx::query_as::<_, ResolvedDayRow>(
        "SELECT work_date, day_kind, work_schedule_id, locked_at FROM resolved_workdays
         WHERE user_id = $1 AND work_date = ANY($2) ORDER BY work_date FOR UPDATE",
    )
    .bind(&request.user_id)
    .bind(vec![request.work_date, substitute_date])
    .fetch_all(&mut **tx)
    .await
    .map_err(repository_error)?;
    if rows.len() != 2 {
        return Err(HolidayWorkRepositoryError::WorkdayNotResolved);
    }
    if rows.iter().any(|row| row.locked_at.is_some()) {
        return Err(HolidayWorkRepositoryError::Locked);
    }
    let origin = rows
        .iter()
        .find(|row| row.work_date == request.work_date)
        .ok_or(HolidayWorkRepositoryError::WorkdayNotResolved)?;
    let substitute = rows
        .iter()
        .find(|row| row.work_date == substitute_date)
        .ok_or(HolidayWorkRepositoryError::WorkdayNotResolved)?;
    if !matches!(
        origin.day_kind.as_str(),
        "scheduled_non_working_day" | "public_holiday"
    ) || substitute.day_kind != "scheduled_workday"
    {
        return Err(HolidayWorkRepositoryError::InvalidDayKinds);
    }
    let reason = format!("holiday substitution request {}", request.id);
    sqlx::query(
        "INSERT INTO workday_overrides
         (id, user_id, work_date, kind, work_schedule_id, reason, created_by)
         VALUES ($1,$2,$3,'use_schedule',$4,$5,$6),
                ($7,$2,$8,'non_working_day',NULL,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(&request.user_id)
    .bind(request.work_date)
    .bind(origin.work_schedule_id)
    .bind(reason)
    .bind(actor_id)
    .bind(Uuid::new_v4())
    .bind(substitute_date)
    .execute(&mut **tx)
    .await
    .map_err(repository_error)?;
    Ok(())
}

async fn approve_compensatory(
    tx: &mut Transaction<'_, Postgres>,
    request: &HolidayWorkRequestResponse,
    actor_id: &str,
) -> Result<(), HolidayWorkRepositoryError> {
    let minutes = request
        .compensatory_minutes
        .ok_or(HolidayWorkError::InvalidCompensatoryMinutes)?;
    let expiry_months = sqlx::query_scalar::<_, i32>(
        "SELECT expiry_months FROM compensatory_leave_settings
         WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(request.work_date)
    .fetch_optional(&mut **tx)
    .await
    .map_err(repository_error)?
    .and_then(|months| u32::try_from(months).ok());
    let expires_at = compensatory_expiry(request.work_date, expiry_months)?;
    sqlx::query(
        "INSERT INTO leave_ledger_entries
         (id, user_id, leave_type, kind, lot_id, amount_minutes,
          obligation_minutes, day_equivalent_minutes, granted_at, expires_at,
          grant_base_date, reason, created_by, effective_at, holiday_work_request_id)
         VALUES ($1,$2,'compensatory','grant',$3,$4,0,$4,$5,$6,$5,$7,$8,NOW(),$9)
         ON CONFLICT (holiday_work_request_id) WHERE
             kind = 'grant' AND holiday_work_request_id IS NOT NULL DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(&request.user_id)
    .bind(Uuid::new_v4())
    .bind(minutes)
    .bind(request.work_date)
    .bind(expires_at)
    .bind(format!("holiday work request {}", request.id))
    .bind(actor_id)
    .bind(
        Uuid::parse_str(&request.id)
            .map_err(|error| HolidayWorkRepositoryError::Repository(error.to_string()))?,
    )
    .execute(&mut **tx)
    .await
    .map_err(repository_error)?;
    Ok(())
}

fn benefit_str(value: HolidayWorkBenefit) -> &'static str {
    match value {
        HolidayWorkBenefit::Substitution => "substitution",
        HolidayWorkBenefit::Compensatory => "compensatory",
    }
}

fn status_str(value: HolidayWorkRequestStatus) -> &'static str {
    match value {
        HolidayWorkRequestStatus::Pending => "pending",
        HolidayWorkRequestStatus::Approved => "approved",
        HolidayWorkRequestStatus::Rejected => "rejected",
        HolidayWorkRequestStatus::Cancelled => "cancelled",
    }
}

fn parse_benefit(value: &str) -> Result<HolidayWorkBenefit, HolidayWorkRepositoryError> {
    match value {
        "substitution" => Ok(HolidayWorkBenefit::Substitution),
        "compensatory" => Ok(HolidayWorkBenefit::Compensatory),
        _ => Err(HolidayWorkRepositoryError::Repository(
            "invalid stored benefit".into(),
        )),
    }
}

fn parse_status(value: &str) -> Result<HolidayWorkRequestStatus, HolidayWorkRepositoryError> {
    match value {
        "pending" => Ok(HolidayWorkRequestStatus::Pending),
        "approved" => Ok(HolidayWorkRequestStatus::Approved),
        "rejected" => Ok(HolidayWorkRequestStatus::Rejected),
        "cancelled" => Ok(HolidayWorkRequestStatus::Cancelled),
        _ => Err(HolidayWorkRepositoryError::Repository(
            "invalid stored status".into(),
        )),
    }
}

impl TryFrom<HolidayWorkRow> for HolidayWorkRequestResponse {
    type Error = HolidayWorkRepositoryError;

    fn try_from(row: HolidayWorkRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.to_string(),
            user_id: row.user_id,
            work_date: row.work_date,
            benefit: parse_benefit(&row.benefit)?,
            substitute_date: row.substitute_date,
            compensatory_minutes: row.compensatory_minutes,
            status: parse_status(&row.status)?,
            reason: row.reason,
            decision_comment: row.decision_comment,
            decided_by: row.decided_by,
            decided_at: row.decided_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn repository_error(error: sqlx::Error) -> HolidayWorkRepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("23514") && database.message().contains("locked") {
            return HolidayWorkRepositoryError::Locked;
        }
    }
    HolidayWorkRepositoryError::Repository(error.to_string())
}
