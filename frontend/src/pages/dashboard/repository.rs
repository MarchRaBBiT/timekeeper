use crate::api::{ApiClient, ApiError, AttendanceSummary};
use crate::pages::dashboard::utils::{current_year_month, ActivityStatusFilter};
use crate::pages::requests::repository::RequestsRepository;
use crate::pages::requests::types::{flatten_requests, RequestKind, RequestSummary};
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

pub async fn fetch_summary(api: &ApiClient) -> Result<DashboardSummary, ApiError> {
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

pub async fn fetch_recent_activities(
    api: &ApiClient,
    filter: ActivityStatusFilter,
) -> Result<Vec<DashboardActivity>, ApiError> {
    let repo = RequestsRepository::new(api.clone());
    let response = repo.list_my_requests().await?;
    let summaries = flatten_requests(&response);

    let leave_pending = count_by(&summaries, RequestKind::Leave, "pending");
    let overtime_pending = count_by(&summaries, RequestKind::Overtime, "pending");
    let leave_approved = count_by(&summaries, RequestKind::Leave, "approved");
    let overtime_approved = count_by(&summaries, RequestKind::Overtime, "approved");

    let activities = vec![
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
    ];

    let filtered = match filter {
        ActivityStatusFilter::All => activities,
        ActivityStatusFilter::PendingOnly => activities.into_iter().take(2).collect(),
        ActivityStatusFilter::ApprovedOnly => activities.into_iter().skip(2).collect(),
    };

    Ok(filtered)
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
    use httpmock::prelude::*;

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
        let empty: crate::pages::requests::types::MyRequestsResponse = Default::default();
        let summaries = flatten_requests(&empty);
        assert_eq!(count_by(&summaries, RequestKind::Leave, "pending"), 0);
    }

    #[tokio::test]
    async fn fetch_summary_and_activities_from_api() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/me/summary");
            then.status(200).json_body(serde_json::json!({
                "month": 1,
                "year": 2025,
                "total_work_hours": 160.0,
                "total_work_days": 20,
                "average_daily_hours": 8.0
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/requests/me");
            then.status(200).json_body(serde_json::json!({
                "leave_requests": [],
                "overtime_requests": []
            }));
        });

        let api = ApiClient::new_with_base_url(&server.url("/api"));
        let summary = fetch_summary(&api).await.unwrap();
        assert_eq!(summary.total_work_days, Some(20));

        let activities = fetch_recent_activities(&api, ActivityStatusFilter::All)
            .await
            .unwrap();
        assert_eq!(activities.len(), 4);
    }
}
