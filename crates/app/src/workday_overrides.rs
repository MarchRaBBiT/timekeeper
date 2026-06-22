use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use thiserror::Error;

use crate::work_schedules::WorkdayOverrideKind;

/// 永続化済みの日別例外。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWorkdayOverride {
    pub id: String,
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: WorkdayOverrideKind,
    pub work_schedule_id: Option<String>,
    pub reason: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 日別例外のupsert入力。reason等は use case で正規化・検証済み。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewWorkdayOverride {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: WorkdayOverrideKind,
    pub work_schedule_id: Option<String>,
    pub reason: String,
    pub created_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetWorkdayOverrideCommand {
    pub user_id: String,
    pub work_date: NaiveDate,
    pub kind: WorkdayOverrideKind,
    pub work_schedule_id: Option<String>,
    pub reason: String,
    pub created_by: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WorkdayOverrideError {
    #[error("invalid workday override input: {0}")]
    InvalidInput(String),
    #[error("workday override not found")]
    NotFound,
    #[error("resolved workday is locked")]
    ResolvedWorkdayLocked,
    #[error("workday override repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait WorkdayOverrideRepository: Send + Sync {
    /// 対象日に locked な resolved workday が存在するかを返す。
    async fn is_resolved_locked(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError>;

    async fn upsert_override(
        &self,
        input: NewWorkdayOverride,
    ) -> Result<StoredWorkdayOverride, WorkdayOverrideError>;

    /// 例外を削除する。削除対象が存在した場合に true を返す。
    async fn delete_override(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<bool, WorkdayOverrideError>;
}

const MAX_REASON_CHARS: usize = 500;

#[derive(Debug, Clone)]
pub struct SetWorkdayOverride<R> {
    repository: R,
}

impl<R> SetWorkdayOverride<R>
where
    R: WorkdayOverrideRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: SetWorkdayOverrideCommand,
    ) -> Result<StoredWorkdayOverride, WorkdayOverrideError> {
        let reason = command.reason.trim();
        if reason.is_empty() || reason.chars().count() > MAX_REASON_CHARS {
            return Err(WorkdayOverrideError::InvalidInput(
                "reason must be between 1 and 500 characters".to_string(),
            ));
        }
        match command.kind {
            WorkdayOverrideKind::UseSchedule if command.work_schedule_id.is_none() => {
                return Err(WorkdayOverrideError::InvalidInput(
                    "use_schedule override requires work_schedule_id".to_string(),
                ));
            }
            WorkdayOverrideKind::NonWorkingDay if command.work_schedule_id.is_some() => {
                return Err(WorkdayOverrideError::InvalidInput(
                    "non_working_day override must not set work_schedule_id".to_string(),
                ));
            }
            _ => {}
        }
        if self
            .repository
            .is_resolved_locked(&command.user_id, command.work_date)
            .await?
        {
            return Err(WorkdayOverrideError::ResolvedWorkdayLocked);
        }
        self.repository
            .upsert_override(NewWorkdayOverride {
                user_id: command.user_id,
                work_date: command.work_date,
                kind: command.kind,
                work_schedule_id: command.work_schedule_id,
                reason: reason.to_string(),
                created_by: command.created_by,
            })
            .await
    }
}

#[derive(Debug, Clone)]
pub struct DeleteWorkdayOverride<R> {
    repository: R,
}

impl<R> DeleteWorkdayOverride<R>
where
    R: WorkdayOverrideRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        user_id: &str,
        work_date: NaiveDate,
    ) -> Result<(), WorkdayOverrideError> {
        if self
            .repository
            .is_resolved_locked(user_id, work_date)
            .await?
        {
            return Err(WorkdayOverrideError::ResolvedWorkdayLocked);
        }
        if self.repository.delete_override(user_id, work_date).await? {
            Ok(())
        } else {
            Err(WorkdayOverrideError::NotFound)
        }
    }
}
