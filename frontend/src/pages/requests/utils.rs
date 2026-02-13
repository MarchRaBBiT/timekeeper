use crate::api::{
    ApiError, CorrectionBreakItem, CreateAttendanceCorrectionRequest, CreateLeaveRequest,
    CreateOvertimeRequest, UpdateAttendanceCorrectionRequest,
};
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
pub struct AttendanceCorrectionFormState {
    date: RwSignal<String>,
    clock_in_time: RwSignal<String>,
    clock_out_time: RwSignal<String>,
    break_rows: RwSignal<String>,
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

impl Default for AttendanceCorrectionFormState {
    fn default() -> Self {
        Self {
            date: create_rw_signal(String::new()),
            clock_in_time: create_rw_signal(String::new()),
            clock_out_time: create_rw_signal(String::new()),
            break_rows: create_rw_signal(String::new()),
            reason: create_rw_signal(String::new()),
        }
    }
}

impl AttendanceCorrectionFormState {
    pub fn date_signal(&self) -> RwSignal<String> {
        self.date
    }

    pub fn clock_in_signal(&self) -> RwSignal<String> {
        self.clock_in_time
    }

    pub fn clock_out_signal(&self) -> RwSignal<String> {
        self.clock_out_time
    }

    pub fn break_rows_signal(&self) -> RwSignal<String> {
        self.break_rows
    }

    pub fn reason_signal(&self) -> RwSignal<String> {
        self.reason
    }

    pub fn reset(&self) {
        self.date.set(String::new());
        self.clock_in_time.set(String::new());
        self.clock_out_time.set(String::new());
        self.break_rows.set(String::new());
        self.reason.set(String::new());
    }

