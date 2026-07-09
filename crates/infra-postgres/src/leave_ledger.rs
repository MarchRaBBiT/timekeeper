use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use std::future::Future;
use timekeeper_app::leave_ledger::{
    GrantCandidate, LeaveGrantUserRepository, LeaveLedgerError, LeaveLedgerRepository,
    LeaveRuleRepository, LedgerLockFuture, NewLeaveLedgerEntry, StoredLeaveLedgerEntry,
};
use timekeeper_domain::leave_ledger::{LeaveGrantRule, LeaveLedgerKind, LeaveObligationRule};
use uuid::Uuid;

/// 有給付与バッチ（`RunLeaveGrants`）の claim が stale と見なされ、次の実行が
/// 再取得できるようになるまでの秒数（M-1a のクラッシュ回復）。
///
/// プロセスが claim と release の間でクラッシュしても、この秒数を過ぎれば
/// 次のバッチ実行が claim を奪い直せるため、手動復旧は不要。
/// バッチはユーザー単位トランザクションに分離しているため、この排他は
/// 「同時に 2 つの付与バッチを走らせない」という運用上の重複実行防止であり、
/// 残高の正しさ自体は `with_user_lock`（users 行 FOR UPDATE）が担保する。
/// したがって stale 判定が早すぎて 2 つのバッチが重なっても残高は壊れない
/// （二重付与は AlreadyGranted 判定と uq_leave_ledger_grant_base で防がれる）。
pub const LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS: i64 = 600;

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

    /// 付与バッチの二重起動を排他して `job` を実行する（M-1a）。
    ///
    /// 排他は `leave_grant_batch_lock` の単一行を claim/release する方式。
    /// session-level `pg_advisory_lock` はロックを保持するコネクションを
    /// バッチ実行中ずっと専有する必要があり、`job` 内部が同じ pool から
    /// ユーザー単位トランザクションを開くため、小さい pool では自己
    /// デッドロックする（`pg_advisory_xact_lock` はバッチ全体を覆う単一
    /// トランザクションが存在しないため使えない）。claim/release は
    /// 取得・解放とも短い独立クエリなのでコネクションを専有しない。
    ///
    /// クラッシュ回復: claim したまま release されなかった場合、
    /// [`LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS`] を過ぎた claim は次の実行が
    /// 奪い直す。`claim_token` により、stale 判定で奪われた古いバッチが
    /// 新しい claim を誤って release することはない。
    ///
    /// 実行中の claim が存在する間は `LeaveLedgerError::BatchAlreadyRunning`
    /// を即時に返す（ブロック待ちしない）。
    pub async fn run_grant_batch_exclusive<F, Fut, T>(
        &self,
        started_by: Option<&str>,
        job: F,
    ) -> Result<T, LeaveLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, LeaveLedgerError>>,
    {
        let claim_token = Uuid::new_v4();
        self.claim_grant_batch_lock(claim_token, started_by).await?;
        let result = job().await;
        if let Err(release_error) = self.release_grant_batch_lock(claim_token).await {
            // release 失敗は stale timeout（LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS）
            // で自動回復する。バッチ本体の結果を壊さないため、ここではログに
            // 残すだけにする（silent swallow ではなく明示的に記録する）。
            tracing::error!(
                error = %release_error,
                stale_after_seconds = LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS,
                "failed to release leave grant batch lock; it will expire automatically"
            );
        }
        result
    }

    async fn claim_grant_batch_lock(
        &self,
        claim_token: Uuid,
        started_by: Option<&str>,
    ) -> Result<(), LeaveLedgerError> {
        let claimed = sqlx::query(
            "UPDATE leave_grant_batch_lock
             SET running = TRUE, claim_token = $1, started_at = NOW(), started_by = $2
             WHERE id = 1
               AND (running = FALSE
                    OR started_at IS NULL
                    OR started_at < NOW() - make_interval(secs => $3))",
        )
        .bind(claim_token)
        .bind(started_by)
        .bind(LEAVE_GRANT_BATCH_STALE_AFTER_SECONDS as f64)
        .execute(&self.pool)
        .await
        .map_err(repository_error)?
        .rows_affected();
        if claimed == 0 {
            return Err(LeaveLedgerError::BatchAlreadyRunning);
        }
        Ok(())
    }

    async fn release_grant_batch_lock(&self, claim_token: Uuid) -> Result<(), LeaveLedgerError> {
        sqlx::query(
            "UPDATE leave_grant_batch_lock
             SET running = FALSE, claim_token = NULL, started_at = NULL, started_by = NULL
             WHERE id = 1 AND claim_token = $1",
        )
        .bind(claim_token)
        .execute(&self.pool)
        .await
        .map_err(repository_error)?;
        Ok(())
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

    async fn list_entries_for_users(
        &self,
        user_ids: &[String],
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, LeaveLedgerEntryRow>(
            "SELECT id, user_id, leave_type, kind, lot_id, amount_minutes,
                    day_equivalent_minutes, granted_at, expires_at, grant_base_date,
                    leave_request_id, reason, created_by, effective_at, created_at
             FROM leave_ledger_entries
             WHERE user_id = ANY($1) AND leave_type = $2
             ORDER BY user_id ASC, effective_at ASC, created_at ASC, id ASC",
        )
        .bind(user_ids)
        .bind(leave_type)
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)?
        .into_iter()
        .map(row_to_stored_entry)
        .collect()
    }

    fn with_user_lock<'a, F, T>(
        &'a self,
        user_id: &'a str,
        leave_type: &'a str,
        compute: F,
    ) -> LedgerLockFuture<'a, (Vec<StoredLeaveLedgerEntry>, T)>
    where
        F: FnOnce(
                &[StoredLeaveLedgerEntry],
            ) -> Result<(Vec<NewLeaveLedgerEntry>, T), LeaveLedgerError>
            + Send
            + 'a,
        T: Send + 'a,
    {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(repository_error)?;
            // H-2: users 行 → leave_ledger_entries 行の順でロックし、
            // backend/src/repositories/request.rs の承認・取消経路
            // (lock_user_for_ledger → list_entries_for_update) とロック順序を揃える
            // (デッドロック防止)。
            lock_user_row(&mut tx, user_id).await?;
            let entries = Self::list_entries_for_update(&mut tx, user_id, leave_type).await?;
            let (new_entries, value) = compute(&entries)?;
            let stored = if new_entries.is_empty() {
                Vec::new()
            } else {
                Self::append_entries_in_transaction(&mut tx, new_entries).await?
            };
            tx.commit().await.map_err(repository_error)?;
            Ok((stored, value))
        })
    }
}

async fn lock_user_row(
    tx: &mut Transaction<'_, Postgres>,
    user_id: &str,
) -> Result<(), LeaveLedgerError> {
    let found: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(repository_error)?;
    if found.is_none() {
        return Err(LeaveLedgerError::UserNotFound);
    }
    Ok(())
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
