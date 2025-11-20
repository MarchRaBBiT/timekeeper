use crate::{
    api::{ApiClient, AttendanceSummary},
    pages::{
        dashboard::utils::current_year_month,
        requests::{
            repository::RequestsRepository,
            types::{flatten_requests, RequestKind, RequestSummary},
        },
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_work_hours: Option<f64>,
    pub total_work_days: Option<i32>,
    pub average_daily_hours: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardAlertLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub level: DashboardAlertLevel,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DashboardActivity {
    pub title: String,
    pub detail: Option<String>,
}

pub async fn fetch_summary() -> Result<DashboardSummary, String> {
    let api = ApiClient::new();
    let (y, m) = current_year_month();
    let summary: AttendanceSummary = api.get_my_summary(Some(y), Some(m)).await?;
    Ok(DashboardSummary {
        total_work_hours: Some(summary.total_work_hours),
        total_work_days: Some(summary.total_work_days),
        average_daily_hours: Some(summary.average_daily_hours),
    })
}

pub fn build_alerts(summary: &DashboardSummary) -> Vec<DashboardAlert> {
    let mut alerts = Vec::new();

    if summary.total_work_days.unwrap_or_default() == 0 {
        alerts.push(DashboardAlert {
            level: DashboardAlertLevel::Warning,
            message: "今月の勤怠が未登録です。出勤打刻を確認してください。".into(),
        });
    }

    if alerts.is_empty() {
        alerts.push(DashboardAlert {
            level: DashboardAlertLevel::Info,
            message: "新しいアラートはありません。".into(),
        });
    }

    alerts
}

pub async fn fetch_alerts() -> Result<Vec<DashboardAlert>, String> {
    let summary = fetch_summary().await?;
    Ok(build_alerts(&summary))
}

pub async fn fetch_recent_activities() -> Result<Vec<DashboardActivity>, String> {
    let repo = RequestsRepository::new();
    let response = repo.list_my_requests().await?;
    let summaries = flatten_requests(&response);

    let leave_pending = count_by(&summaries, RequestKind::Leave, "pending");
    let overtime_pending = count_by(&summaries, RequestKind::Overtime, "pending");
    let leave_approved = count_by(&summaries, RequestKind::Leave, "approved");
    let overtime_approved = count_by(&summaries, RequestKind::Overtime, "approved");

    Ok(vec![
        DashboardActivity {
            title: "休暇申請（承認待ち）".into(),
            detail: Some(format!("{leave_pending} 件")),
        },
        DashboardActivity {
            title: "残業申請（承認待ち）".into(),
            detail: Some(format!("{overtime_pending} 件")),
        },
        DashboardActivity {
            title: "休暇申請（承認済み）".into(),
            detail: Some(format!("{leave_approved} 件")),
        },
        DashboardActivity {
            title: "残業申請（承認済み）".into(),
            detail: Some(format!("{overtime_approved} 件")),
        },
    ])
}

pub async fn reload_announcements() -> Result<(), String> {
    // 追加のアナウンス API が用意されたらここで呼び出す。
    Ok(())
}

fn count_by(summaries: &[RequestSummary], kind: RequestKind, status: &str) -> i32 {
    summaries
        .iter()
        .filter(|s| s.kind == kind && s.status == status)
        .count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn alerts_warn_when_no_workdays() {
        let summary = DashboardSummary {
            total_work_hours: Some(0.0),
            total_work_days: Some(0),
            average_daily_hours: None,
        };
        let alerts = build_alerts(&summary);
        assert!(alerts
            .iter()
            .any(|a| matches!(a.level, DashboardAlertLevel::Warning)));
    }

    #[test]
    fn count_handles_missing_kind() {
        let value = json!({});
        assert_eq!(count(&value, "leave_requests", "pending"), 0);
    }

    #[test]
    fn count_filters_by_status() {
        let value = json!({
            "leave_requests": [
                { "status": "pending" },
                { "status": "approved" },
                { "status": "pending" }
            ]
        });
        assert_eq!(count(&value, "leave_requests", "pending"), 2);
        assert_eq!(count(&value, "leave_requests", "approved"), 1);
    }
}
