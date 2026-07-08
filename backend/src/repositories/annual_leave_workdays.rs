//! 年次有給休暇の消化計算に使う稼働日数の解決（H-1 修正）。
//!
//! docs/design-docs/leave-entitlement.md の Consumption Target Days 決定に従い、
//! 消化対象日は resolved workday が稼働日（`ScheduledWorkday`）と判定した日のみに
//! 限定する。稼働日判定は既存の `ResolveWorkday` use case（T-16 の resolved
//! workday）をそのまま再利用し、消化計算専用の曜日判定は実装しない。
//!
//! 勤務予定を解決できない日が期間に含まれる場合は fail-closed（エラーで拒否）と
//! し、暦日フォールバックはしない。申請作成時（残高検証）・承認時（消化確定）の
//! 両方から同じ関数を呼び、稼働日数の計算を乖離させない。

use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use timekeeper_app::user_workdays::{
    ListUserWorkdays, ListUserWorkdaysCommand, ListUserWorkdaysError,
};
use timekeeper_app::work_schedules::{
    ResolveWorkday, ResolveWorkdayCommand, ResolveWorkdayError, ResolvedDayKind,
};
use timekeeper_contract::leave::LEAVE_REQUEST_SCHEDULE_UNRESOLVED_CODE;
use timekeeper_infra_postgres::work_schedules::WorkdayResolverPostgresRepository;

use crate::error::AppError;

/// `[from, to]`（両端含む）の期間について、resolved workday を解決したうえで
/// 稼働日（`ScheduledWorkday`）の件数を返す。
///
/// 期間内のいずれかの日で勤務予定を解決できない場合は
/// `LEAVE_REQUEST_SCHEDULE_UNRESOLVED` の 422 エラーを返す（fail-closed）。
pub async fn count_working_days_for_annual_leave(
    db: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<i64, AppError> {
    materialize_resolved_workdays_fail_closed(db, user_id, from, to).await?;

    let repository = WorkdayResolverPostgresRepository::new(db.clone());
    let use_case = ListUserWorkdays::new(repository);
    let workdays = use_case
        .execute(ListUserWorkdaysCommand {
            user_id: user_id.to_string(),
            from,
            to,
        })
        .await
        .map_err(list_workdays_error_to_app_error)?;

    let expected_days = (to - from).num_days() + 1;
    if workdays.len() as i64 != expected_days {
        // 解決直後に読み直して件数が合わない場合は、想定外のデータ不整合として
        // 暦日フォールバックせずに拒否する。
        return Err(schedule_unresolved_error());
    }

    Ok(workdays
        .iter()
        .filter(|day| day.day_kind == ResolvedDayKind::ScheduledWorkday)
        .count() as i64)
}

async fn materialize_resolved_workdays_fail_closed(
    db: &PgPool,
    user_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(), AppError> {
    let repository = WorkdayResolverPostgresRepository::new(db.clone());
    let resolver = ResolveWorkday::new(repository.clone(), repository.clone(), repository);
    let resolved_at = Utc::now();
    let mut current = from;
    while current <= to {
        resolver
            .execute(ResolveWorkdayCommand {
                user_id: user_id.to_string(),
                work_date: current,
                resolved_at,
            })
            .await
            .map_err(resolve_workday_error_to_app_error)?;
        current = current
            .succ_opt()
            .ok_or_else(|| AppError::InternalServerError(anyhow::anyhow!("date overflow")))?;
    }
    Ok(())
}

fn schedule_unresolved_error() -> AppError {
    AppError::UnprocessableEntityWithCode {
        message: "could not resolve a work schedule for every day in the requested leave period"
            .to_string(),
        code: LEAVE_REQUEST_SCHEDULE_UNRESOLVED_CODE.to_string(),
    }
}

fn resolve_workday_error_to_app_error(error: ResolveWorkdayError) -> AppError {
    match error {
        ResolveWorkdayError::WorkScheduleNotConfigured => schedule_unresolved_error(),
        ResolveWorkdayError::InvalidScheduleData(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
        ResolveWorkdayError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}

fn list_workdays_error_to_app_error(error: ListUserWorkdaysError) -> AppError {
    match error {
        ListUserWorkdaysError::InvalidRange => {
            AppError::BadRequest("start_date must be <= end_date".to_string())
        }
        ListUserWorkdaysError::RangeTooLarge => AppError::BadRequest(
            "requested leave period exceeds the supported maximum range".to_string(),
        ),
        ListUserWorkdaysError::Repository(message) => {
            AppError::InternalServerError(anyhow::anyhow!(message))
        }
    }
}
