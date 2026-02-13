use crate::api::{
    ApiClient, ApiError, CreateAttendanceCorrectionRequest, CreateLeaveRequest,
    CreateOvertimeRequest, UpdateAttendanceCorrectionRequest,
};
use crate::pages::requests::types::MyRequestsResponse;
use crate::pages::requests::{
    repository::RequestsRepository,
    types::{flatten_requests, RequestSummary},
    utils::{
        AttendanceCorrectionFormState, EditTarget, LeaveFormState, MessageState, OvertimeFormState,
        RequestFilterState,
    },
};
use leptos::*;

#[derive(Clone, Copy)]
pub struct RequestsViewModel {
    pub leave_state: LeaveFormState,
    pub overtime_state: OvertimeFormState,
    pub correction_state: AttendanceCorrectionFormState,
    pub filter_state: RequestFilterState,
    pub leave_message: RwSignal<MessageState>,
    pub overtime_message: RwSignal<MessageState>,
    pub correction_message: RwSignal<MessageState>,
    pub list_message: RwSignal<MessageState>,
    pub selected_request: RwSignal<Option<RequestSummary>>,
    pub editing_request: RwSignal<Option<EditTarget>>,
    pub active_form: ReadSignal<RequestFormKind>,
    pub set_active_form: WriteSignal<RequestFormKind>,
    pub requests_resource: Resource<u32, Result<MyRequestsResponse, ApiError>>,
    pub leave_action: Action<CreateLeaveRequest, Result<(), ApiError>>,
    pub overtime_action: Action<CreateOvertimeRequest, Result<(), ApiError>>,
    pub correction_action: Action<CreateAttendanceCorrectionRequest, Result<(), ApiError>>,
    pub update_action: Action<EditPayload, Result<(), ApiError>>,
    pub correction_update_action: Action<AttendanceCorrectionEditPayload, Result<(), ApiError>>,
    pub cancel_action: Action<String, Result<(), ApiError>>,
    pub correction_cancel_action: Action<String, Result<(), ApiError>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestFormKind {
    Leave,
    Overtime,
    AttendanceCorrection,
}

#[derive(Clone)]
pub struct EditPayload {
    pub id: String,
    pub body: serde_json::Value,
}

#[derive(Clone)]
pub struct AttendanceCorrectionEditPayload {
    pub id: String,
    pub payload: UpdateAttendanceCorrectionRequest,
}

impl From<(String, UpdateAttendanceCorrectionRequest)> for AttendanceCorrectionEditPayload {
    fn from(value: (String, UpdateAttendanceCorrectionRequest)) -> Self {
        Self {
            id: value.0,
            payload: value.1,
        }
    }
}

impl From<(String, serde_json::Value)> for EditPayload {
    fn from(value: (String, serde_json::Value)) -> Self {
        Self {
            id: value.0,
            body: value.1,
        }
    }
}

fn apply_optional_update_action_result(
    result: Option<Result<(), ApiError>>,
    list_message: RwSignal<MessageState>,
    editing_request: RwSignal<Option<EditTarget>>,
    leave_state: LeaveFormState,
    overtime_state: OvertimeFormState,
    correction_state: AttendanceCorrectionFormState,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        match result {
            Ok(_) => {
                list_message.update(|msg| msg.set_success("申請内容を更新しました。"));
                editing_request.set(None);
                leave_state.reset();
                overtime_state.reset();
                correction_state.reset();
                reload.update(|value| *value = value.wrapping_add(1));
            }
            Err(err) => list_message.update(|msg| msg.set_error(err)),
        }
    }
}

fn apply_optional_cancel_action_result(
    result: Option<Result<(), ApiError>>,
    list_message: RwSignal<MessageState>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        match result {
            Ok(_) => {
                list_message.update(|msg| msg.set_success("申請を取消しました。"));
                reload.update(|value| *value = value.wrapping_add(1));
            }
            Err(err) => list_message.update(|msg| msg.set_error(err)),
        }
    }
}

fn apply_optional_leave_action_result(
    result: Option<Result<(), ApiError>>,
    leave_message: RwSignal<MessageState>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        match result {
            Ok(_) => {
                leave_message.update(|msg| msg.set_success("休暇申請を送信しました。"));
                reload.update(|value| *value = value.wrapping_add(1));
            }
            Err(err) => leave_message.update(|msg| msg.set_error(err)),
        }
    }
}

