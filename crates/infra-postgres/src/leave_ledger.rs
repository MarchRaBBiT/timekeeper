use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use timekeeper_app::leave_ledger::{
    GrantCandidate, LeaveGrantUserRepository, LeaveLedgerError, LeaveLedgerRepository,
    LeaveRuleRepository, NewLeaveLedgerEntry, StoredLeaveLedgerEntry,
};
use timekeeper_domain::leave_ledger::{LeaveGrantRule, LeaveLedgerKind, LeaveObligationRule};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LeaveLedgerPostgresRepository {
    pool: PgPool,
}

impl LeaveLedgerPostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_entries_for_update(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &str,
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        sqlx::query_as::<_, LeaveLedgerEntryRow>(
            "SELECT id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at, created_at
             FROM leave_ledger_entries
             WHERE user_id = $1 AND leave_type = $2
             ORDER BY effective_at ASC, created_at ASC, id ASC
             FOR UPDATE",
        )
        .bind(user_id)
        .bind(leave_type)
        .fetch_all(&mut **tx)
        .await
        .map_err(repository_error)?
        .into_iter()
        .map(row_to_stored_entry)
        .collect()
    }

    pub async fn append_entries_in_transaction(
        tx: &mut Transaction<'_, Postgres>,
        entries: Vec<NewLeaveLedgerEntry>,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        let mut stored = Vec::with_capacity(entries.len());
        for entry in entries {
            let id = Uuid::new_v4();
            let lot_id = match entry.lot_id {
                Some(lot_id) => parse_uuid(&lot_id, "lot_id")?,
                None => Uuid::new_v4(),
            };
            let amount_minutes = i32::try_from(entry.amount_minutes).map_err(|_| {
                LeaveLedgerError::InvalidInput("amount_minutes is out of range".to_string())
            })?;
            let day_equivalent_minutes =
                i32::try_from(entry.day_equivalent_minutes).map_err(|_| {
                    LeaveLedgerError::InvalidInput(
                        "day_equivalent_minutes is out of range".to_string(),
                    )
                })?;
            let row = sqlx::query_as::<_, LeaveLedgerEntryRow>(
                "INSERT INTO leave_ledger_entries (
                    id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at
                 ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                 RETURNING id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at, created_at",
            )
            .bind(id)
            .bind(entry.user_id)
            .bind(entry.leave_type)
            .bind(kind_to_str(entry.kind))
            .bind(lot_id)
            .bind(amount_minutes)
            .bind(day_equivalent_minutes)
            .bind(entry.granted_at)
            .bind(entry.expires_at)
            .bind(entry.grant_base_date)
            .bind(entry.leave_request_id)
            .bind(entry.reason)
            .bind(entry.created_by)
            .bind(entry.effective_at)
            .fetch_one(&mut **tx)
            .await
            .map_err(repository_error)?;
            stored.push(row_to_stored_entry(row)?);
        }
        Ok(stored)
    }
}

#[derive(Debug, Clone, FromRow)]
struct LeaveLedgerEntryRow {
    id: Uuid,
    user_id: String,
    leave_type: String,
    kind: String,
    lot_id: Uuid,
    amount_minutes: i32,
    day_equivalent_minutes: i32,
    granted_at: Option<NaiveDate>,
    expires_at: Option<NaiveDate>,
    grant_base_date: Option<NaiveDate>,
    leave_request_id: Option<String>,
    reason: Option<String>,
    created_by: Option<String>,
    effective_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct GrantRuleRow {
    tenure_months: i32,
    granted_days: i32,
    expiry_months: i32,
    day_equivalent_minutes: i32,
}

#[derive(Debug, Clone, FromRow)]
struct ObligationRuleRow {
    minimum_granted_days: i32,
    required_days: i32,
    window_months: i32,
    warning_lead_days: i32,
}

#[derive(Debug, Clone, FromRow)]
struct GrantCandidateRow {
    id: String,
    hire_date: Option<NaiveDate>,
}

#[async_trait]
impl LeaveLedgerRepository for LeaveLedgerPostgresRepository {
    async fn list_entries(
        &self,
        user_id: &str,
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        sqlx::query_as::<_, LeaveLedgerEntryRow>(
            "SELECT id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at, created_at
             FROM leave_ledger_entries
             WHERE user_id = $1 AND leave_type = $2
             ORDER BY effective_at ASC, created_at ASC, id ASC",
        )
        .bind(user_id)
        .bind(leave_type)
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)?
        .into_iter()
        .map(row_to_stored_entry)
        .collect()
    }

