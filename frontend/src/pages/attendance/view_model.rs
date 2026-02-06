use super::repository;
use super::utils::{month_bounds, AttendanceFormState};
use crate::api::{ApiClient, ApiError};
use crate::state::attendance::{
    self as attendance_state, load_attendance_range, refresh_today_context, use_attendance,
    AttendanceState, ClockEventKind, ClockEventPayload, ClockMessage,
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
    pub state: (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>),
    pub form_state: AttendanceFormState,
    pub history_query: RwSignal<HistoryQuery>,
    pub history_resource: Resource<HistoryQuery, Result<(), ApiError>>,
    pub holiday_query: RwSignal<HolidayQuery>,
    pub holiday_resource:
        Resource<HolidayQuery, Result<Vec<crate::api::HolidayCalendarEntry>, ApiError>>,
    pub context_resource: Resource<(), Result<(), ApiError>>,
    pub export_action: Action<ExportPayload, Result<Value, ApiError>>,
    pub clock_action: Action<ClockEventPayload, Result<(), ApiError>>,
    pub clock_message: RwSignal<Option<ClockMessage>>,
    pub last_clock_event: RwSignal<Option<ClockEventKind>>,
    pub range_error: RwSignal<Option<String>>,
    pub export_error: RwSignal<Option<ApiError>>,
    pub export_success: RwSignal<Option<String>>,
}

fn clock_success_message(last_event: Option<ClockEventKind>) -> &'static str {
    match last_event {
        Some(ClockEventKind::ClockIn) => "出勤しました。",
        Some(ClockEventKind::BreakStart) => "休憩を開始しました。",
        Some(ClockEventKind::BreakEnd) => "休憩を終了しました。",
        Some(ClockEventKind::ClockOut) => "退勤しました。",
        None => "操作が完了しました。",
    }
}

fn map_clock_action_result(
    last_event: Option<ClockEventKind>,
    result: Result<(), ApiError>,
) -> ClockMessage {
    match result {
        Ok(_) => ClockMessage::Success(clock_success_message(last_event).to_string()),
        Err(err) => ClockMessage::Error(err),
    }
}

fn map_export_action_result(result: Result<Value, ApiError>) -> (Option<String>, Option<ApiError>) {
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
                Ok(_) => (Some(format!("{filename} をダウンロードしました。")), None),
                Err(err) => (
                    None,
                    Some(ApiError::unknown(format!(
                        "CSVのダウンロードに失敗しました: {err}"
                    ))),
                ),
            }
        }
        Err(err) => (None, Some(err)),
    }
}

fn apply_optional_clock_action_result(
    result: Option<Result<(), ApiError>>,
    last_clock_event: RwSignal<Option<ClockEventKind>>,
    clock_message: RwSignal<Option<ClockMessage>>,
) {
    if let Some(result) = result {
        let mapped = map_clock_action_result(last_clock_event.get_untracked(), result);
        clock_message.set(Some(mapped));
    }
}

fn apply_optional_export_action_result(
    result: Option<Result<Value, ApiError>>,
    export_success: RwSignal<Option<String>>,
    export_error: RwSignal<Option<ApiError>>,
) {
    if let Some(result) = result {
        let (success, error) = map_export_action_result(result);
        export_success.set(success);
        export_error.set(error);
    }
}

fn select_current_month(
    form_state: &AttendanceFormState,
    history_query: RwSignal<HistoryQuery>,
    holiday_query: RwSignal<HolidayQuery>,
    range_error: RwSignal<Option<String>>,
    export_error: RwSignal<Option<ApiError>>,
    export_success: RwSignal<Option<String>>,
) {
    range_error.set(None);
    export_error.set(None);
    export_success.set(None);
    let today = today_in_app_tz();
    let Some((first_day, last_day)) = month_bounds(today) else {
        return;
    };
    form_state.set_range(first_day, last_day);
    history_query.update(|query| *query = query.with_range(Some(first_day), Some(last_day)));
    holiday_query.update(|query| {
        *query = query.with_period(first_day.year(), first_day.month());
    });
}

