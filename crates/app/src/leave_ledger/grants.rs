//! `RunLeaveGrants`: 管理者起動の付与実行（dry-run 付き）+ 冪等な expire 補記（T-04）。

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_domain::leave_ledger::{
    derive_balance, grant_expiry_date, pending_expirations, scheduled_grant_tenure_months,
    select_grant_rule, LeaveBalance, LeaveGrantRule, LeaveLedgerKind,
};

use super::{
    start_of_day_utc, stored_entries_to_events, GrantCandidate, LeaveGrantUserRepository,
    LeaveLedgerError, LeaveLedgerRepository, LeaveRuleRepository, NewLeaveLedgerEntry,
    StoredLeaveLedgerEntry, ANNUAL_LEAVE_TYPE,
};

// ---------------------------------------------------------------------------
// RunLeaveGrants
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLeaveGrantsCommand {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    /// None は全ユーザー。Some は明示リスト（存在しない ID は UserNotFound）。
    pub user_ids: Option<Vec<String>>,
    pub exclude_user_ids: Vec<String>,
    pub created_by: Option<String>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantSkipReason {
    HireDateNotSet,
    NotDue,
    AlreadyGranted,
    NoMatchingRule,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantOutcome {
    pub user_id: String,
    /// dry-run では未採番のため None。
    pub lot_id: Option<String>,
    pub tenure_months: i64,
    pub granted_minutes: i64,
    pub day_equivalent_minutes: i64,
    pub granted_at: NaiveDate,
    pub expires_at: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryOutcome {
    pub user_id: String,
    pub lot_id: String,
    pub amount_minutes: i64,
    pub expires_at: NaiveDate,
}

/// ユーザー単位トランザクションの書き込みが失敗した記録（M-1a）。
///
/// 既存の `LeaveGrantRunResponse` API 契約は変更しないため、この情報は
/// レスポンス JSON へはマッピングしない。呼び出し元（handler）で
/// `tracing` 等へログ出力し、運用上のフォローに使う。失敗したユーザーは
/// 冪等な再実行（同じ `base_date` での再実行）で再度対象になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFailure {
    pub user_id: String,
    pub error: LeaveLedgerError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLeaveGrantsReport {
    pub base_date: NaiveDate,
    pub dry_run: bool,
    pub granted: Vec<GrantOutcome>,
    pub skipped: Vec<(String, GrantSkipReason)>,
    pub expired: Vec<ExpiryOutcome>,
    pub failed: Vec<GrantFailure>,
}

#[derive(Debug, Clone)]
pub struct RunLeaveGrants<L, R, U> {
    ledger: L,
    rules: R,
    users: U,
}

impl<L, R, U> RunLeaveGrants<L, R, U>
where
    L: LeaveLedgerRepository,
    R: LeaveRuleRepository,
    U: LeaveGrantUserRepository,
{
    pub fn new(ledger: L, rules: R, users: U) -> Self {
        Self {
            ledger,
            rules,
            users,
        }
    }

    pub async fn execute(
        &self,
        command: RunLeaveGrantsCommand,
    ) -> Result<RunLeaveGrantsReport, LeaveLedgerError> {
        if let Some(ids) = &command.user_ids {
            if ids.is_empty() {
                return Err(LeaveLedgerError::InvalidInput(
                    "user_ids must not be empty when provided".to_string(),
                ));
            }
        }
        let rules = self
            .rules
            .grant_rules(ANNUAL_LEAVE_TYPE, command.base_date)
            .await?;
        if rules.is_empty() {
            return Err(LeaveLedgerError::RulesNotConfigured);
        }
        let candidates = self
            .users
            .list_candidates(command.user_ids.as_deref())
            .await?;
        if let Some(ids) = &command.user_ids {
            if candidates.len() != ids.len() {
                return Err(LeaveLedgerError::UserNotFound);
            }
        }

        let mut acc = GrantRunAccumulator::default();
        let mut active_candidates = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if command.exclude_user_ids.contains(&candidate.user_id) {
                acc.skipped
                    .push((candidate.user_id, GrantSkipReason::Excluded));
            } else {
                active_candidates.push(candidate);
            }
        }

        // M-1b: 候補者ごとの N+1 SELECT を避けるため、ロックなしで一括取得する。
        // これは事前絞り込み専用。実際に書き込むかどうかの判定は、必ず
        // with_user_lock で取得したロック済み entries で再検証する。
        let entries_by_user = self.bulk_entries_by_user(&active_candidates).await?;

        if command.dry_run {
            collect_dry_run(
                &active_candidates,
                &entries_by_user,
                &rules,
                &command,
                &mut acc,
            )?;
        } else {
            self.collect_with_locks(
                active_candidates,
                &entries_by_user,
                &rules,
                &command,
                &mut acc,
            )
            .await?;
        }

        Ok(RunLeaveGrantsReport {
            base_date: command.base_date,
            dry_run: command.dry_run,
            granted: acc.granted,
            skipped: acc.skipped,
            expired: acc.expired,
            failed: acc.failed,
        })
    }

    async fn bulk_entries_by_user(
        &self,
        candidates: &[GrantCandidate],
    ) -> Result<HashMap<String, Vec<StoredLeaveLedgerEntry>>, LeaveLedgerError> {
        let user_ids: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.user_id.clone())
            .collect();
        let bulk_entries = self
            .ledger
            .list_entries_for_users(&user_ids, ANNUAL_LEAVE_TYPE)
            .await?;
        let mut entries_by_user: HashMap<String, Vec<StoredLeaveLedgerEntry>> = HashMap::new();
        for entry in bulk_entries {
            entries_by_user
                .entry(entry.user_id.clone())
                .or_default()
                .push(entry);
        }
        Ok(entries_by_user)
    }

    /// H-2: 各ユーザーの読み取り〜書き込みを users 行ロック + 台帳行
    /// FOR UPDATE を取った単一トランザクションに閉じる（TOCTOU 防止）。
    /// M-1a: ユーザー単位でトランザクションを分離しているため、1 人の
    /// 失敗（unique 制約違反など）が他ユーザーの付与・expire 補記を
    /// ロールバックしない。
    async fn collect_with_locks(
        &self,
        candidates: Vec<GrantCandidate>,
        entries_by_user: &HashMap<String, Vec<StoredLeaveLedgerEntry>>,
        rules: &[LeaveGrantRule],
        command: &RunLeaveGrantsCommand,
        acc: &mut GrantRunAccumulator,
    ) -> Result<(), LeaveLedgerError> {
        for candidate in candidates {
            let entries = entries_by_user
                .get(&candidate.user_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let (preview, preview_entries) = evaluate_candidate(
                &candidate,
                entries,
                rules,
                command.base_date,
                &command.created_by,
            )?;
            if preview_entries.is_empty() {
                if let Some(reason) = preview.skip {
                    acc.skipped.push((candidate.user_id, reason));
                }
                continue;
            }

            match self.grant_with_lock(&candidate, rules, command).await {
                Ok((stored, outcome)) => {
                    acc.record(candidate.user_id, outcome, &stored);
                }
                // 系統的な設定不備（付与ルールの時効月数から日付を算出できない
                // 等）は全ユーザーに影響するため、バッチ全体を中断する。
                Err(LeaveLedgerError::InvalidInput(message)) => {
                    return Err(LeaveLedgerError::InvalidInput(message));
                }
                // それ以外（ロック/DB 起因のエラー）は当該ユーザーのみ
                // failed として記録し、他ユーザーの処理は継続する。
                Err(error) => {
                    acc.failed.push(GrantFailure {
                        user_id: candidate.user_id,
                        error,
                    });
                }
            }
        }
        Ok(())
    }

    /// ロック済み entries で判定し直してから同一トランザクションで追記する。
    async fn grant_with_lock(
        &self,
        candidate: &GrantCandidate,
        rules: &[LeaveGrantRule],
        command: &RunLeaveGrantsCommand,
    ) -> Result<(Vec<StoredLeaveLedgerEntry>, CandidateOutcome), LeaveLedgerError> {
        let candidate_for_lock = candidate.clone();
        let rules_for_lock = rules.to_vec();
        let created_by_for_lock = command.created_by.clone();
        let base_date = command.base_date;
        self.ledger
            .with_user_lock(
                &candidate.user_id,
                ANNUAL_LEAVE_TYPE,
                move |locked_entries| {
                    let (outcome, new_entries) = evaluate_candidate(
                        &candidate_for_lock,
                        locked_entries,
                        &rules_for_lock,
                        base_date,
                        &created_by_for_lock,
                    )?;
                    Ok((new_entries, outcome))
                },
            )
            .await
    }
}

/// dry-run: ロックなしの一括読み取りだけで結果を導出する（書き込みなし）。
fn collect_dry_run(
    candidates: &[GrantCandidate],
    entries_by_user: &HashMap<String, Vec<StoredLeaveLedgerEntry>>,
    rules: &[LeaveGrantRule],
    command: &RunLeaveGrantsCommand,
    acc: &mut GrantRunAccumulator,
) -> Result<(), LeaveLedgerError> {
    for candidate in candidates {
        let entries = entries_by_user
            .get(&candidate.user_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let (outcome, _new_entries) = evaluate_candidate(
            candidate,
            entries,
            rules,
            command.base_date,
            &command.created_by,
        )?;
        acc.record(candidate.user_id.clone(), outcome, &[]);
    }
    Ok(())
}

/// `RunLeaveGrants::execute` の集計バッファ。
#[derive(Default)]
struct GrantRunAccumulator {
    granted: Vec<GrantOutcome>,
    skipped: Vec<(String, GrantSkipReason)>,
    expired: Vec<ExpiryOutcome>,
    failed: Vec<GrantFailure>,
}

impl GrantRunAccumulator {
    /// 1 候補者分の判定結果を集計へ反映する。`stored` が非空なら末尾の entry
    /// （grant は必ず最後に追記される）から採番済み lot_id を補完する。
    fn record(
        &mut self,
        user_id: String,
        outcome: CandidateOutcome,
        stored: &[StoredLeaveLedgerEntry],
    ) {
        self.expired.extend(outcome.expired);
        match outcome.grant {
            Some(mut grant) => {
                if let Some(entry) = stored.last() {
                    grant.lot_id = Some(entry.lot_id.clone());
                }
                self.granted.push(grant);
            }
            None => {
                if let Some(reason) = outcome.skip {
                    self.skipped.push((user_id, reason));
                }
            }
        }
    }
}

/// 1 候補者分の付与可否と冪等 expire 補記を判定する（純ロジック）。
///
/// `entries` は呼び出し元がロック付き／ロックなしのどちらで取得したかを
/// 問わない。書き込みを伴う判定にこの関数を使う場合、呼び出し元が
/// `with_user_lock` 等でトランザクション境界を保証すること。
fn evaluate_candidate(
    candidate: &GrantCandidate,
    entries: &[StoredLeaveLedgerEntry],
    rules: &[LeaveGrantRule],
    base_date: NaiveDate,
    created_by: &Option<String>,
) -> Result<(CandidateOutcome, Vec<NewLeaveLedgerEntry>), LeaveLedgerError> {
    let events = stored_entries_to_events(entries);
    let balance = derive_balance(&events, base_date);
    let (expired, mut new_entries) = expiration_entries(candidate, &balance, created_by);

    let Some(hire_date) = candidate.hire_date else {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::HireDateNotSet,
        ));
    };
    let Some(tenure_months) = scheduled_grant_tenure_months(hire_date, base_date) else {
        return Ok(skip_outcome(expired, new_entries, GrantSkipReason::NotDue));
    };
    let already_granted = entries.iter().any(|entry| {
        entry.kind == LeaveLedgerKind::Grant && entry.grant_base_date == Some(base_date)
    });
    if already_granted {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::AlreadyGranted,
        ));
    }
    let Some(rule) = select_grant_rule(rules, tenure_months) else {
        return Ok(skip_outcome(
            expired,
            new_entries,
            GrantSkipReason::NoMatchingRule,
        ));
    };
    let Some(expires_at) = grant_expiry_date(base_date, rule.expiry_months) else {
        return Err(LeaveLedgerError::InvalidInput(format!(
            "cannot compute expiry date for base date {base_date}"
        )));
    };

    let granted_minutes = rule.granted_days * rule.day_equivalent_minutes;
    new_entries.push(NewLeaveLedgerEntry {
        user_id: candidate.user_id.clone(),
        leave_type: ANNUAL_LEAVE_TYPE.to_string(),
        kind: LeaveLedgerKind::Grant,
        lot_id: None,
        amount_minutes: granted_minutes,
        day_equivalent_minutes: rule.day_equivalent_minutes,
        granted_at: Some(base_date),
        expires_at: Some(expires_at),
        grant_base_date: Some(base_date),
        leave_request_id: None,
        reason: None,
        created_by: created_by.clone(),
        effective_at: start_of_day_utc(base_date),
    });
    let grant = GrantOutcome {
        user_id: candidate.user_id.clone(),
        lot_id: None,
        tenure_months,
        granted_minutes,
        day_equivalent_minutes: rule.day_equivalent_minutes,
        granted_at: base_date,
        expires_at,
    };
    Ok((
        CandidateOutcome {
            expired,
            grant: Some(grant),
            skip: None,
        },
        new_entries,
    ))
}

