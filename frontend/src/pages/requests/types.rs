use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MyRequestsResponse {
    #[serde(default)]
    pub leave_requests: Vec<Value>,
    #[serde(default)]
    pub overtime_requests: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    Leave,
    Overtime,
}

#[derive(Debug, Clone)]
pub struct RequestSummary {
    pub id: String,
    pub kind: RequestKind,
    pub status: String,
    pub submitted_at: Option<String>,
    pub primary_label: Option<String>,
    pub secondary_label: Option<String>,
    pub reason: Option<String>,
    pub details: Value,
}

impl RequestSummary {
    pub fn from_leave(value: &Value) -> Self {
        let start = value
            .get("start_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let end = value
            .get("end_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let primary_label = match (start, end) {
            (Some(s), Some(e)) if s != e => Some(format!("{} 〜 {}", s, e)),
            (Some(s), _) => Some(s),
            _ => None,
        };
        Self {
            id: extract_string(value, "id"),
            kind: RequestKind::Leave,
            status: extract_string(value, "status"),
            submitted_at: value
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            primary_label,
            secondary_label: value
                .get("leave_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            reason: value
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            details: value.clone(),
        }
    }

    pub fn from_overtime(value: &Value) -> Self {
        let hours = value
            .get("planned_hours")
            .and_then(|v| v.as_f64())
            .map(|h| format!("{} 時間", h));
        Self {
            id: extract_string(value, "id"),
            kind: RequestKind::Overtime,
            status: extract_string(value, "status"),
            submitted_at: value
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            primary_label: value
                .get("date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            secondary_label: hours,
            reason: value
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            details: value.clone(),
        }
    }
}

fn extract_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

pub fn flatten_requests(response: &MyRequestsResponse) -> Vec<RequestSummary> {
    let mut list: Vec<RequestSummary> = response
        .leave_requests
        .iter()
        .map(RequestSummary::from_leave)
        .chain(
            response
                .overtime_requests
                .iter()
                .map(RequestSummary::from_overtime),
        )
        .collect();
    list.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn summarizes_leave_request() {
        let value = json!({
            "id": "req-1",
            "status": "pending",
            "start_date": "2025-01-10",
            "end_date": "2025-01-12",
            "leave_type": "annual",
            "reason": "family",
            "created_at": "2025-01-05T10:00:00Z"
        });
        let summary = RequestSummary::from_leave(&value);
        assert_eq!(summary.id, "req-1");
        assert_eq!(summary.status, "pending");
        assert_eq!(
            summary.primary_label.as_deref(),
            Some("2025-01-10 〜 2025-01-12")
        );
        assert_eq!(summary.secondary_label.as_deref(), Some("annual"));
    }

    #[test]
    fn summarizes_overtime_request() {
        let value = json!({
            "id": "ot-1",
            "status": "approved",
            "date": "2025-01-15",
            "planned_hours": 3.5,
            "reason": "deploy",
            "created_at": "2025-01-14T09:00:00Z"
        });
        let summary = RequestSummary::from_overtime(&value);
        assert_eq!(summary.kind, RequestKind::Overtime);
        assert_eq!(summary.primary_label.as_deref(), Some("2025-01-15"));
        assert_eq!(summary.secondary_label.as_deref(), Some("3.5 時間"));
    }
}