fn load_selected_range(
    form_state: &AttendanceFormState,
    history_query: RwSignal<HistoryQuery>,
    holiday_query: RwSignal<HolidayQuery>,
    range_error: RwSignal<Option<String>>,
    export_error: RwSignal<Option<ApiError>>,
    export_success: RwSignal<Option<String>>,
) {
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
        Err(err) => range_error.set(Some(err.error)),
    }
}

fn export_csv_from_form<F>(
    form_state: &AttendanceFormState,
    export_error: RwSignal<Option<ApiError>>,
    export_success: RwSignal<Option<String>>,
    dispatch_export: F,
) where
    F: FnOnce(ExportPayload),
{
    export_error.set(None);
    export_success.set(None);
    match form_state.to_payload() {
        Ok((from, to)) => dispatch_export(ExportPayload::from_dates(from, to)),
        Err(err) => export_error.set(Some(err)),
    }
}

fn resolve_clock_in_payload(
    pending: bool,
    clock_message: RwSignal<Option<ClockMessage>>,
    last_event: RwSignal<Option<ClockEventKind>>,
) -> Option<ClockEventPayload> {
    if pending {
        return None;
    }
    clock_message.set(None);
    last_event.set(Some(ClockEventKind::ClockIn));
    Some(ClockEventPayload::clock_in())
}

fn resolve_clock_out_payload(
    pending: bool,
    clock_message: RwSignal<Option<ClockMessage>>,
    last_event: RwSignal<Option<ClockEventKind>>,
) -> Option<ClockEventPayload> {
    if pending {
        return None;
    }
    clock_message.set(None);
    last_event.set(Some(ClockEventKind::ClockOut));
    Some(ClockEventPayload::clock_out())
}

fn resolve_break_start_payload(
    pending: bool,
    status: Option<&crate::api::AttendanceStatusResponse>,
    clock_message: RwSignal<Option<ClockMessage>>,
    last_event: RwSignal<Option<ClockEventKind>>,
) -> Option<ClockEventPayload> {
    if pending {
        return None;
    }
    let att_id = match break_start_attendance_id(status) {
        Ok(id) => id,
        Err(err) => {
            clock_message.set(Some(ClockMessage::Error(err)));
            return None;
        }
    };
    clock_message.set(None);
    last_event.set(Some(ClockEventKind::BreakStart));
    Some(ClockEventPayload::break_start(att_id))
}

fn resolve_break_end_payload(
    pending: bool,
    status: Option<&crate::api::AttendanceStatusResponse>,
    clock_message: RwSignal<Option<ClockMessage>>,
    last_event: RwSignal<Option<ClockEventKind>>,
) -> Option<ClockEventPayload> {
    if pending {
        return None;
    }
    let break_id = match break_end_break_id(status) {
        Ok(id) => id,
        Err(err) => {
            clock_message.set(Some(ClockMessage::Error(err)));
            return None;
        }
    };
    clock_message.set(None);
    last_event.set(Some(ClockEventKind::BreakEnd));
    Some(ClockEventPayload::break_end(break_id))
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
                        let attendance_id = payload.attendance_id.as_deref().ok_or_else(|| {
                            ApiError::validation("出勤レコードが見つかりません。")
                        })?;
                        attendance_state::start_break(&api, attendance_id).await?
                    }
                    ClockEventKind::BreakEnd => {
                        let break_id = payload.break_id.as_deref().ok_or_else(|| {
                            ApiError::validation("休憩レコードが見つかりません。")
                        })?;
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
                apply_optional_clock_action_result(
                    clock_action.value().get(),
                    last_clock_event,
                    clock_message,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_export_action_result(
                    export_action.value().get(),
                    export_success,
                    export_error,
                );
            });
        }

        Self {
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
            select_current_month(
                &form_state,
                history_query,
                holiday_query,
                range_error,
                export_error,
                export_success,
            );
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
            load_selected_range(
                &form_state,
                history_query,
                holiday_query,
                range_error,
                export_error,
                export_success,
            );
        }
    }

    pub fn on_export_csv(&self) -> impl Fn(MouseEvent) {
        let form_state = self.form_state.clone();
        let export_action = self.export_action;
        let export_error = self.export_error;
        let export_success = self.export_success;

        move |_ev| {
            export_csv_from_form(&form_state, export_error, export_success, |payload| {
                export_action.dispatch(payload);
            });
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
            if let Some(payload) = resolve_clock_in_payload(
                clock_action.pending().get_untracked(),
                clock_message,
                last_event,
            ) {
                clock_action.dispatch(payload);
            }
        }
    }

    pub fn handle_clock_out(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        move |_| {
            if let Some(payload) = resolve_clock_out_payload(
                clock_action.pending().get_untracked(),
                clock_message,
                last_event,
            ) {
                clock_action.dispatch(payload);
            }
        }
    }

    pub fn handle_break_start(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.state;
        move |_| {
            let status = state.with(|s| s.today_status.clone());
            if let Some(payload) = resolve_break_start_payload(
                clock_action.pending().get_untracked(),
                status.as_ref(),
                clock_message,
                last_event,
            ) {
                clock_action.dispatch(payload);
            }
        }
    }

    pub fn handle_break_end(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.state;
        move |_| {
            let status = state.with(|s| s.today_status.clone());
            if let Some(payload) = resolve_break_end_payload(
                clock_action.pending().get_untracked(),
                status.as_ref(),
                clock_message,
                last_event,
            ) {
                clock_action.dispatch(payload);
            }
        }
    }
}

