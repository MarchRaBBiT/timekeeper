//! `GetLeaveBalance`: 残高・時効予定・年5日義務の read-model 導出 use case（T-04）。

use chrono::NaiveDate;
use timekeeper_domain::leave_ledger::{
    annual_obligations, derive_balance, LeaveBalance, LeaveLedgerEvent, ObligationWindow,
};

use super::{LeaveLedgerError, LeaveLedgerRepository, LeaveRuleRepository};

// ---------------------------------------------------------------------------
// GetLeaveBalance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLeaveBalanceCommand {
    pub user_id: String,
    pub leave_type_code: String,
    pub as_of: NaiveDate,
}

/// 残高 read-model（保存されない導出値）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveBalanceView {
    pub user_id: String,
    pub leave_type: String,
    pub balance: LeaveBalance,
    pub obligations: Vec<ObligationWindow>,
}

#[derive(Debug, Clone)]
pub struct GetLeaveBalance<L, R> {
    ledger: L,
    rules: R,
}

impl<L, R> GetLeaveBalance<L, R>
where
    L: LeaveLedgerRepository,
    R: LeaveRuleRepository,
{
    pub fn new(ledger: L, rules: R) -> Self {
        Self { ledger, rules }
    }

    pub async fn execute(
        &self,
        command: GetLeaveBalanceCommand,
    ) -> Result<LeaveBalanceView, LeaveLedgerError> {
        let entries = self
            .ledger
            .list_entries(&command.user_id, &command.leave_type_code)
            .await?;
        let events: Vec<LeaveLedgerEvent> = entries
            .iter()
            .map(|entry| entry.to_domain_event())
            .collect();
        let balance = derive_balance(&events, command.as_of);
        let obligations = match self
            .rules
            .obligation_rule(&command.leave_type_code, command.as_of)
            .await?
        {
            Some(rule) => annual_obligations(&events, &rule, command.as_of),
            None => Vec::new(),
        };
        Ok(LeaveBalanceView {
            user_id: command.user_id,
            leave_type: command.leave_type_code,
            balance,
            obligations,
        })
    }
}
