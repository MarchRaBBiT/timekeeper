use crate::api::ApiClient;
use crate::pages::attendance::{
    repository,
    utils::{month_bounds, AttendanceFormState},
};
use crate::state::attendance::{
    load_attendance_range, refresh_today_context, use_attendance, AttendanceState,
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
    pub range_error: RwSignal<Option<String>>,
    pub export_error: RwSignal<Option<String>>,
    pub export_success: RwSignal<Option<String>>,
}

impl AttendanceViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().expect("ApiClient should be provided");
        let (state, set_state) = use_attendance();
        let initial_today = today_in_app_tz();

        let form_state = AttendanceFormState::new();
        form_state.set_range(initial_today, initial_today);

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

        let api_for_export = api.clone();
        let export_action = create_action(move |payload: &ExportPayload| {
            let api = api_for_export.clone();
            let request = payload.clone();
            async move {
                repository::export_attendance_csv(
                    &api,
                    request.from.as_deref(),
                    request.to.as_deref(),
                )
                .await
            }
        });

        let range_error = create_rw_signal(None);
        let export_error = create_rw_signal(None);
        let export_success = create_rw_signal(None);

        {
            let export_error = export_error;
            let export_success = export_success;
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
