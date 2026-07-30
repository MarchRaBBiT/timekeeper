use std::str::FromStr;

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use chrono::Utc;
use timekeeper_app::leave_ledger::{
    AdjustLeaveLedger, AdjustLeaveLedgerCommand, GetLeaveBalance, GetLeaveBalanceCommand,
    GrantSkipReason, RunLeaveGrants, RunLeaveGrantsCommand, SetHireDate,
};
use timekeeper_contract::leave::{
    HireDateResponse, LeaveBalanceQuery, LeaveBalanceResponse, LeaveExpiryBackfillResponse,
    LeaveGrantResultResponse, LeaveGrantRunRequest, LeaveGrantRunResponse, LeaveGrantSkipReason,
    LeaveGrantSkipResponse, LeaveLedgerAdjustRequest, LeaveLedgerAdjustResponse,
    SetHireDateRequest,
};
use timekeeper_infra_postgres::leave_ledger::LeaveLedgerPostgresRepository;
use validator::Validate;

use crate::{
    error::{leave_ledger::leave_ledger_error_to_app_error, AppError},
    handlers::leave_ledger::{balance_view_to_response, entry_to_response, minutes_to_days},
    models::user::User,
    repositories::department::can_manager_approve,
    state::AppState,
    types::UserId,
};

pub async fn get_user_leave_balance(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Path(user_id): Path<String>,
    Query(query): Query<LeaveBalanceQuery>,
) -> Result<Json<LeaveBalanceResponse>, AppError> {
    let target_user_id =
        UserId::from_str(&user_id).map_err(|_| AppError::BadRequest("Invalid user_id".into()))?;
    authorize_balance_read(&state, &actor, target_user_id).await?;

    let as_of = query.as_of.unwrap_or_else(|| Utc::now().date_naive());
    let repository = LeaveLedgerPostgresRepository::new(state.read_pool().clone());
    repository
        .ensure_balance_tracked_leave_type(&query.leave_type_code)
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    let use_case = GetLeaveBalance::new(repository.clone(), repository);
    let view = use_case
        .execute(GetLeaveBalanceCommand {
            user_id,
            leave_type_code: query.leave_type_code,
            as_of,
        })
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    Ok(Json(balance_view_to_response(view)))
}

pub async fn run_leave_grants(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Json(payload): Json<LeaveGrantRunRequest>,
) -> Result<Json<LeaveGrantRunResponse>, AppError> {
    // LOW: defense-in-depth. このハンドラは `system_admin_routes` の
    // `auth_system_admin` middleware にも認可を委ねているが、middleware の
    // 設定ミスやルート付け替えだけで認可が抜けないよう、ハンドラ側でも
    // 明示的に system admin を検証する。
    if !actor.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    repository
        .ensure_balance_tracked_leave_type(&payload.leave_type_code)
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    let command = RunLeaveGrantsCommand {
        leave_type_code: payload.leave_type_code.clone(),
        base_date: payload.base_date,
        dry_run: payload.dry_run,
        user_ids: payload.user_ids,
        exclude_user_ids: payload.exclude_user_ids.unwrap_or_default(),
        created_by: Some(actor.id.to_string()),
        now: Utc::now(),
    };

    // M-1a: 付与バッチの二重起動を leave_grant_batch_lock の claim/release で
    // 排他する。個々のユーザーの正しさ（負残高・二重付与の防止）は use case
    // 内部の users 行ロックが担保するため、この排他はあくまで運用上の重複
    // 実行防止。dry_run は書き込みが無い read-only 経路なので排他しない
    // （実行中バッチがあってもプレビューは返せる）。
    let actor_id = actor.id.to_string();
    let batch_repository = repository.clone();
    let run = move || async move {
        let use_case = RunLeaveGrants::new(
            batch_repository.clone(),
            batch_repository.clone(),
            batch_repository,
        );
        use_case.execute(command).await
    };
    let report = if payload.dry_run {
        run().await
    } else {
        repository
            .run_grant_batch_exclusive(Some(&actor_id), run)
            .await
    }
    .map_err(leave_ledger_error_to_app_error)?;

    // M-1a: ユーザー単位トランザクションが失敗しても他ユーザーの処理は継続する。
    // 失敗は既存の API 契約（LeaveGrantRunResponse）に含めず、運用フォロー用に
    // ログへ残す。失敗したユーザーは同じ base_date で再実行すれば再度対象になる。
    for failure in &report.failed {
        tracing::error!(
            user_id = %failure.user_id,
            base_date = %report.base_date,
            error = %failure.error,
            "leave grant run: per-user ledger write failed; other users were unaffected and this user can be retried"
        );
    }

    Ok(Json(LeaveGrantRunResponse {
        base_date: report.base_date,
        dry_run: report.dry_run,
        granted: report
            .granted
            .into_iter()
            .map(|outcome| LeaveGrantResultResponse {
                user_id: outcome.user_id,
                lot_id: outcome.lot_id,
                tenure_months: outcome.tenure_months,
                granted_minutes: outcome.granted_minutes,
                granted_days: outcome.granted_minutes / outcome.day_equivalent_minutes,
                day_equivalent_minutes: outcome.day_equivalent_minutes,
                granted_at: outcome.granted_at,
                expires_at: outcome.expires_at,
            })
            .collect(),
        skipped: report
            .skipped
            .into_iter()
            .map(|(user_id, reason)| LeaveGrantSkipResponse {
                user_id,
                reason: grant_skip_reason_to_contract(reason),
            })
            .collect(),
        expired: report
            .expired
            .into_iter()
            .map(|expired| LeaveExpiryBackfillResponse {
                user_id: expired.user_id,
                lot_id: expired.lot_id,
                amount_minutes: expired.amount_minutes,
                expires_at: expired.expires_at,
            })
            .collect(),
    }))
}