fn apply_optional_overtime_action_result(
    result: Option<Result<(), ApiError>>,
    overtime_message: RwSignal<MessageState>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        match result {
            Ok(_) => {
                overtime_message.update(|msg| msg.set_success("残業申請を送信しました。"));
                reload.update(|value| *value = value.wrapping_add(1));
            }
            Err(err) => overtime_message.update(|msg| msg.set_error(err)),
        }
    }
}

fn apply_optional_correction_action_result(
    result: Option<Result<(), ApiError>>,
    correction_message: RwSignal<MessageState>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        match result {
            Ok(_) => {
                correction_message.update(|msg| msg.set_success("勤怠修正依頼を送信しました。"));
                reload.update(|value| *value = value.wrapping_add(1));
            }
            Err(err) => correction_message.update(|msg| msg.set_error(err)),
        }
    }
}

fn apply_edit_selection(
    summary: RequestSummary,
    leave_state: LeaveFormState,
    overtime_state: OvertimeFormState,
    correction_state: AttendanceCorrectionFormState,
    list_message: RwSignal<MessageState>,
    editing_request: RwSignal<Option<EditTarget>>,
    set_active_form: WriteSignal<RequestFormKind>,
) {
    list_message.update(|msg| msg.clear());
    let target = EditTarget {
        id: summary.id.clone(),
        kind: summary.kind,
    };
    editing_request.set(Some(target));
    match summary.kind {
        crate::pages::requests::types::RequestKind::Leave => {
            leave_state.load_from_value(&summary.details);
            set_active_form.set(RequestFormKind::Leave);
        }
        crate::pages::requests::types::RequestKind::Overtime => {
            overtime_state.load_from_value(&summary.details);
            set_active_form.set(RequestFormKind::Overtime);
        }
        crate::pages::requests::types::RequestKind::AttendanceCorrection => {
            correction_state.load_from_value(&summary.details);
            set_active_form.set(RequestFormKind::AttendanceCorrection);
        }
    }
}

fn apply_cancel_selection<F>(
    summary: RequestSummary,
    leave_state: LeaveFormState,
    overtime_state: OvertimeFormState,
    correction_state: AttendanceCorrectionFormState,
    editing_request: RwSignal<Option<EditTarget>>,
    dispatch_cancel: F,
) where
    F: FnOnce(String),
{
    editing_request.set(None);
    leave_state.reset();
    overtime_state.reset();
    correction_state.reset();
    dispatch_cancel(summary.id);
}

