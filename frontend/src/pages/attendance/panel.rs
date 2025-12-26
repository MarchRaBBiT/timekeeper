use super::{
    components::{
        alerts::HolidayAlerts, form::RangeFormSection, history::HistorySection,
        summary::SummarySection,
    },
    layout::AttendanceFrame,
    repository,
    utils::{month_bounds, AttendanceFormState},
};
use crate::{
    state::attendance::{load_attendance_range, refresh_today_context, use_attendance},
    utils::{time::today_in_app_tz, trigger_csv_download},
};
use chrono::{Datelike, NaiveDate};
use leptos::{ev::MouseEvent, *};

#[derive(Clone, Copy, PartialEq, Eq)]
struct HistoryQuery {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    token: u32,
}

impl HistoryQuery {
    fn new(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self { from, to, token: 0 }
    }

    fn with_range(self, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self {
            from,
            to,
            token: self.token.wrapping_add(1),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HolidayQuery {
    year: i32,
    month: u32,
    token: u32,
}

impl HolidayQuery {
    fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            token: 0,
        }
    }

    fn with_period(self, year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            token: self.token.wrapping_add(1),
        }
    }

    fn refresh(self) -> Self {
        Self {
            year: self.year,
            month: self.month,
            token: self.token.wrapping_add(1),
        }
    }
}

#[derive(Clone, Default)]
struct ExportPayload {
    from: Option<String>,
    to: Option<String>,
}

impl ExportPayload {
    fn from_dates(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self {
            from: from.map(|date| date.format("%Y-%m-%d").to_string()),
            to: to.map(|date| date.format("%Y-%m-%d").to_string()),
        }
    }
}

#[component]
pub fn AttendancePage() -> impl IntoView {
    view! { <AttendancePanel /> }
}

#[component]
pub fn AttendancePanel() -> impl IntoView {
    let (state, set_state) = use_attendance();
    let initial_today = today_in_app_tz();

    let form_state = AttendanceFormState::new();
    form_state.set_range(initial_today, initial_today);
    let from_input = form_state.start_date_signal();
    let to_input = form_state.end_date_signal();

    let (history_query, set_history_query) =
        create_signal(HistoryQuery::new(Some(initial_today), Some(initial_today)));
    let history_resource = {
        create_resource(
            move || history_query.get(),
            move |query| {
                let api =
                    use_context::<crate::api::ApiClient>().expect("ApiClient should be provided");
                async move { load_attendance_range(&api, set_state, query.from, query.to).await }
            },
        )
    };
    let history_loading = history_resource.loading();
    let history_error =
        Signal::derive(move || history_resource.get().and_then(|result| result.err()));

    let (holiday_query, set_holiday_query) = create_signal(HolidayQuery::new(
        initial_today.year(),
        initial_today.month(),
    ));
    let holiday_resource = create_resource(
        move || holiday_query.get(),
        move |query| {
            let api = use_context::<crate::api::ApiClient>().expect("ApiClient should be provided");
            async move { repository::fetch_monthly_holidays(&api, query.year, query.month).await }
        },
    );
    let holiday_loading = holiday_resource.loading();
    let holiday_entries = Signal::derive(move || {
        holiday_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default()
    });
    let holiday_error =
        Signal::derive(move || holiday_resource.get().and_then(|result| result.err()));
    let active_holiday_period =
        Signal::derive(move || holiday_query.with(|query| (query.year, query.month)));

    let _context_resource = {
        create_resource(
            || (),
            move |_| {
                let api =
                    use_context::<crate::api::ApiClient>().expect("ApiClient should be provided");
                async move { refresh_today_context(&api, set_state).await }
            },
        )
    };

    let export_error = create_rw_signal(Option::<String>::None);
    let export_success = create_rw_signal(Option::<String>::None);
    let range_error = create_rw_signal(None::<String>);
    let export_action = create_action(move |payload: &ExportPayload| {
        let request = payload.clone();
        async move {
            let api = use_context::<crate::api::ApiClient>().expect("ApiClient should be provided");
            repository::export_attendance_csv(&api, request.from.as_deref(), request.to.as_deref())
                .await
        }
    });
    let exporting = export_action.pending();
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
                        match trigger_csv_download(filename, csv) {
                            Ok(_) => {
                                export_success
                                    .set(Some(format!("{filename} をダウンロードしました。")));
                            }
                            Err(err) => {
                                export_error
                                    .set(Some(format!("CSVのダウンロードに失敗しました: {err}")));
                            }
                        }
                    }
                    Err(err) => export_error.set(Some(err)),
                }
            }
        });
    }

    let on_select_current_month = {
        let form_state = form_state.clone();
        Callback::new(move |_ev: MouseEvent| {
            range_error.set(None);
            export_error.set(None);
            export_success.set(None);
            let today = today_in_app_tz();
            let Some((first_day, last_day)) = month_bounds(today) else {
                return;
            };
            form_state.set_range(first_day, last_day);
            set_history_query
                .update(|query| *query = query.with_range(Some(first_day), Some(last_day)));
            set_holiday_query.update(|query| {
                *query = query.with_period(first_day.year(), first_day.month());
            });
        })
    };

    let on_load_range = {
        let form_state = form_state.clone();
        Callback::new(move |_ev: MouseEvent| {
            export_error.set(None);
            export_success.set(None);
            match form_state.to_payload() {
                Ok((from, to)) => {
                    range_error.set(None);
                    set_history_query.update(|query| *query = query.with_range(from, to));
                    if let Some(date) = from {
                        set_holiday_query.update(|query| {
                            *query = query.with_period(date.year(), date.month());
                        });
                    }
                }
                Err(err) => range_error.set(Some(err)),
            }
        })
    };

    let on_export_csv = {
        let form_state = form_state.clone();
        Callback::new(move |_ev: MouseEvent| {
            export_error.set(None);
            export_success.set(None);
            match form_state.to_payload() {
                Ok((from, to)) => {
                    let payload = ExportPayload::from_dates(from, to);
                    export_action.dispatch(payload);
                }
                Err(err) => export_error.set(Some(err)),
            }
        })
    };

    let last_refresh_error = Signal::derive(move || state.get().last_refresh_error.clone());
    let refresh_holidays = {
        Callback::new(move |_| {
            set_holiday_query.update(|query| {
                *query = query.refresh();
            })
        })
    };
    let history_signal = Signal::derive(move || state.get().attendance_history.clone());

    view! {
        <AttendanceFrame>
            <div class="space-y-6">
                <SummarySection state=state set_state=set_state />
                <RangeFormSection
                    from_input=from_input
                    to_input=to_input
                    exporting={exporting.into()}
                    export_error={export_error.read_only()}
                    export_success={export_success.read_only()}
                    history_loading=history_loading
                    history_error={history_error}
                    range_error={range_error.read_only()}
                    last_refresh_error=last_refresh_error
                    on_select_current_month=on_select_current_month
                    on_load_range=on_load_range
                    on_export_csv=on_export_csv
                />
                <HolidayAlerts
                    holiday_entries={holiday_entries}
                    loading={holiday_loading}
                    error={holiday_error}
                    active_period={active_holiday_period}
                    on_refresh=refresh_holidays
                />
                <HistorySection
                    history=history_signal
                    holiday_entries={holiday_entries}
                    loading=history_loading
                    error={history_error}
                />
            </div>
        </AttendanceFrame>
    }
}
