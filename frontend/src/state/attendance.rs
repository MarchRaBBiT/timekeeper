use crate::api::{ApiClient, AttendanceResponse, AttendanceStatusResponse};
use chrono::NaiveDate;
use leptos::*;

#[derive(Debug, Clone)]
pub struct AttendanceState {
    pub current_attendance: Option<AttendanceResponse>,
    pub attendance_history: Vec<AttendanceResponse>,
    pub today_status: Option<AttendanceStatusResponse>,
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
            range_from: None,
            range_to: None,
            loading: false,
        }
    }
}

pub fn use_attendance() -> (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>) {
    let (attendance_state, set_attendance_state) = create_signal(AttendanceState::default());
    (attendance_state, set_attendance_state)
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

// load_attendance_summary was unused; consider adding a call site before reintroducing.
