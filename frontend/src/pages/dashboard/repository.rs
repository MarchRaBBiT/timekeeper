use crate::{
    api::{ApiClient, AttendanceSummary},
    pages::dashboard::utils::current_year_month,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

pub async fn fetch_recent_activities() -> Result<Vec<DashboardActivity>, String> {
    let api = ApiClient::new();
    let value: Value = api.get_my_requests().await?;

    let mut activities = Vec::new();
    let leave_pending = count(&value, "leave_requests", "pending");
    let overtime_pending = count(&value, "overtime_requests", "pending");
    let leave_approved = count(&value, "leave_requests", "approved");
    let overtime_approved = count(&value, "overtime_requests", "approved");

    activities.push(DashboardActivity {
        title: "休暇申請（承認待ち）".into(),
        detail: Some(format!("{leave_pending} 件")),
    });
    activities.push(DashboardActivity {
        title: "残業申請（承認待ち）".into(),
        detail: Some(format!("{overtime_pending} 件")),
    });
    activities.push(DashboardActivity {
        title: "休暇申請（承認済み）".into(),
        detail: Some(format!("{leave_approved} 件")),
    });
    activities.push(DashboardActivity {
        title: "残業申請（承認済み）".into(),
        detail: Some(format!("{overtime_approved} 件")),
    });

    Ok(activities)
}

pub async fn reload_announcements() -> Result<(), String> {
    // 追加のアナウンス API が用意されたらここで呼び出す。
    Ok(())
}

fn count(value: &Value, kind: &str, status: &str) -> i32 {
    value
        .get(kind)
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|item| item.get("status").and_then(|s| s.as_str()) == Some(status))
                .count() as i32
        })
        .unwrap_or(0)
}