struct CandidateOutcome {
    expired: Vec<ExpiryOutcome>,
    grant: Option<GrantOutcome>,
    skip: Option<GrantSkipReason>,
}

fn skip_outcome(
    expired: Vec<ExpiryOutcome>,
    new_entries: Vec<NewLeaveLedgerEntry>,
    reason: GrantSkipReason,
) -> (CandidateOutcome, Vec<NewLeaveLedgerEntry>) {
    (
        CandidateOutcome {
            expired,
            grant: None,
            skip: Some(reason),
        },
        new_entries,
    )
}

/// 冪等な expire 補記（付与可否に関わらず全対象者に対して行う）。
fn expiration_entries(
    candidate: &GrantCandidate,
    balance: &LeaveBalance,
    created_by: &Option<String>,
) -> (Vec<ExpiryOutcome>, Vec<NewLeaveLedgerEntry>) {
    let mut expired = Vec::new();
    let mut new_entries = Vec::new();
    for draft in pending_expirations(balance) {
        expired.push(ExpiryOutcome {
            user_id: candidate.user_id.clone(),
            lot_id: draft.lot_id.clone(),
            amount_minutes: draft.amount_minutes,
            expires_at: draft.expires_at,
        });
        new_entries.push(NewLeaveLedgerEntry {
            user_id: candidate.user_id.clone(),
            leave_type: ANNUAL_LEAVE_TYPE.to_string(),
            kind: LeaveLedgerKind::Expire,
            lot_id: Some(draft.lot_id),
            amount_minutes: draft.amount_minutes,
            day_equivalent_minutes: draft.day_equivalent_minutes,
            granted_at: None,
            expires_at: None,
            grant_base_date: None,
            leave_request_id: None,
            reason: None,
            created_by: created_by.clone(),
            effective_at: start_of_day_utc(draft.expires_at),
        });
    }
    (expired, new_entries)
}
