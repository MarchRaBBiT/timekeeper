use crate::api::{ApiClient, AttendanceResponse, AttendanceStatusResponse};
use crate::utils::time::today_in_app_tz;
use chrono::NaiveDate;
use leptos::*;

#[derive(Debug, Clone)]
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

impl Default for AttendanceState {
    fn default() -> Self {
        Self {
            current_attendance: None,
            attendance_history: Vec::new(),
            today_status: None,
            today_holiday_reason: None,
            last_refresh_error: None,
            range_from: None,
            range_to: None,
            loading: false,
        }
    }
}

pub fn use_attendance() -> (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>) {
    if let Some(ctx) = use_context::<(ReadSignal<AttendanceState>, WriteSignal<AttendanceState>)>()
    {
        ctx
    } else {
        let signals = create_signal(AttendanceState::default());
        provide_context(signals);
        signals
    }
}

pub async fn clock_in(set_attendance_state: WriteSignal<AttendanceState>) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    let api_client = ApiClient::new();
    match api_client.clock_in().await {
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

pub async fn clock_out(set_attendance_state: WriteSignal<AttendanceState>) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    let api_client = ApiClient::new();
    match api_client.clock_out().await {
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

pub async fn load_attendance_range(
    set_attendance_state: WriteSignal<AttendanceState>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    let api_client = ApiClient::new();
    match api_client.get_my_attendance_range(from, to).await {
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
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    set_attendance_state.update(|state| state.loading = true);
    let api_client = ApiClient::new();
    match api_client.get_attendance_status(None).await {
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
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    let api_client = ApiClient::new();
    let today = today_in_app_tz();
    match api_client.check_holiday(today).await {
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
        let mut state = AttendanceState::default();
        state.last_refresh_error = Some("network error".into());
        assert_eq!(state.last_refresh_error.as_deref(), Some("network error"));
    }

    #[wasm_bindgen_test]
    fn use_attendance_reuses_context_within_scope() {
        let _runtime = leptos_reactive::create_runtime();
        let (_, setter) = super::use_attendance();
        setter.update(|state| state.today_holiday_reason = Some("shared".into()));

        let (reader, _) = super::use_attendance();
        assert_eq!(reader.get().today_holiday_reason, Some("shared".into()));
    }
}

pub async fn refresh_today_context(
    set_attendance_state: WriteSignal<AttendanceState>,
) -> Result<(), String> {
    let status_set = set_attendance_state.clone();
    if let Err(err) = load_today_status(status_set).await {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    let today = today_in_app_tz();
    let range_set = set_attendance_state.clone();
    if let Err(err) = load_attendance_range(range_set, Some(today), Some(today)).await {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    let holiday_set = set_attendance_state.clone();
    if let Err(err) = load_today_holiday_reason(holiday_set).await {
        set_attendance_state.update(|state| state.last_refresh_error = Some(err.clone()));
        return Err(err);
    }

    set_attendance_state.update(|state| state.last_refresh_error = None);
    Ok(())
}

// load_attendance_summary was unused; consider adding a call site before reintroducing.
