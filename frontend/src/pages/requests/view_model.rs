use crate::api::{ApiClient, CreateLeaveRequest, CreateOvertimeRequest};
use crate::pages::requests::types::MyRequestsResponse;
use crate::pages::requests::{
    repository::RequestsRepository,
    types::{flatten_requests, RequestSummary},
    utils::{EditTarget, LeaveFormState, MessageState, OvertimeFormState, RequestFilterState},
};
use leptos::*;

#[derive(Clone, Copy)]
pub struct RequestsViewModel {
    pub leave_state: LeaveFormState,
    pub overtime_state: OvertimeFormState,
    pub filter_state: RequestFilterState,
    pub leave_message: RwSignal<MessageState>,
    pub overtime_message: RwSignal<MessageState>,
    pub list_message: RwSignal<MessageState>,
    pub selected_request: RwSignal<Option<RequestSummary>>,
    pub editing_request: RwSignal<Option<EditTarget>>,
    pub active_form: ReadSignal<RequestFormKind>,
    pub set_active_form: WriteSignal<RequestFormKind>,
    pub requests_resource: Resource<u32, Result<MyRequestsResponse, String>>,
    pub reload: RwSignal<u32>,
    pub leave_action: Action<CreateLeaveRequest, Result<(), String>>,
    pub overtime_action: Action<CreateOvertimeRequest, Result<(), String>>,
    pub update_action: Action<EditPayload, Result<(), String>>,
    pub cancel_action: Action<String, Result<(), String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestFormKind {
    Leave,
    Overtime,
}

#[derive(Clone)]
pub struct EditPayload {
    pub id: String,
    pub body: serde_json::Value,
}

impl From<(String, serde_json::Value)> for EditPayload {
    fn from(value: (String, serde_json::Value)) -> Self {
        Self {
            id: value.0,
            body: value.1,
        }
    }
}

impl RequestsViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
        let repository = store_value(RequestsRepository::new(api));

        let leave_state = LeaveFormState::default();
        let overtime_state = OvertimeFormState::default();
        let filter_state = RequestFilterState::default();
        let leave_message = create_rw_signal(MessageState::default());
        let overtime_message = create_rw_signal(MessageState::default());
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

        let update_action = create_action(move |payload: &EditPayload| {
            let repo = repository.get_value();
            let payload = payload.clone();
            async move { repo.update_request(&payload.id, payload.body.clone()).await }
        });

        let cancel_action = create_action(move |id: &String| {
            let repo = repository.get_value();
            let id = id.clone();
            async move { repo.cancel_request(&id).await }
        });

        // Setup effects for actions
        {
            create_effect(move |_| {
                if let Some(result) = update_action.value().get() {
                    match result {
                        Ok(_) => {
                            list_message.update(|msg| msg.set_success("申請内容を更新しました。"));
                            editing_request.set(None);
                            leave_state.reset();
                            // Note: reset() for overtime_state is handled by the caller or needed here?
                            // In original code it was both.
                            reload.update(|value| *value = value.wrapping_add(1));
                        }
                        Err(err) => list_message.update(|msg| msg.set_error(err)),
                    }
                }
            });
        }

        {
            create_effect(move |_| {
                if let Some(result) = cancel_action.value().get() {
                    match result {
                        Ok(_) => {
                            list_message.update(|msg| msg.set_success("申請を取消しました。"));
                            reload.update(|value| *value = value.wrapping_add(1));
                        }
                        Err(err) => list_message.update(|msg| msg.set_error(err)),
                    }
                }
            });
        }

        {
            create_effect(move |_| {
                if let Some(result) = leave_action.value().get() {
                    match result {
                        Ok(_) => {
                            leave_message.update(|msg| msg.set_success("休暇申請を送信しました。"));
                            reload.update(|value| *value = value.wrapping_add(1));
                        }
                        Err(err) => leave_message.update(|msg| msg.set_error(err)),
                    }
                }
            });
        }

        {
            create_effect(move |_| {
                if let Some(result) = overtime_action.value().get() {
                    match result {
                        Ok(_) => {
                            overtime_message
                                .update(|msg| msg.set_success("残業申請を送信しました。"));
                            reload.update(|value| *value = value.wrapping_add(1));
                        }
                        Err(err) => overtime_message.update(|msg| msg.set_error(err)),
                    }
                }
            });
        }

        Self {
            leave_state,
            overtime_state,
            filter_state,
            leave_message,
            overtime_message,
            list_message,
            selected_request,
            editing_request,
            active_form,
            set_active_form,
            requests_resource,
            reload,
            leave_action,
            overtime_action,
            update_action,
            cancel_action,
        }
    }

    pub fn filtered_summaries(&self) -> Signal<Vec<RequestSummary>> {
        let requests_resource = self.requests_resource;
        let all_summaries = Signal::derive(move || {
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
            all_summaries
                .get()
                .into_iter()
                .filter(|summary| summary.status == status)
                .collect::<Vec<_>>()
        })
    }

    pub fn on_edit(&self) -> Callback<RequestSummary> {
        let leave_state = self.leave_state;
        let overtime_state = self.overtime_state;
        let list_message = self.list_message;
        let editing_request = self.editing_request;
        let set_active_form = self.set_active_form;

        Callback::new(move |summary: RequestSummary| {
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
            }
        })
    }

    pub fn on_cancel_request(&self) -> Callback<RequestSummary> {
        let leave_state = self.leave_state;
        let overtime_state = self.overtime_state;
        let editing_request = self.editing_request;
        let cancel_action = self.cancel_action;

        Callback::new(move |summary: RequestSummary| {
            editing_request.set(None);
            leave_state.reset();
            overtime_state.reset();
            cancel_action.dispatch(summary.id.clone());
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
