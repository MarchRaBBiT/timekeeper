use crate::api::{ApiError, CreateLeaveRequest, CreateOvertimeRequest};
use chrono::NaiveDate;
use leptos::*;
use serde_json::Value;

use super::types::RequestKind;

#[derive(Clone, PartialEq, Eq)]
pub struct EditTarget {
    pub id: String,
    pub kind: RequestKind,
}

#[derive(Clone, Copy)]
pub struct LeaveFormState {
    leave_type: RwSignal<String>,
    start_date: RwSignal<String>,
    end_date: RwSignal<String>,
    reason: RwSignal<String>,
}

impl Default for LeaveFormState {
    fn default() -> Self {
        Self {
            leave_type: create_rw_signal("annual".to_string()),
            start_date: create_rw_signal(String::new()),
            end_date: create_rw_signal(String::new()),
            reason: create_rw_signal(String::new()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct OvertimeFormState {
    date: RwSignal<String>,
    hours: RwSignal<String>,
    reason: RwSignal<String>,
}

#[derive(Clone, Copy)]
pub struct RequestFilterState {
    status: RwSignal<String>,
}

impl LeaveFormState {
    pub fn leave_type_signal(&self) -> RwSignal<String> {
        self.leave_type
    }

    pub fn start_signal(&self) -> RwSignal<String> {
        self.start_date
    }

    pub fn end_signal(&self) -> RwSignal<String> {
        self.end_date
    }

    pub fn reason_signal(&self) -> RwSignal<String> {
        self.reason
    }

    pub fn reset(&self) {
        self.leave_type.set("annual".into());
        self.start_date.set(String::new());
        self.end_date.set(String::new());
        self.reason.set(String::new());
    }

    pub fn load_from_value(&self, value: &Value) {
        if let Some(leave_type) = value.get("leave_type").and_then(|v| v.as_str()) {
            self.leave_type.set(leave_type.to_string());
        }
        if let Some(start) = value.get("start_date").and_then(|v| v.as_str()) {
            self.start_date.set(start.to_string());
        }
        if let Some(end) = value.get("end_date").and_then(|v| v.as_str()) {
            self.end_date.set(end.to_string());
        }
        if let Some(reason) = value.get("reason").and_then(|v| v.as_str()) {
            self.reason.set(reason.to_string());
        }
    }

    pub fn to_payload(self) -> Result<CreateLeaveRequest, ApiError> {
        let start = parse_date(
            &self.start_date.get(),
            "開始日を YYYY-MM-DD 形式で入力してください。",
        )?;
        let end = parse_date(
            &self.end_date.get(),
            "終了日を YYYY-MM-DD 形式で入力してください。",
        )?;
        if end < start {
            return Err(ApiError::validation(
                "終了日は開始日以降の日付を指定してください。",
            ));
        }
        Ok(CreateLeaveRequest {
            leave_type: self.leave_type.get(),
            start_date: start,
            end_date: end,
            reason: optional_string(self.reason.get()),
        })
    }
}

impl Default for OvertimeFormState {
    fn default() -> Self {
        Self {
            date: create_rw_signal(String::new()),
            hours: create_rw_signal(String::new()),
            reason: create_rw_signal(String::new()),
        }
    }
}

impl OvertimeFormState {
    pub fn date_signal(&self) -> RwSignal<String> {
        self.date
    }

    pub fn hours_signal(&self) -> RwSignal<String> {
        self.hours
    }

    pub fn reason_signal(&self) -> RwSignal<String> {
        self.reason
    }

    pub fn reset(&self) {
        self.date.set(String::new());
        self.hours.set(String::new());
        self.reason.set(String::new());
    }

    pub fn load_from_value(&self, value: &Value) {
        if let Some(date) = value.get("date").and_then(|v| v.as_str()) {
            self.date.set(date.to_string());
        }
        if let Some(hours) = value.get("planned_hours").and_then(|v| v.as_f64()) {
            self.hours.set(format!("{:.2}", hours));
        }
        if let Some(reason) = value.get("reason").and_then(|v| v.as_str()) {
            self.reason.set(reason.to_string());
        }
    }

    pub fn to_payload(self) -> Result<CreateOvertimeRequest, ApiError> {
        let date = parse_date(
            &self.date.get(),
            "残業日を YYYY-MM-DD 形式で入力してください。",
        )?;
        let hours_raw = self.hours.get();
        let hours = hours_raw
            .trim()
            .parse::<f64>()
            .map_err(|_| ApiError::validation("残業時間は数値で入力してください。"))?;
        if !(0.25..=24.0).contains(&hours) {
            return Err(ApiError::validation(
                "残業時間は0.25〜24.0の範囲で指定してください。",
            ));
        }
        Ok(CreateOvertimeRequest {
            date,
            planned_hours: hours,
            reason: optional_string(self.reason.get()),
        })
    }
}

#[derive(Clone, Default)]
pub struct MessageState {
    pub success: Option<String>,
    pub error: Option<ApiError>,
}

impl MessageState {
    pub fn set_success(&mut self, msg: impl Into<String>) {
        self.success = Some(msg.into());
        self.error = None;
    }

    pub fn set_error(&mut self, msg: ApiError) {
        self.error = Some(msg);
        self.success = None;
    }

    pub fn clear(&mut self) {
        self.success = None;
        self.error = None;
    }
}

impl Default for RequestFilterState {
    fn default() -> Self {
        Self {
            status: create_rw_signal(String::new()),
        }
    }
}

impl RequestFilterState {
    pub fn status_signal(&self) -> RwSignal<String> {
        self.status
    }

    pub fn status_filter(&self) -> String {
        self.status.get()
    }
}

fn parse_date(input: &str, err: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::validation(err.to_string()))
}

fn optional_string(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ssr::with_runtime;

    #[test]
    fn leave_form_rejects_invalid_dates() {
        with_runtime(|| {
            let state = LeaveFormState::default();
            state.start_signal().set("2025-01-10".into());
            state.end_signal().set("2025-01-05".into());
            assert!(state.to_payload().is_err());
        });
    }

    #[test]
    fn overtime_form_validates_hours() {
        with_runtime(|| {
            let state = OvertimeFormState::default();
            state.date_signal().set("2025-01-15".into());
            state.hours_signal().set("0.1".into());
            assert!(state.to_payload().is_err());
            state.hours_signal().set("2.5".into());
            assert!(state.to_payload().is_ok());
        });
    }
}