impl RequestsViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
        let repository = store_value(RequestsRepository::new(api));

        let leave_state = LeaveFormState::default();
        let overtime_state = OvertimeFormState::default();
        let correction_state = AttendanceCorrectionFormState::default();
        let filter_state = RequestFilterState::default();
        let leave_message = create_rw_signal(MessageState::default());
        let overtime_message = create_rw_signal(MessageState::default());
        let correction_message = create_rw_signal(MessageState::default());
        let list_message = create_rw_signal(MessageState::default());
        let selected_request = create_rw_signal(None::<RequestSummary>);
        let editing_request = create_rw_signal(None::<EditTarget>);
        let reload = create_rw_signal(0u32);
        let (active_form, set_active_form) = create_signal(RequestFormKind::Leave);

        let requests_resource = create_resource(
            move || reload.get(),
            move |_| {
                let repo = repository.get_value();
                async move { repo.list_my_requests().await }
            },
        );

        let leave_action = create_action(move |payload: &CreateLeaveRequest| {
            let repo = repository.get_value();
            let payload = payload.clone();
            async move { repo.submit_leave(payload).await.map(|_| ()) }
        });

        let overtime_action = create_action(move |payload: &CreateOvertimeRequest| {
            let repo = repository.get_value();
            let payload = payload.clone();
            async move { repo.submit_overtime(payload).await.map(|_| ()) }
        });

        let correction_action =
            create_action(move |payload: &CreateAttendanceCorrectionRequest| {
                let repo = repository.get_value();
                let payload = payload.clone();
                async move { repo.submit_attendance_correction(payload).await }
            });

        let update_action = create_action(move |payload: &EditPayload| {
            let repo = repository.get_value();
            let payload = payload.clone();
            async move { repo.update_request(&payload.id, payload.body.clone()).await }
        });

        let correction_update_action =
            create_action(move |payload: &AttendanceCorrectionEditPayload| {
                let repo = repository.get_value();
                let payload = payload.clone();
                async move {
                    repo.update_attendance_correction(&payload.id, payload.payload)
                        .await
                }
            });

        let cancel_action = create_action(move |id: &String| {
            let repo = repository.get_value();
            let id = id.clone();
            async move { repo.cancel_request(&id).await }
        });

        let correction_cancel_action = create_action(move |id: &String| {
            let repo = repository.get_value();
            let id = id.clone();
            async move { repo.cancel_attendance_correction(&id).await }
        });

        // Setup effects for actions
        {
            create_effect(move |_| {
                apply_optional_update_action_result(
                    update_action.value().get(),
                    list_message,
                    editing_request,
                    leave_state,
                    overtime_state,
                    correction_state,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_cancel_action_result(
                    cancel_action.value().get(),
                    list_message,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_cancel_action_result(
                    correction_cancel_action.value().get(),
                    list_message,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_leave_action_result(
                    leave_action.value().get(),
                    leave_message,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_overtime_action_result(
                    overtime_action.value().get(),
                    overtime_message,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_correction_action_result(
                    correction_action.value().get(),
                    correction_message,
                    reload,
                );
            });
        }

        {
            create_effect(move |_| {
                apply_optional_update_action_result(
                    correction_update_action.value().get(),
                    list_message,
                    editing_request,
                    leave_state,
                    overtime_state,
                    correction_state,
                    reload,
                );
            });
        }

        Self {
            leave_state,
            overtime_state,
            correction_state,
            filter_state,
            leave_message,
            overtime_message,
            correction_message,
            list_message,
            selected_request,
            editing_request,
            active_form,
            set_active_form,
            requests_resource,
            leave_action,
            overtime_action,
            correction_action,
            update_action,
            correction_update_action,
            cancel_action,
            correction_cancel_action,
        }
    }

    pub fn filtered_summaries(&self) -> Signal<Vec<RequestSummary>> {
        let requests_resource = self.requests_resource;
        let all_summaries = create_memo(move |_| {
            requests_resource
                .get()
                .and_then(|result| result.ok())
                .map(|data| flatten_requests(&data))
                .unwrap_or_default()
        });
        let filter_state = self.filter_state;
        Signal::derive(move || {
            let status = filter_state.status_filter();
            if status.is_empty() {
                return all_summaries.get();
            }
            all_summaries.with(|summaries| {
                summaries
                    .iter()
                    .filter(|summary| summary.status == status)
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    }

    pub fn on_edit(&self) -> Callback<RequestSummary> {
        let leave_state = self.leave_state;
        let overtime_state = self.overtime_state;
        let correction_state = self.correction_state;
        let list_message = self.list_message;
        let editing_request = self.editing_request;
        let set_active_form = self.set_active_form;

        Callback::new(move |summary: RequestSummary| {
            apply_edit_selection(
                summary,
                leave_state,
                overtime_state,
                correction_state,
                list_message,
                editing_request,
                set_active_form,
            );
        })
    }

    pub fn on_cancel_request(&self) -> Callback<RequestSummary> {
        let leave_state = self.leave_state;
        let overtime_state = self.overtime_state;
        let correction_state = self.correction_state;
        let editing_request = self.editing_request;
        let cancel_action = self.cancel_action;
        let correction_cancel_action = self.correction_cancel_action;

        Callback::new(move |summary: RequestSummary| {
            let summary_kind = summary.kind;
            apply_cancel_selection(
                summary,
                leave_state,
                overtime_state,
                correction_state,
                editing_request,
                |id| match summary_kind {
                    crate::pages::requests::types::RequestKind::AttendanceCorrection => {
                        correction_cancel_action.dispatch(id)
                    }
                    _ => cancel_action.dispatch(id),
                },
            );
        })
    }
}

pub fn use_requests_view_model() -> RequestsViewModel {
    match use_context::<RequestsViewModel>() {
        Some(vm) => vm,
        None => {
            let vm = RequestsViewModel::new();
            provide_context(vm.clone());
            vm
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::ssr::{with_local_runtime, with_local_runtime_async, with_runtime};
    use chrono::NaiveDate;
    use serde_json::json;

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/requests/me");
            then.status(200).json_body(json!({
                "leave_requests": [],
                "overtime_requests": [],
                "attendance_corrections": []
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/requests/leave");
            then.status(200).json_body(json!({
                "id": "leave-1",
                "user_id": "u1",
                "leave_type": "annual",
                "start_date": "2025-01-10",
                "end_date": "2025-01-12",
                "reason": null,
                "status": "pending",
                "approved_by": null,
                "approved_at": null,
                "rejected_by": null,
                "rejected_at": null,
                "cancelled_at": null,
                "decision_comment": null,
                "created_at": "2025-01-01T00:00:00Z"
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/requests/overtime");
            then.status(200).json_body(json!({
                "id": "ot-1",
                "user_id": "u1",
                "date": "2025-01-11",
                "planned_hours": 2.5,
                "reason": null,
                "status": "pending",
                "approved_by": null,
                "approved_at": null,
                "rejected_by": null,
                "rejected_at": null,
                "cancelled_at": null,
                "decision_comment": null,
                "created_at": "2025-01-01T00:00:00Z"
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/attendance-corrections");
            then.status(200).json_body(json!({
                "id": "corr-1",
                "status": "pending"
            }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/requests/req-1");
            then.status(200).json_body(json!({ "status": "updated" }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/attendance-corrections/corr-1");
            then.status(200).json_body(json!({ "status": "updated" }));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/requests/req-1");
            then.status(200).json_body(json!({ "status": "cancelled" }));
        });
        server
    }

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
    fn requests_view_model_filters_summaries() {
        with_runtime(|| {
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            leptos_reactive::suppress_resource_load(true);
            let vm = RequestsViewModel::new();
            let response = MyRequestsResponse {
                leave_requests: vec![json!({
                    "id": "leave-1",
                    "status": "approved",
                    "start_date": "2025-01-10",
                    "end_date": "2025-01-10",
                    "leave_type": "annual",
                    "created_at": "2025-01-05T10:00:00Z"
                })],
                overtime_requests: vec![json!({
                    "id": "ot-1",
                    "status": "pending",
                    "date": "2025-01-11",
                    "planned_hours": 2.5,
                    "created_at": "2025-01-06T10:00:00Z"
                })],
                attendance_corrections: vec![json!({
                    "id": "corr-1",
                    "status": "pending",
                    "date": "2025-01-12",
                    "reason": "fix",
                    "created_at": "2025-01-07T10:00:00Z",
                    "proposed_values": {
                        "breaks": []
                    }
                })],
            };
            vm.requests_resource.set(Ok(response));
            let all = vm.filtered_summaries().get();
            assert_eq!(all.len(), 3);

            vm.filter_state.status_signal().set("approved".into());
            let filtered = vm.filtered_summaries().get();
            assert_eq!(filtered.len(), 1);
            leptos_reactive::suppress_resource_load(false);
        });
    }

    #[test]
    fn requests_view_model_actions_update_messages() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = RequestsViewModel::new();

            vm.leave_action.dispatch(CreateLeaveRequest {
                leave_type: "annual".into(),
                start_date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
                reason: None,
            });
            assert!(
                wait_until(|| vm.leave_action.value().get().is_some()).await,
                "leave action should complete"
            );
            assert!(matches!(vm.leave_action.value().get(), Some(Ok(()))));
            let _ = vm.leave_message.get();

            vm.overtime_action.dispatch(CreateOvertimeRequest {
                date: NaiveDate::from_ymd_opt(2025, 1, 11).unwrap(),
                planned_hours: 2.5,
                reason: None,
            });
            assert!(
                wait_until(|| vm.overtime_action.value().get().is_some()).await,
                "overtime action should complete"
            );
            assert!(matches!(vm.overtime_action.value().get(), Some(Ok(()))));
            let _ = vm.overtime_message.get();

            vm.correction_action
                .dispatch(CreateAttendanceCorrectionRequest {
                    date: NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
                    clock_in_time: None,
                    clock_out_time: None,
                    breaks: Some(vec![]),
                    reason: "fix".to_string(),
                });
            assert!(
                wait_until(|| vm.correction_action.value().get().is_some()).await,
                "correction action should complete"
            );
            assert!(matches!(vm.correction_action.value().get(), Some(Ok(()))));
            let _ = vm.correction_message.get();

            vm.editing_request.set(Some(EditTarget {
                id: "req-1".into(),
                kind: crate::pages::requests::types::RequestKind::Leave,
            }));
            vm.update_action.dispatch(EditPayload::from((
                "req-1".to_string(),
                json!({ "status": "updated" }),
            )));
            assert!(
                wait_until(|| vm.update_action.value().get().is_some()).await,
                "update action should complete"
            );
            assert!(matches!(vm.update_action.value().get(), Some(Ok(()))));
            let _ = vm.editing_request.get();
            let _ = vm.list_message.get();

            vm.editing_request.set(Some(EditTarget {
                id: "corr-1".into(),
                kind: crate::pages::requests::types::RequestKind::AttendanceCorrection,
            }));
            vm.correction_update_action.dispatch(
                (
                    "corr-1".to_string(),
                    UpdateAttendanceCorrectionRequest {
                        clock_in_time: None,
                        clock_out_time: None,
                        breaks: Some(vec![]),
                        reason: "updated".to_string(),
                    },
                )
                    .into(),
            );
            assert!(
                wait_until(|| vm.correction_update_action.value().get().is_some()).await,
                "correction update action should complete"
            );
            assert!(matches!(
                vm.correction_update_action.value().get(),
                Some(Ok(()))
            ));

            vm.cancel_action.dispatch("req-1".into());
            assert!(
                wait_until(|| vm.cancel_action.value().get().is_some()).await,
                "cancel action should complete"
            );
            assert!(matches!(vm.cancel_action.value().get(), Some(Ok(()))));
            let _ = vm.list_message.get();
            runtime.dispose();
        });
    }

    #[test]
    fn helper_effect_and_selection_paths_cover_branches() {
        with_runtime(|| {
            let leave_state = LeaveFormState::default();
            let overtime_state = OvertimeFormState::default();
            let correction_state = AttendanceCorrectionFormState::default();
            let list_message = create_rw_signal(MessageState::default());
            let leave_message = create_rw_signal(MessageState::default());
            let overtime_message = create_rw_signal(MessageState::default());
            let correction_message = create_rw_signal(MessageState::default());
            let editing_request = create_rw_signal(Some(EditTarget {
                id: "req-old".to_string(),
                kind: crate::pages::requests::types::RequestKind::Leave,
            }));
            let reload = create_rw_signal(0u32);
            let (active_form, set_active_form) = create_signal(RequestFormKind::Leave);

            leave_state.start_signal().set("2025-01-10".to_string());
            apply_optional_update_action_result(
                Some(Ok(())),
                list_message,
                editing_request,
                leave_state,
                overtime_state,
                correction_state,
                reload,
            );
            assert_eq!(
                list_message.get().success.as_deref(),
                Some("申請内容を更新しました。")
            );
            assert!(editing_request.get().is_none());
            assert_eq!(leave_state.start_signal().get(), "");
            assert_eq!(reload.get(), 1);

            apply_optional_update_action_result(
                Some(Err(ApiError::unknown("update failed"))),
                list_message,
                editing_request,
                leave_state,
                overtime_state,
                correction_state,
                reload,
            );
            assert_eq!(
                list_message.get().error.map(|err| err.error),
                Some("update failed".to_string())
            );
            assert_eq!(reload.get(), 1);

            apply_optional_cancel_action_result(Some(Ok(())), list_message, reload);
            assert_eq!(
                list_message.get().success.as_deref(),
                Some("申請を取消しました。")
            );
            assert_eq!(reload.get(), 2);
            apply_optional_cancel_action_result(
                Some(Err(ApiError::unknown("cancel failed"))),
                list_message,
                reload,
            );
            assert_eq!(
                list_message.get().error.map(|err| err.error),
                Some("cancel failed".to_string())
            );

            apply_optional_leave_action_result(Some(Ok(())), leave_message, reload);
            assert_eq!(
                leave_message.get().success.as_deref(),
                Some("休暇申請を送信しました。")
            );
            assert_eq!(reload.get(), 3);
            apply_optional_leave_action_result(
                Some(Err(ApiError::unknown("leave failed"))),
                leave_message,
                reload,
            );
            assert_eq!(
                leave_message.get().error.map(|err| err.error),
                Some("leave failed".to_string())
            );

            apply_optional_overtime_action_result(Some(Ok(())), overtime_message, reload);
            assert_eq!(
                overtime_message.get().success.as_deref(),
                Some("残業申請を送信しました。")
            );
            assert_eq!(reload.get(), 4);
            apply_optional_overtime_action_result(
                Some(Err(ApiError::unknown("overtime failed"))),
                overtime_message,
                reload,
            );
            assert_eq!(
                overtime_message.get().error.map(|err| err.error),
                Some("overtime failed".to_string())
            );

            apply_optional_correction_action_result(Some(Ok(())), correction_message, reload);
            assert_eq!(
                correction_message.get().success.as_deref(),
                Some("勤怠修正依頼を送信しました。")
            );
            assert_eq!(reload.get(), 5);
            apply_optional_correction_action_result(
                Some(Err(ApiError::unknown("correction failed"))),
                correction_message,
                reload,
            );
            assert_eq!(
                correction_message.get().error.map(|err| err.error),
                Some("correction failed".to_string())
            );

            list_message.set(MessageState {
                success: Some("old".to_string()),
                error: None,
            });
            let leave_summary = RequestSummary {
                id: "leave-1".to_string(),
                kind: crate::pages::requests::types::RequestKind::Leave,
                status: "pending".to_string(),
                submitted_at: None,
                primary_label: None,
                secondary_label: None,
                reason: None,
                details: json!({
                    "leave_type": "sick",
                    "start_date": "2025-03-01",
                    "end_date": "2025-03-02",
                    "reason": "private"
                }),
            };
            apply_edit_selection(
                leave_summary,
                leave_state,
                overtime_state,
                correction_state,
                list_message,
                editing_request,
                set_active_form,
            );
            assert!(list_message.get().success.is_none());
            assert_eq!(active_form.get(), RequestFormKind::Leave);
            assert_eq!(leave_state.leave_type_signal().get(), "sick");
            assert_eq!(leave_state.start_signal().get(), "2025-03-01");
            assert!(matches!(
                editing_request.get(),
                Some(EditTarget { id, kind: crate::pages::requests::types::RequestKind::Leave }) if id == "leave-1"
            ));

            let overtime_summary = RequestSummary {
                id: "ot-1".to_string(),
                kind: crate::pages::requests::types::RequestKind::Overtime,
                status: "pending".to_string(),
                submitted_at: None,
                primary_label: None,
                secondary_label: None,
                reason: None,
                details: json!({
                    "date": "2025-04-01",
                    "planned_hours": 3.5,
                    "reason": "deploy"
                }),
            };
            apply_edit_selection(
                overtime_summary.clone(),
                leave_state,
                overtime_state,
                correction_state,
                list_message,
                editing_request,
                set_active_form,
            );
            assert_eq!(active_form.get(), RequestFormKind::Overtime);
            assert_eq!(overtime_state.date_signal().get(), "2025-04-01");
            assert_eq!(overtime_state.hours_signal().get(), "3.50");

            let correction_summary = RequestSummary {
                id: "corr-2".to_string(),
                kind: crate::pages::requests::types::RequestKind::AttendanceCorrection,
                status: "pending".to_string(),
                submitted_at: None,
                primary_label: None,
                secondary_label: None,
                reason: None,
                details: json!({
                    "date": "2025-06-01",
                    "reason": "forgot",
                    "proposed_values": {
                        "clock_in_time": "2025-06-01T09:00:00",
                        "clock_out_time": "2025-06-01T18:00:00",
                        "breaks": []
                    }
                }),
            };
            apply_edit_selection(
                correction_summary.clone(),
                leave_state,
                overtime_state,
                correction_state,
                list_message,
                editing_request,
                set_active_form,
            );
            assert_eq!(active_form.get(), RequestFormKind::AttendanceCorrection);
            assert_eq!(correction_state.date_signal().get(), "2025-06-01");
            assert_eq!(correction_state.reason_signal().get(), "forgot");

            leave_state.start_signal().set("2025-05-01".to_string());
            overtime_state.date_signal().set("2025-05-02".to_string());
            correction_state.date_signal().set("2025-05-03".to_string());
            let mut cancelled_id = None;
            apply_cancel_selection(
                correction_summary,
                leave_state,
                overtime_state,
                correction_state,
                editing_request,
                |id| cancelled_id = Some(id),
            );
            assert!(editing_request.get().is_none());
            assert_eq!(leave_state.start_signal().get(), "");
            assert_eq!(overtime_state.date_signal().get(), "");
            assert_eq!(correction_state.date_signal().get(), "");
            assert_eq!(cancelled_id.as_deref(), Some("corr-2"));
        });
    }

    #[test]
    fn use_requests_view_model_reuses_context() {
        with_local_runtime(|| {
            with_runtime(|| {
                let server = mock_server();
                provide_context(ApiClient::new_with_base_url(&server.url("/api")));
                let vm = RequestsViewModel::new();
                vm.filter_state.status_signal().set("approved".to_string());
                provide_context(vm);

                let used = use_requests_view_model();
                assert_eq!(used.filter_state.status_filter(), "approved");
            });
        });
    }
}
