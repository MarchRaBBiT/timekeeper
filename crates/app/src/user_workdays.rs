use async_trait::async_trait;
use chrono::NaiveDate;
use thiserror::Error;

use crate::work_schedules::ResolvedWorkday;

/// 従業員またはマネージャーが範囲指定で解決済み勤務日を閲覧するためのコマンド。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUserWorkdaysCommand {
    pub user_id: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ListUserWorkdaysError {
    #[error("from must be on or before to")]
    InvalidRange,
    #[error("requested range exceeds the supported maximum")]
    RangeTooLarge,
    #[error("workday read repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait UserWorkdayReadRepository: Send + Sync {
    async fn list_resolved_in_range(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, ListUserWorkdaysError>;
}

/// 1リクエストで返す範囲の上限（両端含む）。
pub const MAX_WORKDAY_RANGE_DAYS: i64 = 366;

#[derive(Debug, Clone)]
pub struct ListUserWorkdays<R> {
    repository: R,
}

impl<R> ListUserWorkdays<R>
where
    R: UserWorkdayReadRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: ListUserWorkdaysCommand,
    ) -> Result<Vec<ResolvedWorkday>, ListUserWorkdaysError> {
        if command.from > command.to {
            return Err(ListUserWorkdaysError::InvalidRange);
        }
        if (command.to - command.from).num_days() >= MAX_WORKDAY_RANGE_DAYS {
            return Err(ListUserWorkdaysError::RangeTooLarge);
        }
        self.repository
            .list_resolved_in_range(&command.user_id, command.from, command.to)
            .await
    }
}