fn break_start_attendance_id(
    status: Option<&crate::api::AttendanceStatusResponse>,
) -> Result<String, ApiError> {
    let Some(status) = status else {
        return Err(ApiError::validation("ステータスを取得できません。"));
    };
    if status.status != "clocked_in" {
        return Err(ApiError::validation("出勤中のみ休憩を開始できます。"));
    }
    status
        .attendance_id
        .clone()
        .ok_or_else(|| ApiError::validation("出勤レコードが見つかりません。"))
}

fn break_end_break_id(
    status: Option<&crate::api::AttendanceStatusResponse>,
) -> Result<String, ApiError> {
    let Some(status) = status else {
        return Err(ApiError::validation("ステータスを取得できません。"));
    };
    if status.status != "on_break" {
        return Err(ApiError::validation("休憩中のみ休憩を終了できます。"));
    }
    status
        .active_break_id
        .clone()
        .ok_or_else(|| ApiError::validation("休憩レコードが見つかりません。"))
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

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use leptos::create_runtime;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[wasm_bindgen_test]
    fn attendance_view_model_sets_up_context_refresh() {
        with_runtime(|| {
            let vm = AttendanceViewModel::new();
            let _ = vm.context_resource.loading().get();
        });
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::ssr::{with_local_runtime, with_local_runtime_async, with_runtime};

    async fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
        for _ in 0..100 {
            if condition() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    #[test]
    fn wait_until_returns_false_when_condition_never_met() {
        with_local_runtime_async(|| async {
            assert!(!wait_until(|| false).await);
        });
    }

    #[test]
    fn history_query_refreshes_token() {
        let query = HistoryQuery::new(None, None);
        let updated = query.with_range(Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()), None);
        assert_eq!(updated.token, query.token.wrapping_add(1));
    }

    #[test]
    fn holiday_query_refreshes_token() {
        let query = HolidayQuery::new(2025, 1);
        let updated = query.with_period(2025, 2);
        assert_eq!(updated.token, query.token.wrapping_add(1));
        let refreshed = updated.refresh();
        assert_eq!(refreshed.token, updated.token.wrapping_add(1));
    }

    #[test]
    fn export_payload_formats_dates() {
        let from = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let payload = ExportPayload::from_dates(Some(from), Some(to));
        assert_eq!(payload.from.as_deref(), Some("2025-01-01"));
        assert_eq!(payload.to.as_deref(), Some("2025-01-31"));

        let open_ended = ExportPayload::from_dates(Some(from), None);
        assert_eq!(open_ended.from.as_deref(), Some("2025-01-01"));
        assert!(open_ended.to.is_none());
    }

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/status");
            then.status(200).json_body(serde_json::json!({
                "status": "clocked_in",
                "attendance_id": "att-1",
                "active_break_id": null,
                "clock_in_time": "2025-01-01T09:00:00",
                "clock_out_time": null
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/me");
            then.status(200).json_body(serde_json::json!([]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/holidays/check");
            then.status(200).json_body(serde_json::json!({
                "is_holiday": false,
                "reason": null
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/holidays/month");
            then.status(200).json_body(serde_json::json!([]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/attendance/clock-in");
            then.status(200).json_body(serde_json::json!({
                "id": "att-1",
                "user_id": "u1",
                "date": "2025-01-01",
                "clock_in_time": "2025-01-01T09:00:00",
                "clock_out_time": null,
                "status": "clocked_in",
                "total_work_hours": null,
                "break_records": []
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/attendance/clock-out");
            then.status(200).json_body(serde_json::json!({
                "id": "att-1",
                "user_id": "u1",
                "date": "2025-01-01",
                "clock_in_time": "2025-01-01T09:00:00",
                "clock_out_time": "2025-01-01T18:00:00",
                "status": "clocked_out",
                "total_work_hours": 8.0,
                "break_records": []
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/attendance/break-start");
            then.status(200).json_body(serde_json::json!({
                "id": "br-1",
                "attendance_id": "att-1",
                "break_start_time": "2025-01-01T12:00:00",
                "break_end_time": null,
                "duration_minutes": null
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/attendance/break-end");
            then.status(200).json_body(serde_json::json!({
                "id": "br-1",
                "attendance_id": "att-1",
                "break_start_time": "2025-01-01T12:00:00",
                "break_end_time": "2025-01-01T12:30:00",
                "duration_minutes": 30
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/attendance/export");
            then.status(200).json_body(serde_json::json!({
                "filename": "attendance.csv",
                "csv_data": "date,hours\n2025-01-01,8"
            }));
        });
        server
    }

    fn status(
        status: &str,
        attendance_id: Option<&str>,
        break_id: Option<&str>,
    ) -> crate::api::AttendanceStatusResponse {
        crate::api::AttendanceStatusResponse {
            status: status.into(),
            attendance_id: attendance_id.map(|v| v.to_string()),
            active_break_id: break_id.map(|v| v.to_string()),
            clock_in_time: None,
            clock_out_time: None,
        }
    }

    #[test]
    fn refresh_holidays_increments_query_token() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let vm = AttendanceViewModel::new();
                let before = vm.holiday_query.get().token;
                let refresh = vm.on_refresh_holidays();
                refresh(());
                let after = vm.holiday_query.get().token;
                assert_eq!(after, before.wrapping_add(1));
            });
        });
    }

    #[test]
    fn break_start_requires_status_clocked_in_and_attendance_id() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let _vm = AttendanceViewModel::new();

                let no_status = break_start_attendance_id(None).unwrap_err();
                assert_eq!(no_status.error, "ステータスを取得できません。");

                let wrong_status =
                    break_start_attendance_id(Some(&status("clocked_out", None, None)))
                        .unwrap_err();
                assert_eq!(wrong_status.error, "出勤中のみ休憩を開始できます。");

                let missing_id =
                    break_start_attendance_id(Some(&status("clocked_in", None, None))).unwrap_err();
                assert_eq!(missing_id.error, "出勤レコードが見つかりません。");

                let ok =
                    break_start_attendance_id(Some(&status("clocked_in", Some("att-1"), None)))
                        .unwrap();
                assert_eq!(ok, "att-1");
            });
        });
    }

    #[test]
    fn break_end_requires_status_on_break_and_break_id() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let _vm = AttendanceViewModel::new();

                let no_status = break_end_break_id(None).unwrap_err();
                assert_eq!(no_status.error, "ステータスを取得できません。");

                let wrong_status =
                    break_end_break_id(Some(&status("clocked_in", Some("att-1"), None)))
                        .unwrap_err();
                assert_eq!(wrong_status.error, "休憩中のみ休憩を終了できます。");

                let missing_id =
                    break_end_break_id(Some(&status("on_break", Some("att-1"), None))).unwrap_err();
                assert_eq!(missing_id.error, "休憩レコードが見つかりません。");

                let ok = break_end_break_id(Some(&status("on_break", Some("att-1"), Some("br-1"))))
                    .unwrap();
                assert_eq!(ok, "br-1");
            });
        });
    }

    #[test]
    fn helper_clock_message_mapping_covers_success_and_error_paths() {
        assert_eq!(
            clock_success_message(Some(ClockEventKind::ClockIn)),
            "出勤しました。"
        );
        assert_eq!(
            clock_success_message(Some(ClockEventKind::BreakStart)),
            "休憩を開始しました。"
        );
        assert_eq!(
            clock_success_message(Some(ClockEventKind::BreakEnd)),
            "休憩を終了しました。"
        );
        assert_eq!(
            clock_success_message(Some(ClockEventKind::ClockOut)),
            "退勤しました。"
        );
        assert_eq!(clock_success_message(None), "操作が完了しました。");

        let success = map_clock_action_result(Some(ClockEventKind::ClockIn), Ok(()));
        assert!(matches!(success, ClockMessage::Success(msg) if msg == "出勤しました。"));

        let failure = map_clock_action_result(None, Err(ApiError::unknown("clock failed")));
        assert!(matches!(failure, ClockMessage::Error(err) if err.error == "clock failed"));

        let default_success = map_clock_action_result(None, Ok(()));
        assert!(
            matches!(default_success, ClockMessage::Success(msg) if msg == "操作が完了しました。")
        );
    }

    #[test]
    fn use_attendance_view_model_reuses_existing_context() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let vm = AttendanceViewModel::new();
                vm.range_error.set(Some("context-error".to_string()));
                provide_context(vm);

                let used = use_attendance_view_model();
                assert_eq!(used.range_error.get().as_deref(), Some("context-error"));
            });
        });
    }

    #[test]
    fn clock_action_dispatch_covers_event_variants_and_validation() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = AttendanceViewModel::new();

            vm.clock_action.dispatch(ClockEventPayload::clock_in());
            assert!(
                wait_until(|| vm.clock_action.value().get().is_some()).await,
                "clock_in result timeout"
            );
            assert!(matches!(vm.clock_action.value().get(), Some(Ok(()))));

            vm.clock_action.dispatch(ClockEventPayload::clock_out());
            assert!(
                wait_until(|| vm.clock_action.pending().get_untracked()).await,
                "clock_out pending timeout"
            );
            assert!(
                wait_until(|| !vm.clock_action.pending().get_untracked()).await,
                "clock_out completion timeout"
            );
            assert!(matches!(vm.clock_action.value().get(), Some(Ok(()))));

            vm.clock_action
                .dispatch(ClockEventPayload::break_start("att-1".to_string()));
            assert!(
                wait_until(|| vm.clock_action.pending().get_untracked()).await,
                "break_start pending timeout"
            );
            assert!(
                wait_until(|| !vm.clock_action.pending().get_untracked()).await,
                "break_start completion timeout"
            );
            assert!(matches!(vm.clock_action.value().get(), Some(Ok(()))));

            vm.clock_action
                .dispatch(ClockEventPayload::break_end("br-1".to_string()));
            assert!(
                wait_until(|| vm.clock_action.pending().get_untracked()).await,
                "break_end pending timeout"
            );
            assert!(
                wait_until(|| !vm.clock_action.pending().get_untracked()).await,
                "break_end completion timeout"
            );
            assert!(matches!(vm.clock_action.value().get(), Some(Ok(()))));

            vm.clock_action.dispatch(ClockEventPayload {
                kind: ClockEventKind::BreakStart,
                attendance_id: None,
                break_id: None,
            });
            assert!(
                wait_until(|| vm.clock_action.pending().get_untracked()).await,
                "missing-break-start-id pending timeout"
            );
            assert!(
                wait_until(|| !vm.clock_action.pending().get_untracked()).await,
                "missing-break-start-id completion timeout"
            );
            assert!(matches!(
                vm.clock_action.value().get(),
                Some(Err(err)) if err.error == "出勤レコードが見つかりません。"
            ));

            vm.clock_action.dispatch(ClockEventPayload {
                kind: ClockEventKind::BreakEnd,
                attendance_id: None,
                break_id: None,
            });
            assert!(
                wait_until(|| vm.clock_action.pending().get_untracked()).await,
                "missing-break-end-id pending timeout"
            );
            assert!(
                wait_until(|| !vm.clock_action.pending().get_untracked()).await,
                "missing-break-end-id completion timeout"
            );
            assert!(matches!(
                vm.clock_action.value().get(),
                Some(Err(err)) if err.error == "休憩レコードが見つかりません。"
            ));

            runtime.dispose();
        });
    }

    #[test]
    fn helper_optional_effect_and_handler_logic_cover_paths() {
        with_runtime(|| {
            let clock_message = create_rw_signal(None);
            let last_clock_event = create_rw_signal(Some(ClockEventKind::ClockIn));
            apply_optional_clock_action_result(None, last_clock_event, clock_message);
            assert!(clock_message.get().is_none());

            apply_optional_clock_action_result(Some(Ok(())), last_clock_event, clock_message);
            assert!(matches!(
                clock_message.get(),
                Some(ClockMessage::Success(msg)) if msg == "出勤しました。"
            ));

            apply_optional_clock_action_result(
                Some(Err(ApiError::unknown("clock result failed"))),
                last_clock_event,
                clock_message,
            );
            assert!(matches!(
                clock_message.get(),
                Some(ClockMessage::Error(err)) if err.error == "clock result failed"
            ));

            let export_success = create_rw_signal(Some("old-success".to_string()));
            let export_error = create_rw_signal(Some(ApiError::unknown("old-error")));
            apply_optional_export_action_result(None, export_success, export_error);
            assert_eq!(export_success.get().as_deref(), Some("old-success"));
            assert_eq!(
                export_error.get().map(|err| err.error),
                Some("old-error".to_string())
            );

            apply_optional_export_action_result(
                Some(Err(ApiError::unknown("export failed"))),
                export_success,
                export_error,
            );
            assert!(export_success.get().is_none());
            assert_eq!(
                export_error.get().map(|err| err.error),
                Some("export failed".to_string())
            );

            apply_optional_export_action_result(
                Some(Ok(serde_json::json!({
                    "filename": "a.csv",
                    "csv_data": "date,hours\n2025-01-01,8"
                }))),
                export_success,
                export_error,
            );
            assert!(export_success.get().is_none());
            assert!(matches!(
                export_error.get(),
                Some(err) if err.error.contains("CSVのダウンロードに失敗しました")
            ));

            let form_state = AttendanceFormState::new();
            let history_query = create_rw_signal(HistoryQuery::new(None, None));
            let holiday_query = create_rw_signal(HolidayQuery::new(2025, 1));
            let range_error = create_rw_signal(Some("old-range-error".to_string()));

            select_current_month(
                &form_state,
                history_query,
                holiday_query,
                range_error,
                export_error,
                export_success,
            );
            assert!(range_error.get().is_none());
            assert!(export_error.get().is_none());
            assert!(export_success.get().is_none());
            assert_ne!(form_state.start_date_signal().get(), "");
            assert_ne!(form_state.end_date_signal().get(), "");

            form_state.start_date_signal().set("2025-02-10".into());
            form_state.end_date_signal().set("2025-02-01".into());
            load_selected_range(
                &form_state,
                history_query,
                holiday_query,
                range_error,
                export_error,
                export_success,
            );
            assert_eq!(
                range_error.get().as_deref(),
                Some("開始日は終了日以前の日付を指定してください。")
            );

            form_state.start_date_signal().set("2025-02-01".into());
            form_state.end_date_signal().set("2025-02-10".into());
            load_selected_range(
                &form_state,
                history_query,
                holiday_query,
                range_error,
                export_error,
                export_success,
            );
            assert!(range_error.get().is_none());
            assert_eq!(
                history_query.get().from,
                NaiveDate::from_ymd_opt(2025, 2, 1)
            );
            assert_eq!(history_query.get().to, NaiveDate::from_ymd_opt(2025, 2, 10));

            let mut exported_payload = None;
            export_csv_from_form(&form_state, export_error, export_success, |payload| {
                exported_payload = Some(payload);
            });
            let exported_payload = exported_payload.expect("payload should be dispatched");
            assert_eq!(exported_payload.from.as_deref(), Some("2025-02-01"));
            assert_eq!(exported_payload.to.as_deref(), Some("2025-02-10"));
            assert!(export_error.get().is_none());
            assert!(export_success.get().is_none());

            form_state.start_date_signal().set("invalid".into());
            form_state.end_date_signal().set("2025-02-10".into());
            export_csv_from_form(&form_state, export_error, export_success, |_| {
                panic!("invalid date should not dispatch export");
            });
            assert!(export_error.get().is_some());

            let payload = resolve_clock_in_payload(false, clock_message, last_clock_event)
                .expect("clock in payload");
            assert_eq!(payload.kind, ClockEventKind::ClockIn);
            assert!(clock_message.get().is_none());
            assert_eq!(last_clock_event.get(), Some(ClockEventKind::ClockIn));
            assert!(resolve_clock_in_payload(true, clock_message, last_clock_event).is_none());

            let payload = resolve_clock_out_payload(false, clock_message, last_clock_event)
                .expect("clock out payload");
            assert_eq!(payload.kind, ClockEventKind::ClockOut);
            assert_eq!(last_clock_event.get(), Some(ClockEventKind::ClockOut));
            assert!(resolve_clock_out_payload(true, clock_message, last_clock_event).is_none());

            let clocked_in = status("clocked_in", Some("att-1"), None);
            let break_start_payload = resolve_break_start_payload(
                false,
                Some(&clocked_in),
                clock_message,
                last_clock_event,
            )
            .expect("break start payload");
            assert_eq!(break_start_payload.kind, ClockEventKind::BreakStart);
            assert_eq!(break_start_payload.attendance_id.as_deref(), Some("att-1"));
            assert_eq!(last_clock_event.get(), Some(ClockEventKind::BreakStart));

            let missing_attendance = status("clocked_in", None, None);
            assert!(resolve_break_start_payload(
                false,
                Some(&missing_attendance),
                clock_message,
                last_clock_event,
            )
            .is_none());
            assert!(matches!(
                clock_message.get(),
                Some(ClockMessage::Error(err)) if err.error == "出勤レコードが見つかりません。"
            ));
            assert!(resolve_break_start_payload(
                true,
                Some(&clocked_in),
                clock_message,
                last_clock_event
            )
            .is_none());

            let on_break = status("on_break", Some("att-1"), Some("br-1"));
            let break_end_payload =
                resolve_break_end_payload(false, Some(&on_break), clock_message, last_clock_event)
                    .expect("break end payload");
            assert_eq!(break_end_payload.kind, ClockEventKind::BreakEnd);
            assert_eq!(break_end_payload.break_id.as_deref(), Some("br-1"));
            assert_eq!(last_clock_event.get(), Some(ClockEventKind::BreakEnd));

            let missing_break = status("on_break", Some("att-1"), None);
            assert!(resolve_break_end_payload(
                false,
                Some(&missing_break),
                clock_message,
                last_clock_event
            )
            .is_none());
            assert!(matches!(
                clock_message.get(),
                Some(ClockMessage::Error(err)) if err.error == "休憩レコードが見つかりません。"
            ));
            assert!(resolve_break_end_payload(
                true,
                Some(&on_break),
                clock_message,
                last_clock_event
            )
            .is_none());
        });
    }

    #[test]
    fn helper_export_result_mapping_covers_success_and_error_paths() {
        let (success_msg, success_err) = map_export_action_result(Ok(serde_json::json!({
            "filename": "attendance.csv",
            "csv_data": "date,hours\n2025-01-01,8"
        })));
        assert!(success_msg.is_none());
        let success_err = success_err.expect("expected host download error");
        assert!(success_err
            .error
            .contains("CSVのダウンロードに失敗しました"));

        let (api_fail_msg, api_fail_err) =
            map_export_action_result(Err(ApiError::unknown("export failed")));
        assert!(api_fail_msg.is_none());
        let api_fail_err = api_fail_err.expect("expected api error");
        assert_eq!(api_fail_err.error, "export failed");
    }
}
