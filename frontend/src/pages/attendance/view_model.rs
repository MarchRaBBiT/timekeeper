use super::repository;
use super::utils::{month_bounds, AttendanceFormState};
use crate::api::ApiClient;
use crate::state::attendance::{
    self as attendance_state, load_attendance_range, refresh_today_context, use_attendance,
    AttendanceState, ClockEventKind, ClockEventPayload,
};
use crate::utils::time::today_in_app_tz;
use chrono::{Datelike, NaiveDate};
use leptos::{ev::MouseEvent, *};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HistoryQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub token: u32,
}

impl HistoryQuery {
    pub fn new(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self { from, to, token: 0 }
    }

    pub fn with_range(self, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self {
            from,
            to,
            token: self.token.wrapping_add(1),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HolidayQuery {
    pub year: i32,
    pub month: u32,
    pub token: u32,
}

impl HolidayQuery {
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            token: 0,
        }
    }

    pub fn with_period(self, year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            token: self.token.wrapping_add(1),
        }
    }

    pub fn refresh(self) -> Self {
        Self {
            year: self.year,
            month: self.month,
            token: self.token.wrapping_add(1),
        }
    }
}

#[derive(Clone, Default)]
pub struct ExportPayload {
    pub from: Option<String>,
    pub to: Option<String>,
}

impl ExportPayload {
    pub fn from_dates(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self {
            from: from.map(|date| date.format("%Y-%m-%d").to_string()),
            to: to.map(|date| date.format("%Y-%m-%d").to_string()),
        }
    }
}

#[derive(Clone)]
pub struct AttendanceViewModel {
    pub api: ApiClient,
    pub state: (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>),
    pub form_state: AttendanceFormState,
    pub history_query: RwSignal<HistoryQuery>,
    pub history_resource: Resource<HistoryQuery, Result<(), String>>,
    pub holiday_query: RwSignal<HolidayQuery>,
    pub holiday_resource:
        Resource<HolidayQuery, Result<Vec<crate::api::HolidayCalendarEntry>, String>>,
    pub context_resource: Resource<(), Result<(), String>>,
    pub export_action: Action<ExportPayload, Result<Value, String>>,
    pub clock_action: Action<ClockEventPayload, Result<(), String>>,
    pub clock_message: RwSignal<Option<String>>,
    pub last_clock_event: RwSignal<Option<ClockEventKind>>,
    pub range_error: RwSignal<Option<String>>,
    pub export_error: RwSignal<Option<String>>,
    pub export_success: RwSignal<Option<String>>,
}

