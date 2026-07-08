use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use timekeeper_app::leave_ledger::{
    AdjustLeaveLedger, AdjustLeaveLedgerCommand, GetLeaveBalance, GetLeaveBalanceCommand,
    GrantCandidate, GrantSkipReason, LeaveGrantUserRepository, LeaveLedgerError,
    LeaveLedgerRepository, LeaveRuleRepository, NewLeaveLedgerEntry, RunLeaveGrants,
    RunLeaveGrantsCommand, SetHireDate, StoredLeaveLedgerEntry, ANNUAL_LEAVE_TYPE,
};
use timekeeper_domain::leave_ledger::{
    LeaveGrantRule, LeaveLedgerKind, LeaveObligationRule, ObligationStatus,
};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

fn ts(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("valid timestamp")
}

#[derive(Default)]
struct FakeLedger {
    entries: Mutex<Vec<StoredLeaveLedgerEntry>>,
    sequence: AtomicUsize,
    fail_append: bool,
}

impl FakeLedger {
    fn seeded(entries: Vec<StoredLeaveLedgerEntry>) -> Self {
        Self {
            entries: Mutex::new(entries),
            sequence: AtomicUsize::new(1000),
            fail_append: false,
        }
    }

    fn stored(&self) -> Vec<StoredLeaveLedgerEntry> {
        self.entries.lock().expect("entries lock").clone()
    }
}

#[async_trait::async_trait]
impl LeaveLedgerRepository for &FakeLedger {
    async fn list_entries(
        &self,
        user_id: &str,
        leave_type: &str,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        Ok(self
            .entries
            .lock()
            .expect("entries lock")
            .iter()
            .filter(|entry| entry.user_id == user_id && entry.leave_type == leave_type)
            .cloned()
            .collect())
    }

    async fn append_entries(
        &self,
        entries: Vec<NewLeaveLedgerEntry>,
    ) -> Result<Vec<StoredLeaveLedgerEntry>, LeaveLedgerError> {
        if self.fail_append {
            return Err(LeaveLedgerError::Repository("append failed".to_string()));
        }
        let mut stored_entries = self.entries.lock().expect("entries lock");
        let mut appended = Vec::new();
        for entry in entries {
            let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
            let stored = StoredLeaveLedgerEntry {
                id: format!("entry-{sequence}"),
                user_id: entry.user_id,
                leave_type: entry.leave_type,
                kind: entry.kind,
                lot_id: entry.lot_id.unwrap_or_else(|| format!("lot-{sequence}")),
                amount_minutes: entry.amount_minutes,
                day_equivalent_minutes: entry.day_equivalent_minutes,
                granted_at: entry.granted_at,
                expires_at: entry.expires_at,
                grant_base_date: entry.grant_base_date,
                leave_request_id: entry.leave_request_id,
                reason: entry.reason,
                created_by: entry.created_by,
                effective_at: entry.effective_at,
                created_at: entry.effective_at,
            };
            stored_entries.push(stored.clone());
            appended.push(stored);
        }
        Ok(appended)
    }
}

struct FakeRules {
    grant_rules: Vec<LeaveGrantRule>,
    obligation: Option<LeaveObligationRule>,
}

impl FakeRules {
    fn standard() -> Self {
        Self {
            grant_rules: vec![
                LeaveGrantRule {
                    tenure_months: 6,
                    granted_days: 10,
                    expiry_months: 24,
                    day_equivalent_minutes: 480,
                },
                LeaveGrantRule {
                    tenure_months: 18,
                    granted_days: 11,
                    expiry_months: 24,
                    day_equivalent_minutes: 480,
                },
            ],
            obligation: Some(LeaveObligationRule {
                minimum_granted_days: 10,
                required_days: 5,
                window_months: 12,
                warning_lead_days: 90,
            }),
        }
    }
}

#[async_trait::async_trait]
impl LeaveRuleRepository for &FakeRules {
    async fn grant_rules(
        &self,
        _leave_type: &str,
        _on: NaiveDate,
    ) -> Result<Vec<LeaveGrantRule>, LeaveLedgerError> {
        Ok(self.grant_rules.clone())
    }

    async fn obligation_rule(
        &self,
        _leave_type: &str,
        _on: NaiveDate,
    ) -> Result<Option<LeaveObligationRule>, LeaveLedgerError> {
        Ok(self.obligation.clone())
    }
}

struct FakeUsers {
    candidates: Vec<GrantCandidate>,
    hire_date_updates: Mutex<Vec<(String, NaiveDate)>>,
}