    async fn append_entries(
        &self,
        entries: Vec<NewLeaveLedgerEntry>,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        let mut tx = self.pool.begin().await.map_err(repository_error)?;
        let mut stored = Vec::with_capacity(entries.len());
        for entry in entries {
            let id = Uuid::new_v4();
            let lot_id = match entry.lot_id {
                Some(lot_id) => parse_uuid(&lot_id, "lot_id")?,
                None => Uuid::new_v4(),
            };
            let amount_minutes = i32::try_from(entry.amount_minutes).map_err(|_| {
                LeaveLedgerError::InvalidInput("amount_minutes is out of range".to_string())
            })?;
            let day_equivalent_minutes =
                i32::try_from(entry.day_equivalent_minutes).map_err(|_| {
                    LeaveLedgerError::InvalidInput(
                        "day_equivalent_minutes is out of range".to_string(),
                    )
                })?;
            let row = sqlx::query_as::<_, LeaveLedgerEntryRow>(
                "INSERT INTO leave_ledger_entries (
                    id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at
                 ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                 RETURNING id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at, created_at",
            )
            .bind(id)
            .bind(entry.user_id)
            .bind(entry.leave_type)
            .bind(kind_to_str(entry.kind))
            .bind(lot_id)
            .bind(amount_minutes)
            .bind(day_equivalent_minutes)
            .bind(entry.granted_at)
            .bind(entry.expires_at)
            .bind(entry.grant_base_date)
            .bind(entry.leave_request_id)
            .bind(entry.reason)
            .bind(entry.created_by)
            .bind(entry.effective_at)
            .fetch_one(&mut *tx)
            .await
            .map_err(repository_error)?;
            stored.push(row_to_stored_entry(row)?);
        }
        tx.commit().await.map_err(repository_error)?;
        Ok(stored)
    }
}

#[async_trait]
impl LeaveRuleRepository for LeaveLedgerPostgresRepository {
    async fn grant_rules(
        &self,
        leave_type: &str,
        on: NaiveDate,
    ) -> Result<Vec<LeaveGrantRule>, LeaveLedgerError> {
        sqlx::query_as::<_, GrantRuleRow>(
            "SELECT tenure_months, granted_days, expiry_months, day_equivalent_minutes
             FROM leave_grant_rules
             WHERE leave_type = $1
               AND accrual_kind = 'standard'
               AND effective_from = (
                   SELECT MAX(effective_from)
                   FROM leave_grant_rules
                   WHERE leave_type = $1
                     AND accrual_kind = 'standard'
                     AND effective_from <= $2
               )
             ORDER BY tenure_months ASC",
        )
        .bind(leave_type)
        .bind(on)
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)
        .map(|rows| {
            rows.into_iter()
                .map(|row| LeaveGrantRule {
                    tenure_months: i64::from(row.tenure_months),
                    granted_days: i64::from(row.granted_days),
                    expiry_months: i64::from(row.expiry_months),
                    day_equivalent_minutes: i64::from(row.day_equivalent_minutes),
                })
                .collect()
        })
    }

    async fn obligation_rule(
        &self,
        leave_type: &str,
        on: NaiveDate,
    ) -> Result<Option<LeaveObligationRule>, LeaveLedgerError> {
        sqlx::query_as::<_, ObligationRuleRow>(
            "SELECT minimum_granted_days, required_days, window_months, warning_lead_days
             FROM leave_obligation_rules
             WHERE leave_type = $1
               AND effective_from <= $2
             ORDER BY effective_from DESC
             LIMIT 1",
        )
        .bind(leave_type)
        .bind(on)
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)
        .map(|row| {
            row.map(|row| LeaveObligationRule {
                minimum_granted_days: i64::from(row.minimum_granted_days),
                required_days: i64::from(row.required_days),
                window_months: i64::from(row.window_months),
                warning_lead_days: i64::from(row.warning_lead_days),
            })
        })
    }
}