impl AttendanceViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
        let (state, set_state) = use_attendance();
        let initial_today = today_in_app_tz();

        let form_state = AttendanceFormState::new();
        form_state.set_range(initial_today, initial_today);

        let api_clone = api.clone();
        let export_action = create_action(move |payload: &ExportPayload| {
            let api = api_clone.clone();
            let payload = payload.clone();
            async move {
                api.export_my_attendance_filtered(payload.from.as_deref(), payload.to.as_deref())
                    .await
            }
        });

        let history_query =
            create_rw_signal(HistoryQuery::new(Some(initial_today), Some(initial_today)));
        let api_for_history = api.clone();
        let history_resource = create_resource(
            move || history_query.get(),
            move |query| {
                let api = api_for_history.clone();
                async move { load_attendance_range(&api, set_state, query.from, query.to).await }
            },
        );

        let holiday_query = create_rw_signal(HolidayQuery::new(
            initial_today.year(),
            initial_today.month(),
        ));
        let api_for_holiday = api.clone();
        let holiday_resource = create_resource(
            move || holiday_query.get(),
            move |query| {
                let api = api_for_holiday.clone();
                async move { repository::fetch_monthly_holidays(&api, query.year, query.month).await }
            },
        );

        let api_for_context = api.clone();
        let context_resource = create_resource(
            || (),
            move |_| {
                let api = api_for_context.clone();
                async move { refresh_today_context(&api, set_state).await }
            },
        );

        let api_for_clock = api.clone();
        let clock_action = create_action(move |payload: &ClockEventPayload| {
            let api = api_for_clock.clone();
            let set_attendance_state = set_state;
            let payload = payload.clone();
            async move {
                match payload.kind {
                    ClockEventKind::ClockIn => {
                        attendance_state::clock_in(&api, set_attendance_state).await?
                    }
                    ClockEventKind::ClockOut => {
                        attendance_state::clock_out(&api, set_attendance_state).await?
                    }
                    ClockEventKind::BreakStart => {
                        let attendance_id = payload
                            .attendance_id
                            .as_deref()
                            .ok_or_else(|| "出勤レコードが見つかりません。".to_string())?;
                        attendance_state::start_break(&api, attendance_id).await?
                    }
                    ClockEventKind::BreakEnd => {
                        let break_id = payload
                            .break_id
                            .as_deref()
                            .ok_or_else(|| "休憩レコードが見つかりません。".to_string())?;
                        attendance_state::end_break(&api, break_id).await?
                    }
                };
                refresh_today_context(&api, set_attendance_state).await
            }
        });

        let clock_message = create_rw_signal(None);
        let last_clock_event = create_rw_signal(None);
        let range_error = create_rw_signal(None);
        let export_error = create_rw_signal(None);
        let export_success = create_rw_signal(None);

        {
            create_effect(move |_| {
                if let Some(result) = clock_action.value().get() {
                    match result {
                        Ok(_) => {
                            let success = match last_clock_event.get_untracked() {
                                Some(ClockEventKind::ClockIn) => "出勤しました。",
                                Some(ClockEventKind::BreakStart) => "休憩を開始しました。",
                                Some(ClockEventKind::BreakEnd) => "休憩を終了しました。",
                                Some(ClockEventKind::ClockOut) => "退勤しました。",
                                None => "操作が完了しました。",
                            };
                            clock_message.set(Some(success.into()));
                        }
                        Err(err) => clock_message.set(Some(err)),
                    }
                }
            });
        }

        {
            create_effect(move |_| {
                if let Some(result) = export_action.value().get() {
                    match result {
                        Ok(payload) => {
                            let filename = payload
                                .get("filename")
                                .and_then(|v| v.as_str())
                                .unwrap_or("my_attendance.csv");
                            let csv = payload
                                .get("csv_data")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            match crate::utils::trigger_csv_download(filename, csv) {
                                Ok(_) => {
                                    export_success
                                        .set(Some(format!("{filename} をダウンロードしました。")));
                                }
                                Err(err) => {
                                    export_error.set(Some(format!(
                                        "CSVのダウンロードに失敗しました: {err}"
                                    )));
                                }
                            }
                        }
                        Err(err) => export_error.set(Some(err)),
                    }
                }
            });
        }

        Self {
            api,
            state: (state, set_state),
            form_state,
            history_query,
            history_resource,
            holiday_query,
            holiday_resource,
            context_resource,
            export_action,
            clock_action,
            clock_message,
            last_clock_event,
            range_error,
            export_error,
            export_success,
        }
    }

    pub fn on_select_current_month(&self) -> impl Fn(MouseEvent) {
        let form_state = self.form_state.clone();
        let history_query = self.history_query;
        let holiday_query = self.holiday_query;
        let range_error = self.range_error;
        let export_error = self.export_error;
        let export_success = self.export_success;

        move |_ev| {
            range_error.set(None);
            export_error.set(None);
            export_success.set(None);
            let today = today_in_app_tz();
            let Some((first_day, last_day)) = month_bounds(today) else {
                return;
            };
            form_state.set_range(first_day, last_day);
            history_query
                .update(|query| *query = query.with_range(Some(first_day), Some(last_day)));
            holiday_query.update(|query| {
                *query = query.with_period(first_day.year(), first_day.month());
            });
        }
    }

    pub fn on_load_range(&self) -> impl Fn(MouseEvent) {
        let form_state = self.form_state.clone();
        let history_query = self.history_query;
        let holiday_query = self.holiday_query;
        let range_error = self.range_error;
        let export_error = self.export_error;
        let export_success = self.export_success;

        move |_ev| {
            export_error.set(None);
            export_success.set(None);
            match form_state.to_payload() {
                Ok((from, to)) => {
                    range_error.set(None);
                    history_query.update(|query| *query = query.with_range(from, to));
                    if let Some(date) = from {
                        holiday_query.update(|query| {
                            *query = query.with_period(date.year(), date.month());
                        });
                    }
                }
                Err(err) => range_error.set(Some(err)),
            }
        }
    }

    pub fn on_export_csv(&self) -> impl Fn(MouseEvent) {
        let form_state = self.form_state.clone();
        let export_action = self.export_action;
        let export_error = self.export_error;
        let export_success = self.export_success;

        move |_ev| {
            export_error.set(None);
            export_success.set(None);
            match form_state.to_payload() {
                Ok((from, to)) => {
                    let payload = ExportPayload::from_dates(from, to);
                    export_action.dispatch(payload);
                }
                Err(err) => export_error.set(Some(err)),
            }
        }
    }

    pub fn on_refresh_holidays(&self) -> impl Fn(()) {
        let holiday_query = self.holiday_query;
        move |_| {
            holiday_query.update(|query| {
                *query = query.refresh();
            })
        }
    }

    pub fn handle_clock_in(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::ClockIn));
            clock_action.dispatch(ClockEventPayload::clock_in());
        }
    }

    pub fn handle_clock_out(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::ClockOut));
            clock_action.dispatch(ClockEventPayload::clock_out());
        }
    }

    pub fn handle_break_start(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let Some(status) = state.get().today_status.clone() else {
                clock_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "clocked_in" {
                clock_message.set(Some("出勤中のみ休憩を開始できます。".into()));
                return;
            }
            let Some(att_id) = status.attendance_id.clone() else {
                clock_message.set(Some("出勤レコードが見つかりません。".into()));
                return;
            };
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::BreakStart));
            clock_action.dispatch(ClockEventPayload::break_start(att_id));
        }
    }

    pub fn handle_break_end(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let Some(status) = state.get().today_status.clone() else {
                clock_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "on_break" {
                clock_message.set(Some("休憩中のみ休憩を終了できます。".into()));
                return;
            }
            let Some(break_id) = status.active_break_id.clone() else {
                clock_message.set(Some("休憩レコードが見つかりません。".into()));
                return;
            };
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::BreakEnd));
            clock_action.dispatch(ClockEventPayload::break_end(break_id));
        }
    }
}

pub fn use_attendance_view_model() -> AttendanceViewModel {
    match use_context::<AttendanceViewModel>() {
        Some(vm) => vm,
        None => {
            let vm = AttendanceViewModel::new();
            provide_context(vm.clone());
            vm
        }
    }
}
