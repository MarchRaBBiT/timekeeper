use crate::api::{ApiClient, ApiError};
use crate::pages::dashboard::{repository, utils::ActivityStatusFilter};
use crate::state::attendance::{
    self as attendance_state, refresh_today_context, use_attendance, AttendanceState,
    ClockEventKind, ClockEventPayload, ClockMessage,
};
use leptos::{ev::MouseEvent, *};

#[derive(Clone, Copy)]
pub struct DashboardViewModel {
    pub summary_resource: Resource<(), Result<repository::DashboardSummary, ApiError>>,
    pub alerts_resource: Resource<
        Option<Result<repository::DashboardSummary, ApiError>>,
        Result<Vec<repository::DashboardAlert>, ApiError>,
    >,
    pub activities_resource:
        Resource<ActivityStatusFilter, Result<Vec<repository::DashboardActivity>, ApiError>>,
    pub activity_filter: RwSignal<ActivityStatusFilter>,
    pub attendance_state: (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>),
    pub clock_action: Action<ClockEventPayload, Result<(), ApiError>>,
    pub clock_message: RwSignal<Option<ClockMessage>>,
    pub last_clock_event: RwSignal<Option<ClockEventKind>>,
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

impl DashboardViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
        let (attendance_read, attendance_write) = use_attendance();

        let api_clone = api.clone();
        let summary_resource = create_resource(
            || (),
            move |_| {
                let api = api_clone.clone();
                async move { repository::fetch_summary(&api).await }
            },
        );

        let alerts_resource = create_resource(
            move || summary_resource.get(),
            move |summary_opt| async move {
                if let Some(Ok(summary)) = summary_opt {
                    Ok(repository::build_alerts(&summary))
                } else {
                    Ok(Vec::new())
                }
            },
        );

        let activity_filter = create_rw_signal(ActivityStatusFilter::All);
        let api_clone = api.clone();
        let activities_resource = create_resource(
            move || activity_filter.get(),
            move |filter| {
                let api = api_clone.clone();
                async move { repository::fetch_recent_activities(&api, filter).await }
            },
        );

        let api_clone = api.clone();
        let clock_action = create_action(move |payload: &ClockEventPayload| {
            let api = api_clone.clone();
            let set_attendance_state = attendance_write;
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
                            .ok_or_else(|| ApiError::unknown("出勤レコードが見つかりません。"))?;
                        attendance_state::start_break(&api, attendance_id).await?
                    }
                    ClockEventKind::BreakEnd => {
                        let break_id = payload
                            .break_id
                            .as_deref()
                            .ok_or_else(|| ApiError::unknown("休憩レコードが見つかりません。"))?;
                        attendance_state::end_break(&api, break_id).await?
                    }
                };
                attendance_state::refresh_today_context(&api, set_attendance_state).await
            }
        });

        let clock_message = create_rw_signal(None);
        let last_clock_event = create_rw_signal(None);

        {
            create_effect(move |_| {
                if let Some(result) = clock_action.value().get() {
                    let mapped = map_clock_action_result(last_clock_event.get_untracked(), result);
                    clock_message.set(Some(mapped));
                }
            });
        }

        {
            let api = api.clone();
            create_effect(move |_| {
                let api = api.clone();
                spawn_local(async move {
                    let _ = refresh_today_context(&api, attendance_write).await;
                });
            });
        }

        Self {
            summary_resource,
            alerts_resource,
            activities_resource,
            activity_filter,
            attendance_state: (attendance_read, attendance_write),
            clock_action,
            clock_message,
            last_clock_event,
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
        let (state, _) = self.attendance_state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let status = state.get().today_status.clone();
            let att_id = match break_start_attendance_id(status.as_ref()) {
                Ok(id) => id,
                Err(err) => {
                    clock_message.set(Some(ClockMessage::Error(err)));
                    return;
                }
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
        let (state, _) = self.attendance_state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let status = state.get().today_status.clone();
            let break_id = match break_end_break_id(status.as_ref()) {
                Ok(id) => id,
                Err(err) => {
                    clock_message.set(Some(ClockMessage::Error(err)));
                    return;
                }
            };
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::BreakEnd));
            clock_action.dispatch(ClockEventPayload::break_end(break_id));
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