#[async_trait]
impl LeaveGrantUserRepository for LeaveLedgerPostgresRepository {
    async fn list_candidates(
        &self,
        user_ids: Option<&[String]>,
    ) -> Result<Vec<GrantCandidate>, LeaveLedgerError> {
        let rows = match user_ids {
            Some(ids) => {
                sqlx::query_as::<_, GrantCandidateRow>(
                    "SELECT id, hire_date
                     FROM users
                     WHERE id = ANY($1)
                     ORDER BY id",
                )
                .bind(ids)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, GrantCandidateRow>(
                    "SELECT id, hire_date
                     FROM users
                     ORDER BY id",
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(repository_error)?;
        Ok(rows
            .into_iter()
            .map(|row| GrantCandidate {
                user_id: row.id,
                hire_date: row.hire_date,
            })
            .collect())
    }

    async fn set_hire_date(
        &self,
        user_id: &str,
        hire_date: NaiveDate,
    ) -> Result<(), LeaveLedgerError> {
        let affected =
            sqlx::query("UPDATE users SET hire_date = $1, updated_at = NOW() WHERE id = $2")
                .bind(hire_date)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(repository_error)?
                .rows_affected();
        if affected == 0 {
            return Err(LeaveLedgerError::UserNotFound);
        }
        Ok(())
    }
}

fn row_to_stored_entry(
    row: LeaveLedgerEntryRow,
) -> Result<StoredLeaveLedgerEntry, LeaveLedgerError> {
    Ok(StoredLeaveLedgerEntry {
        id: row.id.to_string(),
        user_id: row.user_id,
        leave_type: row.leave_type,
        kind: kind_from_str(&row.kind)?,
        lot_id: row.lot_id.to_string(),
        amount_minutes: i64::from(row.amount_minutes),
        day_equivalent_minutes: i64::from(row.day_equivalent_minutes),
        granted_at: row.granted_at,
        expires_at: row.expires_at,
        grant_base_date: row.grant_base_date,
        leave_request_id: row.leave_request_id,
        reason: row.reason,
        created_by: row.created_by,
        effective_at: row.effective_at,
        created_at: row.created_at,
    })
}

fn kind_to_str(kind: LeaveLedgerKind) -> &'static str {
    match kind {
        LeaveLedgerKind::Grant => "grant",
        LeaveLedgerKind::Consume => "consume",
        LeaveLedgerKind::Release => "release",
        LeaveLedgerKind::Expire => "expire",
        LeaveLedgerKind::Adjust => "adjust",
    }
}

fn kind_from_str(kind: &str) -> Result<LeaveLedgerKind, LeaveLedgerError> {
    match kind {
        "grant" => Ok(LeaveLedgerKind::Grant),
        "consume" => Ok(LeaveLedgerKind::Consume),
        "release" => Ok(LeaveLedgerKind::Release),
        "expire" => Ok(LeaveLedgerKind::Expire),
        "adjust" => Ok(LeaveLedgerKind::Adjust),
        other => Err(LeaveLedgerError::Repository(format!(
            "unknown leave ledger kind: {other}"
        ))),
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, LeaveLedgerError> {
    Uuid::parse_str(value)
        .map_err(|_| LeaveLedgerError::InvalidInput(format!("{field} must be a UUID")))
}

fn repository_error(error: sqlx::Error) -> LeaveLedgerError {
    LeaveLedgerError::Repository(error.to_string())
}