impl FakeUsers {
    fn with(candidates: Vec<GrantCandidate>) -> Self {
        Self {
            candidates,
            hire_date_updates: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl LeaveGrantUserRepository for &FakeUsers {
    async fn list_candidates(
        &self,
        user_ids: Option<&[String]>,
    ) -> Result<Vec<GrantCandidate>, LeaveLedgerError> {
        Ok(match user_ids {
            Some(ids) => self
                .candidates
                .iter()
                .filter(|candidate| ids.contains(&candidate.user_id))
                .cloned()
                .collect(),
            None => self.candidates.clone(),
        })
    }

    async fn set_hire_date(
        &self,
        user_id: &str,
        hire_date: NaiveDate,
    ) -> Result<(), LeaveLedgerError> {
        if !self.candidates.iter().any(|c| c.user_id == user_id) {
            return Err(LeaveLedgerError::UserNotFound);
        }
        self.hire_date_updates
            .lock()
            .expect("updates lock")
            .push((user_id.to_string(), hire_date));
        Ok(())
    }
}

fn candidate(user_id: &str, hire_date: Option<NaiveDate>) -> GrantCandidate {
    GrantCandidate {
        user_id: user_id.to_string(),
        hire_date,
    }
}

fn run_command(base_date: NaiveDate, dry_run: bool) -> RunLeaveGrantsCommand {
    RunLeaveGrantsCommand {
        base_date,
        dry_run,
        user_ids: None,
        exclude_user_ids: Vec::new(),
        created_by: Some("admin-1".to_string()),
        now: ts(2026, 7, 5),
    }
}

#[tokio::test]
async fn run_grants_writes_grant_lots_for_due_users() {
    let ledger = FakeLedger::default();
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![
        candidate("due-6m", Some(date(2026, 1, 1))),
        candidate("due-18m", Some(date(2025, 1, 1))),
        candidate("not-due", Some(date(2026, 2, 15))),
        candidate("no-hire-date", None),
    ]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let report = use_case
        .execute(run_command(date(2026, 7, 1), false))
        .await
        .expect("run grants");

    assert_eq!(report.granted.len(), 2);
    let six_months = report
        .granted
        .iter()
        .find(|outcome| outcome.user_id == "due-6m")
        .expect("6 month grant");
    assert_eq!(six_months.tenure_months, 6);
    assert_eq!(six_months.granted_minutes, 4800);
    assert_eq!(six_months.expires_at, date(2028, 7, 1));
    assert!(six_months.lot_id.is_some());

    let eighteen_months = report
        .granted
        .iter()
        .find(|outcome| outcome.user_id == "due-18m")
        .expect("18 month grant");
    assert_eq!(eighteen_months.granted_minutes, 480 * 11);

    assert!(report
        .skipped
        .contains(&("not-due".to_string(), GrantSkipReason::NotDue)));
    assert!(report
        .skipped
        .contains(&("no-hire-date".to_string(), GrantSkipReason::HireDateNotSet)));
    assert_eq!(ledger.stored().len(), 2);
}

#[tokio::test]
async fn run_grants_dry_run_writes_nothing() {
    let ledger = FakeLedger::default();
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![candidate("due-6m", Some(date(2026, 1, 1)))]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let report = use_case
        .execute(run_command(date(2026, 7, 1), true))
        .await
        .expect("dry run");

    assert_eq!(report.granted.len(), 1);
    assert!(report.granted[0].lot_id.is_none());
    assert!(ledger.stored().is_empty());
}

#[tokio::test]
async fn run_grants_is_idempotent_per_base_date() {
    let ledger = FakeLedger::default();
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![candidate("due-6m", Some(date(2026, 1, 1)))]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let first = use_case
        .execute(run_command(date(2026, 7, 1), false))
        .await
        .expect("first run");
    assert_eq!(first.granted.len(), 1);

    let second = use_case
        .execute(run_command(date(2026, 7, 1), false))
        .await
        .expect("second run");
    assert!(second.granted.is_empty());
    assert_eq!(
        second.skipped,
        vec![("due-6m".to_string(), GrantSkipReason::AlreadyGranted)]
    );
    assert_eq!(ledger.stored().len(), 1);
}

#[tokio::test]
async fn run_grants_backfills_expirations_idempotently() {
    let expired_grant = StoredLeaveLedgerEntry {
        id: "entry-1".to_string(),
        user_id: "user-1".to_string(),
        leave_type: ANNUAL_LEAVE_TYPE.to_string(),
        kind: LeaveLedgerKind::Grant,
        lot_id: "lot-old".to_string(),
        amount_minutes: 4800,
        day_equivalent_minutes: 480,
        granted_at: Some(date(2024, 1, 1)),
        expires_at: Some(date(2026, 1, 1)),
        grant_base_date: Some(date(2024, 1, 1)),
        leave_request_id: None,
        reason: None,
        created_by: None,
        effective_at: ts(2024, 1, 1),
        created_at: ts(2024, 1, 1),
    };
    let ledger = FakeLedger::seeded(vec![expired_grant]);
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![candidate("user-1", Some(date(2026, 1, 1)))]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let report = use_case
        .execute(run_command(date(2026, 7, 1), false))
        .await
        .expect("run");
    assert_eq!(report.expired.len(), 1);
    assert_eq!(report.expired[0].lot_id, "lot-old");
    assert_eq!(report.expired[0].amount_minutes, -4800);

    // 2 回目は expire が記帳済みなので補記されない。
    let second = use_case
        .execute(run_command(date(2026, 7, 1), false))
        .await
        .expect("second run");
    assert!(second.expired.is_empty());
}

#[tokio::test]
async fn run_grants_rejects_unknown_explicit_users_and_empty_rules() {
    let ledger = FakeLedger::default();
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![candidate("known", Some(date(2026, 1, 1)))]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let mut command = run_command(date(2026, 7, 1), false);
    command.user_ids = Some(vec!["known".to_string(), "missing".to_string()]);
    assert_eq!(
        use_case.execute(command).await,
        Err(LeaveLedgerError::UserNotFound)
    );

    let empty_rules = FakeRules {
        grant_rules: Vec::new(),
        obligation: None,
    };
    let use_case = RunLeaveGrants::new(&ledger, &empty_rules, &users);
    assert_eq!(
        use_case.execute(run_command(date(2026, 7, 1), false)).await,
        Err(LeaveLedgerError::RulesNotConfigured)
    );
}

#[tokio::test]
async fn run_grants_honors_exclude_list() {
    let ledger = FakeLedger::default();
    let rules = FakeRules::standard();
    let users = FakeUsers::with(vec![candidate("due-6m", Some(date(2026, 1, 1)))]);
    let use_case = RunLeaveGrants::new(&ledger, &rules, &users);

    let mut command = run_command(date(2026, 7, 1), false);
    command.exclude_user_ids = vec!["due-6m".to_string()];
    let report = use_case.execute(command).await.expect("run");
    assert!(report.granted.is_empty());
    assert_eq!(
        report.skipped,
        vec![("due-6m".to_string(), GrantSkipReason::Excluded)]
    );
}

#[tokio::test]
async fn get_balance_derives_lots_and_obligations() {
    let ledger = FakeLedger::seeded(vec![StoredLeaveLedgerEntry {
        id: "entry-1".to_string(),
        user_id: "user-1".to_string(),
        leave_type: ANNUAL_LEAVE_TYPE.to_string(),
        kind: LeaveLedgerKind::Grant,
        lot_id: "lot-1".to_string(),
        amount_minutes: 4800,
        day_equivalent_minutes: 480,
        granted_at: Some(date(2026, 7, 1)),
        expires_at: Some(date(2028, 7, 1)),
        grant_base_date: Some(date(2026, 7, 1)),
        leave_request_id: None,
        reason: None,
        created_by: None,
        effective_at: ts(2026, 7, 1),
        created_at: ts(2026, 7, 1),
    }]);
    let rules = FakeRules::standard();
    let use_case = GetLeaveBalance::new(&ledger, &rules);

    let view = use_case
        .execute(GetLeaveBalanceCommand {
            user_id: "user-1".to_string(),
            as_of: date(2026, 7, 5),
        })
        .await
        .expect("balance");

    assert_eq!(view.balance.available_minutes, 4800);
    assert_eq!(view.balance.lots.len(), 1);
    assert_eq!(view.obligations.len(), 1);
    assert_eq!(view.obligations[0].status, ObligationStatus::Ok);
}

#[tokio::test]
async fn get_balance_without_obligation_rule_returns_no_windows() {
    let ledger = FakeLedger::default();
    let rules = FakeRules {
        grant_rules: Vec::new(),
        obligation: None,
    };
    let use_case = GetLeaveBalance::new(&ledger, &rules);
    let view = use_case
        .execute(GetLeaveBalanceCommand {
            user_id: "user-1".to_string(),
            as_of: date(2026, 7, 5),
        })
        .await
        .expect("balance");
    assert_eq!(view.balance.available_minutes, 0);
    assert!(view.obligations.is_empty());
}

fn adjust_command(amount: i64) -> AdjustLeaveLedgerCommand {
    AdjustLeaveLedgerCommand {
        user_id: "user-1".to_string(),
        amount_minutes: amount,
        lot_id: None,
        day_equivalent_minutes: Some(480),
        granted_at: Some(date(2025, 10, 1)),
        expires_at: Some(date(2027, 10, 1)),
        grant_base_date: Some(date(2025, 10, 1)),
        reason: "initial migration".to_string(),
        created_by: Some("admin-1".to_string()),
        dry_run: false,
        now: ts(2026, 7, 5),
    }
}

#[tokio::test]
async fn adjust_new_lot_appends_and_reports_balance() {
    let ledger = FakeLedger::default();
    let users = FakeUsers::with(vec![candidate("user-1", None)]);
    let use_case = AdjustLeaveLedger::new(&ledger, &users);

    let report = use_case
        .execute(adjust_command(2400))
        .await
        .expect("adjust");
    assert!(!report.dry_run);
    let entry = report.entry.expect("stored entry");
    assert_eq!(entry.kind, LeaveLedgerKind::Adjust);
    assert_eq!(report.balance_after.available_minutes, 2400);
    assert_eq!(ledger.stored().len(), 1);
}

#[tokio::test]
async fn adjust_dry_run_reports_projected_balance_without_writing() {
    let ledger = FakeLedger::default();
    let users = FakeUsers::with(vec![candidate("user-1", None)]);
    let use_case = AdjustLeaveLedger::new(&ledger, &users);

    let mut command = adjust_command(2400);
    command.dry_run = true;
    let report = use_case.execute(command).await.expect("dry run");
    assert!(report.dry_run);
    assert!(report.entry.is_none());
    assert_eq!(report.balance_after.available_minutes, 2400);
    assert!(ledger.stored().is_empty());
}

#[tokio::test]
async fn adjust_validates_input() {
    let ledger = FakeLedger::default();
    let users = FakeUsers::with(vec![candidate("user-1", None)]);
    let use_case = AdjustLeaveLedger::new(&ledger, &users);

    let mut zero = adjust_command(0);
    zero.dry_run = true;
    assert!(matches!(
        use_case.execute(zero).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));

    let mut no_reason = adjust_command(2400);
    no_reason.reason = "  ".to_string();
    assert!(matches!(
        use_case.execute(no_reason).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));

    let mut negative_new_lot = adjust_command(-480);
    negative_new_lot.lot_id = None;
    assert!(matches!(
        use_case.execute(negative_new_lot).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));

    let mut missing_dates = adjust_command(2400);
    missing_dates.granted_at = None;
    assert!(matches!(
        use_case.execute(missing_dates).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));

    let mut unknown_user = adjust_command(2400);
    unknown_user.user_id = "missing".to_string();
    assert_eq!(
        use_case.execute(unknown_user).await,
        Err(LeaveLedgerError::UserNotFound)
    );
}

#[tokio::test]
async fn adjust_on_existing_lot_rejects_overdraw() {
    let ledger = FakeLedger::default();
    let users = FakeUsers::with(vec![candidate("user-1", None)]);
    let use_case = AdjustLeaveLedger::new(&ledger, &users);

    let created = use_case
        .execute(adjust_command(2400))
        .await
        .expect("seed lot");
    let lot_id = created.entry.expect("entry").lot_id;

    let mut deduction = adjust_command(-480);
    deduction.lot_id = Some(lot_id.clone());
    let report = use_case.execute(deduction).await.expect("deduct");
    assert_eq!(report.balance_after.available_minutes, 1920);

    let mut overdraw = adjust_command(-4800);
    overdraw.lot_id = Some(lot_id.clone());
    assert!(matches!(
        use_case.execute(overdraw).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));

    let mut unknown_lot = adjust_command(-480);
    unknown_lot.lot_id = Some("nope".to_string());
    assert!(matches!(
        use_case.execute(unknown_lot).await,
        Err(LeaveLedgerError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn set_hire_date_updates_users() {
    let users = FakeUsers::with(vec![candidate("user-1", None)]);
    let use_case = SetHireDate::new(&users);
    use_case
        .execute("user-1", date(2026, 4, 1))
        .await
        .expect("set hire date");
    assert_eq!(
        users.hire_date_updates.lock().expect("updates").as_slice(),
        &[("user-1".to_string(), date(2026, 4, 1))]
    );

    assert_eq!(
        use_case.execute("missing", date(2026, 4, 1)).await,
        Err(LeaveLedgerError::UserNotFound)
    );
}
