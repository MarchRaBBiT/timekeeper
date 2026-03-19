use crate::api::{
    ApiError, CorrectionBreakItem, CreateAttendanceCorrectionRequest, CreateLeaveRequest,
    CreateOvertimeRequest, UpdateAttendanceCorrectionRequest,
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
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
    break_rows: RwSignal<Vec<(String, String)>>,
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
            &rust_i18n::t!("pages.requests.validation.leave_start_date"),
        )?;
        let end = parse_date(
            &self.end_date.get(),
            &rust_i18n::t!("pages.requests.validation.leave_end_date"),
        )?;
        if end < start {
            return Err(ApiError::validation(rust_i18n::t!(
                "pages.requests.validation.leave_date_order"
            )));
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
            &rust_i18n::t!("pages.requests.validation.overtime_date"),
        )?;
        let hours_raw = self.hours.get();
        let hours = hours_raw.trim().parse::<f64>().map_err(|_| {
            ApiError::validation(rust_i18n::t!(
                "pages.requests.validation.overtime_hours_number"
            ))
        })?;
        if !(0.25..=24.0).contains(&hours) {
            return Err(ApiError::validation(rust_i18n::t!(
                "pages.requests.validation.overtime_hours_range"
            )));
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
            break_rows: create_rw_signal(Vec::new()),
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

    pub fn break_rows_signal(&self) -> RwSignal<Vec<(String, String)>> {
        self.break_rows
    }

    pub fn add_break_row(&self) {
        self.break_rows
            .update(|rows| rows.push((String::new(), String::new())));
    }

    pub fn remove_break_row(&self, idx: usize) {
        self.break_rows.update(|rows| {
            if idx < rows.len() {
                rows.remove(idx);
            }
        });
    }

    pub fn update_break_start(&self, idx: usize, value: String) {
        self.break_rows.update(|rows| {
            if let Some(row) = rows.get_mut(idx) {
                row.0 = value;
            }
        });
    }

    pub fn update_break_end(&self, idx: usize, value: String) {
        self.break_rows.update(|rows| {
            if let Some(row) = rows.get_mut(idx) {
                row.1 = value;
            }
        });
    }

    pub fn reason_signal(&self) -> RwSignal<String> {
        self.reason
    }

    pub fn reset(&self) {
        self.date.set(String::new());
        self.clock_in_time.set(String::new());
        self.clock_out_time.set(String::new());
        self.break_rows.set(Vec::new());
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
                self.break_rows.set(
                    breaks
                        .iter()
                        .map(|item| {
                            let start = item
                                .get("break_start_time")
                                .and_then(|v| v.as_str())
                                .map(format_time_input)
                                .unwrap_or_default();
                            let end = item
                                .get("break_end_time")
                                .and_then(|v| v.as_str())
                                .map(format_time_input)
                                .unwrap_or_default();
                            (start, end)
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
        if let Some(reason) = value.get("reason").and_then(|v| v.as_str()) {
            self.reason.set(reason.to_string());
        }
    }

    pub fn to_create_payload(self) -> Result<CreateAttendanceCorrectionRequest, ApiError> {
        let date = parse_date(
            &self.date.get(),
            &rust_i18n::t!("pages.requests.validation.correction_date"),
        )?;
        let clock_in = parse_datetime_optional(
            &self.clock_in_time.get(),
            &rust_i18n::t!("pages.requests.validation.correction_clock_in"),
        )?;
        let clock_out = parse_datetime_optional(
            &self.clock_out_time.get(),
            &rust_i18n::t!("pages.requests.validation.correction_clock_out"),
        )?;
        let breaks = parse_break_rows(date, &self.break_rows.get())?;
        let reason = self.reason.get().trim().to_string();
        if reason.is_empty() {
            return Err(ApiError::validation(rust_i18n::t!(
                "pages.requests.validation.correction_reason_required"
            )));
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
        let date = parse_date(
            &self.date.get(),
            &rust_i18n::t!("pages.requests.validation.correction_date"),
        )?;
        let clock_in = parse_datetime_optional(
            &self.clock_in_time.get(),
            &rust_i18n::t!("pages.requests.validation.correction_clock_in"),
        )?;
        let clock_out = parse_datetime_optional(
            &self.clock_out_time.get(),
            &rust_i18n::t!("pages.requests.validation.correction_clock_out"),
        )?;
        let breaks = parse_break_rows(date, &self.break_rows.get())?;
        let reason = self.reason.get().trim().to_string();
        if reason.is_empty() {
            return Err(ApiError::validation(rust_i18n::t!(
                "pages.requests.validation.correction_reason_required"
            )));
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

fn parse_time(input: &str, err: &str) -> Result<NaiveTime, ApiError> {
    NaiveTime::parse_from_str(input, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(input, "%H:%M:%S"))
        .map_err(|_| ApiError::validation(err.to_string()))
}

fn format_time_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(value) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M"))
    {
        return value.time().format("%H:%M").to_string();
    }
    if let Ok(value) = NaiveTime::parse_from_str(trimmed, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(trimmed, "%H:%M:%S"))
    {
        return value.format("%H:%M").to_string();
    }
    String::new()
}

fn parse_break_rows(
    date: NaiveDate,
    rows: &[(String, String)],
) -> Result<Vec<CorrectionBreakItem>, ApiError> {
    let mut items = Vec::new();
    for (start_raw, end_raw) in rows {
        let start_trimmed = start_raw.trim();
        if start_trimmed.is_empty() {
            continue;
        }
        let start = parse_time(
            start_trimmed,
            &rust_i18n::t!("pages.requests.validation.break_start"),
        )?;
        let end_trimmed = end_raw.trim();
        let end = if end_trimmed.is_empty() {
            None
        } else {
            Some(parse_time(
                end_trimmed,
                &rust_i18n::t!("pages.requests.validation.break_end"),
            )?)
        };
        items.push(CorrectionBreakItem {
            break_start_time: date.and_time(start),
            break_end_time: end.map(|time| date.and_time(time)),
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
    fn attendance_correction_form_load_reset_and_row_editing() {
        with_runtime(|| {
            let state = AttendanceCorrectionFormState::default();
            state.load_from_value(&serde_json::json!({
                "date": "2025-06-01",
                "proposed_values": {
                    "clock_in_time": "2025-06-01T09:00:00",
                    "clock_out_time": "2025-06-01T18:00:00",
                    "breaks": [{
                        "break_start_time": "2025-06-01T12:00:00",
                        "break_end_time": "2025-06-01T12:30:00"
                    }]
                },
                "reason": "fix"
            }));

            assert_eq!(state.date_signal().get(), "2025-06-01");
            assert_eq!(state.clock_in_signal().get(), "2025-06-01T09:00:00");
            assert_eq!(state.clock_out_signal().get(), "2025-06-01T18:00:00");
            assert_eq!(state.reason_signal().get(), "fix");
            assert_eq!(
                state.break_rows_signal().get(),
                vec![("12:00".to_string(), "12:30".to_string())]
            );

            state.add_break_row();
            assert_eq!(state.break_rows_signal().get().len(), 2);
            state.update_break_start(1, "14:00".to_string());
            state.update_break_end(1, "14:30".to_string());
            assert_eq!(
                state.break_rows_signal().get()[1],
                ("14:00".to_string(), "14:30".to_string())
            );
            state.remove_break_row(0);
            assert_eq!(state.break_rows_signal().get().len(), 1);
            assert_eq!(
                state.break_rows_signal().get()[0],
                ("14:00".to_string(), "14:30".to_string())
            );

            let payload = state.to_update_payload().expect("correction payload");
            assert_eq!(
                payload.clock_in_time.map(|v| v.to_string()),
                Some("2025-06-01 09:00:00".to_string())
            );
            assert_eq!(
                payload.clock_out_time.map(|v| v.to_string()),
                Some("2025-06-01 18:00:00".to_string())
            );
            assert_eq!(payload.breaks.as_ref().map(|v| v.len()), Some(1));
            let break_item = payload
                .breaks
                .as_ref()
                .and_then(|items| items.first())
                .expect("break item");
            assert_eq!(
                break_item.break_start_time.to_string(),
                "2025-06-01 14:00:00"
            );
            assert_eq!(
                break_item.break_end_time.map(|v| v.to_string()),
                Some("2025-06-01 14:30:00".to_string())
            );

            state.reset();
            assert!(state.date_signal().get().is_empty());
            assert!(state.clock_in_signal().get().is_empty());
            assert!(state.clock_out_signal().get().is_empty());
            assert!(state.break_rows_signal().get().is_empty());
            assert!(state.reason_signal().get().is_empty());
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