    pub fn load_from_value(&self, value: &Value) {
        if let Some(date) = value.get("date").and_then(|v| v.as_str()) {
            self.date.set(date.to_string());
        }
        if let Some(proposed) = value.get("proposed_values") {
            if let Some(clock_in) = proposed.get("clock_in_time").and_then(|v| v.as_str()) {
                self.clock_in_time.set(clock_in.to_string());
            }
            if let Some(clock_out) = proposed.get("clock_out_time").and_then(|v| v.as_str()) {
                self.clock_out_time.set(clock_out.to_string());
            }
            if let Some(breaks) = proposed.get("breaks").and_then(|v| v.as_array()) {
                let rows = breaks
                    .iter()
                    .map(|item| {
                        let start = item
                            .get("break_start_time")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let end = item
                            .get("break_end_time")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        format!("{start},{end}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                self.break_rows.set(rows);
            }
        }
        if let Some(reason) = value.get("reason").and_then(|v| v.as_str()) {
            self.reason.set(reason.to_string());
        }
    }

    pub fn to_create_payload(self) -> Result<CreateAttendanceCorrectionRequest, ApiError> {
        let date = parse_date(
            &self.date.get(),
            "対象日を YYYY-MM-DD 形式で入力してください。",
        )?;
        let clock_in = parse_datetime_optional(
            &self.clock_in_time.get(),
            "出勤時刻は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
        )?;
        let clock_out = parse_datetime_optional(
            &self.clock_out_time.get(),
            "退勤時刻は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
        )?;
        let breaks = parse_break_rows(&self.break_rows.get())?;
        let reason = self.reason.get().trim().to_string();
        if reason.is_empty() {
            return Err(ApiError::validation("修正理由を入力してください。"));
        }
        Ok(CreateAttendanceCorrectionRequest {
            date,
            clock_in_time: clock_in,
            clock_out_time: clock_out,
            breaks: Some(breaks),
            reason,
        })
    }

    pub fn to_update_payload(self) -> Result<UpdateAttendanceCorrectionRequest, ApiError> {
        let clock_in = parse_datetime_optional(
            &self.clock_in_time.get(),
            "出勤時刻は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
        )?;
        let clock_out = parse_datetime_optional(
            &self.clock_out_time.get(),
            "退勤時刻は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
        )?;
        let breaks = parse_break_rows(&self.break_rows.get())?;
        let reason = self.reason.get().trim().to_string();
        if reason.is_empty() {
            return Err(ApiError::validation("修正理由を入力してください。"));
        }
        Ok(UpdateAttendanceCorrectionRequest {
            clock_in_time: clock_in,
            clock_out_time: clock_out,
            breaks: Some(breaks),
            reason,
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

fn parse_datetime_optional(
    input: &str,
    err: &str,
) -> Result<Option<chrono::NaiveDateTime>, ApiError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_datetime(trimmed, err).map(Some)
}

fn parse_datetime(input: &str, err: &str) -> Result<chrono::NaiveDateTime, ApiError> {
    chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M"))
        .map_err(|_| ApiError::validation(err.to_string()))
}

fn parse_break_rows(input: &str) -> Result<Vec<CorrectionBreakItem>, ApiError> {
    let mut items = Vec::new();
    for line in input.lines() {
        let raw = line.trim();
        if raw.is_empty() {
            continue;
        }
        let mut parts = raw.splitn(2, ',');
        let start_raw = parts.next().unwrap_or_default().trim();
        if start_raw.is_empty() {
            return Err(ApiError::validation(
                "休憩行は `開始時刻,終了時刻(任意)` 形式で入力してください。",
            ));
        }
        let end_raw = parts.next().map(|s| s.trim()).unwrap_or_default();
        let start = parse_datetime(
            start_raw,
            "休憩開始は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
        )?;
        let end = if end_raw.is_empty() {
            None
        } else {
            Some(parse_datetime(
                end_raw,
                "休憩終了は YYYY-MM-DDTHH:MM[:SS] 形式で入力してください。",
            )?)
        };
        items.push(CorrectionBreakItem {
            break_start_time: start,
            break_end_time: end,
        });
    }
    Ok(items)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
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

    #[test]
    fn leave_form_load_reset_and_payload_trim_reason() {
        with_runtime(|| {
            let state = LeaveFormState::default();
            state.load_from_value(&serde_json::json!({
                "leave_type": "sick",
                "start_date": "2025-02-01",
                "end_date": "2025-02-02",
                "reason": "  family matters  "
            }));
            let payload = state.to_payload().expect("leave payload");
            assert_eq!(payload.leave_type, "sick");
            assert_eq!(payload.start_date.to_string(), "2025-02-01");
            assert_eq!(payload.end_date.to_string(), "2025-02-02");
            assert_eq!(payload.reason.as_deref(), Some("family matters"));

            state.reset();
            assert_eq!(state.leave_type_signal().get(), "annual");
            assert!(state.start_signal().get().is_empty());
            assert!(state.end_signal().get().is_empty());
            assert!(state.reason_signal().get().is_empty());
        });
    }

    #[test]
    fn overtime_form_load_reset_and_payload_trim_reason() {
        with_runtime(|| {
            let state = OvertimeFormState::default();
            state.load_from_value(&serde_json::json!({
                "date": "2025-03-10",
                "planned_hours": 1.5,
                "reason": "  release support  "
            }));
            assert_eq!(state.hours_signal().get(), "1.50");

            let payload = state.to_payload().expect("overtime payload");
            assert_eq!(payload.date.to_string(), "2025-03-10");
            assert!((payload.planned_hours - 1.5).abs() < f64::EPSILON);
            assert_eq!(payload.reason.as_deref(), Some("release support"));

            state.reset();
            assert!(state.date_signal().get().is_empty());
            assert!(state.hours_signal().get().is_empty());
            assert!(state.reason_signal().get().is_empty());
        });
    }

    #[test]
    fn overtime_form_rejects_non_numeric_and_upper_bound() {
        with_runtime(|| {
            let state = OvertimeFormState::default();
            state.date_signal().set("2025-04-01".into());

            state.hours_signal().set("abc".into());
            assert_eq!(
                state.to_payload().expect_err("invalid number").code,
                "VALIDATION_ERROR"
            );

            state.hours_signal().set("24.5".into());
            assert_eq!(
                state.to_payload().expect_err("out of range").code,
                "VALIDATION_ERROR"
            );
        });
    }

    #[test]
    fn message_state_transitions_and_clear() {
        let mut state = MessageState::default();
        state.set_success("done");
        assert_eq!(state.success.as_deref(), Some("done"));
        assert!(state.error.is_none());

        state.set_error(ApiError::validation("bad request"));
        assert!(state.success.is_none());
        assert_eq!(
            state.error.as_ref().expect("error exists").code,
            "VALIDATION_ERROR"
        );

        state.clear();
        assert!(state.success.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn request_filter_state_exposes_status_signal() {
        with_runtime(|| {
            let filter = RequestFilterState::default();
            assert!(filter.status_filter().is_empty());
            filter.status_signal().set("pending".into());
            assert_eq!(filter.status_filter(), "pending");
        });
    }
}