pub async fn adjust_leave_ledger(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Json(payload): Json<LeaveLedgerAdjustRequest>,
) -> Result<Json<LeaveLedgerAdjustResponse>, AppError> {
    // LOW: defense-in-depth（run_leave_grants と同様の理由）。
    if !actor.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    repository
        .ensure_balance_tracked_leave_type(&payload.leave_type_code)
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    let use_case = AdjustLeaveLedger::new(repository.clone(), repository);
    let report = use_case
        .execute(AdjustLeaveLedgerCommand {
            user_id: payload.user_id,
            leave_type_code: payload.leave_type_code,
            amount_minutes: payload.amount_minutes,
            lot_id: payload.lot_id,
            day_equivalent_minutes: payload.day_equivalent_minutes,
            granted_at: payload.granted_at,
            expires_at: payload.expires_at,
            grant_base_date: payload.grant_base_date,
            reason: payload.reason,
            created_by: Some(actor.id.to_string()),
            dry_run: payload.dry_run,
            now: Utc::now(),
        })
        .await
        .map_err(leave_ledger_error_to_app_error)?;

    let balance_after_days = report
        .balance_after
        .active_lots()
        .map(|lot| minutes_to_days(lot.remaining_minutes, lot.day_equivalent_minutes))
        .sum();
    Ok(Json(LeaveLedgerAdjustResponse {
        dry_run: report.dry_run,
        entry: report.entry.map(entry_to_response),
        balance_after_minutes: report.balance_after.available_minutes,
        balance_after_days,
    }))
}

pub async fn set_user_hire_date(
    Extension(state): Extension<AppState>,
    Extension(actor): Extension<User>,
    Path(user_id): Path<String>,
    Json(payload): Json<SetHireDateRequest>,
) -> Result<Json<HireDateResponse>, AppError> {
    // LOW: defense-in-depth（run_leave_grants と同様の理由）。このハンドラは
    // これまで actor を受け取っておらず、system_admin_routes の middleware
    // だけに認可を委ねていた。
    if !actor.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }
    payload.validate()?;
    let repository = LeaveLedgerPostgresRepository::new(state.write_pool.clone());
    let use_case = SetHireDate::new(repository);
    use_case
        .execute(&user_id, payload.hire_date)
        .await
        .map_err(leave_ledger_error_to_app_error)?;
    Ok(Json(HireDateResponse {
        user_id,
        hire_date: payload.hire_date,
    }))
}

async fn authorize_balance_read(
    state: &AppState,
    actor: &User,
    target_user_id: UserId,
) -> Result<(), AppError> {
    if actor.is_system_admin() {
        return Ok(());
    }
    if actor.is_manager()
        && can_manager_approve(state.read_pool(), actor.id, target_user_id)
            .await
            .map_err(|error| AppError::InternalServerError(error.into()))?
    {
        return Ok(());
    }
    Err(AppError::Forbidden("Forbidden".into()))
}

fn grant_skip_reason_to_contract(reason: GrantSkipReason) -> LeaveGrantSkipReason {
    match reason {
        GrantSkipReason::HireDateNotSet => LeaveGrantSkipReason::HireDateNotSet,
        GrantSkipReason::NotDue => LeaveGrantSkipReason::NotDue,
        GrantSkipReason::AlreadyGranted => LeaveGrantSkipReason::AlreadyGranted,
        GrantSkipReason::NoMatchingRule => LeaveGrantSkipReason::NoMatchingRule,
        GrantSkipReason::Excluded => LeaveGrantSkipReason::Excluded,
    }
}
