use crate::api::{ApiClient, AttendanceResponse, AttendanceStatusResponse};
use crate::utils::time::today_in_app_tz;
use chrono::NaiveDate;
use leptos::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockEventKind {
    ClockIn,
    BreakStart,
    BreakEnd,
    ClockOut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockEventPayload {
    pub kind: ClockEventKind,
    pub attendance_id: Option<String>,
    pub break_id: Option<String>,
}

impl ClockEventPayload {
    pub fn clock_in() -> Self {
        Self {
            kind: ClockEventKind::ClockIn,
            attendance_id: None,
            break_id: None,
        }
    }

    pub fn clock_out() -> Self {
        Self {
            kind: ClockEventKind::ClockOut,
            attendance_id: None,
            break_id: None,
        }
    }

    pub fn break_start(attendance_id: String) -> Self {
        Self {
            kind: ClockEventKind::BreakStart,
            attendance_id: Some(attendance_id),
            break_id: None,
        }
    }

    pub fn break_end(break_id: String) -> Self {
        Self {
            kind: ClockEventKind::BreakEnd,
            attendance_id: None,
            break_id: Some(break_id),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AttendanceState {
    pub current_attendance: Option<AttendanceResponse>,
    pub attendance_history: Vec<AttendanceResponse>,
    pub today_status: Option<AttendanceStatusResponse>,
    pub today_holiday_reason: Option<String>,
    pub last_refresh_error: Option<String>,
    pub range_from: Option<NaiveDate>,
    pub range_to: Option<NaiveDate>,
    pub loading: bool,
}

pub fn use_attendance() -> (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>) {
    let (attendance_state, set_attendance_state) = create_signal(AttendanceState::default());
    (attendance_state, set_attendance_state)
}

pub async fn clock_in(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    match api.clock_in().await {
        Ok(attendance) => {
            set_attendance_state.update(|state| {
                state.current_attendance = Some(attendance);
                state.loading = false;
            });
            Ok(())
        }
        Err(error) => {
            set_attendance_state.update(|state| state.loading = false);
            Err(error)
        }
    }
}

pub async fn clock_out(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    match api.clock_out().await {
        Ok(attendance) => {
            set_attendance_state.update(|state| {
                state.current_attendance = Some(attendance);
                state.loading = false;
            });
            Ok(())
        }
        Err(error) => {
            set_attendance_state.update(|state| state.loading = false);
            Err(error)
        }
    }
}

pub async fn start_break(api: &ApiClient, attendance_id: &str) -> Result<(), String> {
    api.break_start(attendance_id).await.map(|_| ())
}

pub async fn end_break(api: &ApiClient, break_id: &str) -> Result<(), String> {
    api.break_end(break_id).await.map(|_| ())
}

pub async fn load_attendance_range(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    match api.get_my_attendance_range(from, to).await {
        Ok(history) => {
            set_attendance_state.update(|state| {
                state.attendance_history = history;
                state.range_from = from;
                state.range_to = to;
                state.loading = false;
            });
            Ok(())
        }
        Err(error) => {
            set_attendance_state.update(|state| state.loading = false);
            Err(error)
        }
    }
}

pub async fn load_today_status(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    match api.get_attendance_status(None).await {
        Ok(status) => {
            set_attendance_state.update(|state| {
                state.today_status = Some(status);
                state.loading = false;
            });
            Ok(())
        }
        Err(e) => {
            set_attendance_state.update(|state| state.loading = false);
            Err(e)
        }
    }
}

pub async fn load_today_holiday_reason(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    let today = today_in_app_tz();
    match api.check_holiday(today).await {
        Ok(response) => {
            set_attendance_state.update(|state| {
                if response.is_holiday {
                    state.today_holiday_reason = response.reason;
                } else {
                    state.today_holiday_reason = None;
                }
            });
            Ok(())
        }
        Err(e) => {
            set_attendance_state.update(|state| state.today_holiday_reason = None);
            Err(e)
        }
    }
}

pub async fn refresh_today_context(
    api: &ApiClient,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    if let Err(err) = load_today_status(api, set_attendance_state).await {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    let today = today_in_app_tz();
    if let Err(err) =
        load_attendance_range(api, set_attendance_state, Some(today), Some(today)).await
    {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    if let Err(err) = load_today_holiday_reason(api, set_attendance_state).await {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    set_attendance_state.update(|state| state.last_refresh_error = None);
    Ok(())
}

pub fn describe_holiday_reason(code: &str) -> &'static str {
    match code {
        "public holiday" => "祝日",
        "weekly holiday" => "定休日",
        "forced holiday" => "特別休日",
        _ => "休日",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn default_state_has_no_holiday_reason() {
        let state = AttendanceState::default();
        assert!(state.today_holiday_reason.is_none());
    }

    #[wasm_bindgen_test]
    fn describe_reason_maps_known_values() {
        assert_eq!(describe_holiday_reason("public holiday"), "祝日");
        assert_eq!(describe_holiday_reason("weekly holiday"), "定休日");
        assert_eq!(describe_holiday_reason("forced holiday"), "特別休日");
    }

    #[wasm_bindgen_test]
    fn describe_reason_falls_back_for_unknown() {
        assert_eq!(describe_holiday_reason("custom"), "休日");
    }

    #[wasm_bindgen_test]
    fn attendance_state_default_has_no_refresh_error() {
        let state = AttendanceState::default();
        assert!(state.last_refresh_error.is_none());
    }

    #[wasm_bindgen_test]
    fn can_store_refresh_error_message() {
        let state = AttendanceState {
            last_refresh_error: Some("network error".into()),
            ..Default::default()
        };
        assert_eq!(state.last_refresh_error.as_deref(), Some("network error"));
    }
}

// load_attendance_summary was unused; consider adding a call site before reintroducing.