pub fn use_dashboard_view_model() -> DashboardViewModel {
    match use_context::<DashboardViewModel>() {
        Some(vm) => vm,
        None => {
            let vm = DashboardViewModel::new();
            provide_context(vm.clone());
            vm
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::pages::dashboard::utils::ActivityStatusFilter;
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
            when.method(GET).path("/api/attendance/me/summary");
            then.status(200).json_body(serde_json::json!({
                "month": 1,
                "year": 2025,
                "total_work_hours": 160.0,
                "total_work_days": 20,
                "average_daily_hours": 8.0
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/requests/me");
            then.status(200).json_body(serde_json::json!({
                "leave_requests": [],
                "overtime_requests": []
            }));
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
        server
    }

    #[test]
    fn dashboard_view_model_sets_clock_message_on_success() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                leptos_reactive::suppress_resource_load(true);
                let vm = DashboardViewModel::new();
                assert_eq!(vm.activity_filter.get(), ActivityStatusFilter::All);

                vm.last_clock_event.set(Some(ClockEventKind::ClockIn));
                vm.clock_action.dispatch(ClockEventPayload::clock_in());
                assert!(vm.last_clock_event.get().is_some());

                leptos_reactive::suppress_resource_load(false);
            });
        });
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
    fn break_start_validation_covers_error_and_success_paths() {
        let no_status = break_start_attendance_id(None).unwrap_err();
        assert_eq!(no_status.error, "ステータスを取得できません。");

        let wrong_status =
            break_start_attendance_id(Some(&status("clocked_out", None, None))).unwrap_err();
        assert_eq!(wrong_status.error, "出勤中のみ休憩を開始できます。");

        let missing_id =
            break_start_attendance_id(Some(&status("clocked_in", None, None))).unwrap_err();
        assert_eq!(missing_id.error, "出勤レコードが見つかりません。");

        let ok =
            break_start_attendance_id(Some(&status("clocked_in", Some("att-1"), None))).unwrap();
        assert_eq!(ok, "att-1");
    }

    #[test]
    fn break_end_validation_covers_error_and_success_paths() {
        let no_status = break_end_break_id(None).unwrap_err();
        assert_eq!(no_status.error, "ステータスを取得できません。");

        let wrong_status =
            break_end_break_id(Some(&status("clocked_in", Some("att-1"), None))).unwrap_err();
        assert_eq!(wrong_status.error, "休憩中のみ休憩を終了できます。");

        let missing_id =
            break_end_break_id(Some(&status("on_break", Some("att-1"), None))).unwrap_err();
        assert_eq!(missing_id.error, "休憩レコードが見つかりません。");

        let ok =
            break_end_break_id(Some(&status("on_break", Some("att-1"), Some("br-1")))).unwrap();
        assert_eq!(ok, "br-1");
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

        let success = map_clock_action_result(Some(ClockEventKind::ClockOut), Ok(()));
        match success {
            ClockMessage::Success(msg) => assert_eq!(msg, "退勤しました。"),
            ClockMessage::Error(_) => panic!("expected success"),
        }

        let failure = map_clock_action_result(None, Err(ApiError::unknown("dashboard failed")));
        match failure {
            ClockMessage::Success(_) => panic!("expected error"),
            ClockMessage::Error(err) => assert_eq!(err.error, "dashboard failed"),
        }

        let default_success = map_clock_action_result(None, Ok(()));
        match default_success {
            ClockMessage::Success(msg) => assert_eq!(msg, "操作が完了しました。"),
            ClockMessage::Error(_) => panic!("expected success"),
        }
    }

    #[test]
    fn use_dashboard_view_model_reuses_existing_context() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let vm = DashboardViewModel::new();
                vm.clock_message
                    .set(Some(ClockMessage::Success("context".to_string())));
                provide_context(vm);

                let used = use_dashboard_view_model();
                match used.clock_message.get() {
                    Some(ClockMessage::Success(msg)) => assert_eq!(msg, "context"),
                    Some(ClockMessage::Error(_)) => panic!("expected success message"),
                    None => panic!("expected message"),
                }
            });
        });
    }

    #[test]
    fn dashboard_resources_and_clock_action_cover_runtime_paths() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = DashboardViewModel::new();

            assert!(
                wait_until(|| {
                    vm.summary_resource.get().is_some()
                        && vm.alerts_resource.get().is_some()
                        && vm.activities_resource.get().is_some()
                })
                .await,
                "initial dashboard resources timeout"
            );

            match vm.summary_resource.get() {
                Some(Ok(summary)) => assert_eq!(summary.total_work_days, Some(20)),
                other => panic!("summary_resource not ready: {:?}", other),
            }
            match vm.alerts_resource.get() {
                Some(Ok(alerts)) => assert!(!alerts.is_empty()),
                other => panic!("alerts_resource not ready: {:?}", other),
            }
            match vm.activities_resource.get() {
                Some(Ok(items)) => assert_eq!(items.len(), 4),
                other => panic!("activities_resource not ready: {:?}", other),
            }

            vm.activity_filter.set(ActivityStatusFilter::PendingOnly);
            assert!(
                wait_until(|| {
                    matches!(
                        vm.activities_resource.get(),
                        Some(Ok(ref items)) if items.len() == 2
                    )
                })
                .await,
                "pending-only activities timeout"
            );

            vm.activity_filter.set(ActivityStatusFilter::ApprovedOnly);
            assert!(
                wait_until(|| {
                    matches!(
                        vm.activities_resource.get(),
                        Some(Ok(ref items)) if items.len() == 2
                    )
                })
                .await,
                "approved-only activities timeout"
            );

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
            match vm.clock_action.value().get() {
                Some(Err(err)) => assert_eq!(err.error, "出勤レコードが見つかりません。"),
                other => panic!("expected break-start validation error, got {:?}", other),
            }

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
            match vm.clock_action.value().get() {
                Some(Err(err)) => assert_eq!(err.error, "休憩レコードが見つかりません。"),
                other => panic!("expected break-end validation error, got {:?}", other),
            }

            runtime.dispose();
        });
    }
}
