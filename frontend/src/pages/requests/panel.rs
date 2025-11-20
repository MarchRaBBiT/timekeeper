use crate::api::{CreateLeaveRequest, CreateOvertimeRequest};
use crate::pages::requests::{
    components::{
        detail_modal::RequestDetailModal, filter::RequestsFilter, leave_form::LeaveRequestForm,
        list::RequestsList, overtime_form::OvertimeRequestForm,
    },
    layout::RequestsLayout,
    repository::RequestsRepository,
    types::{flatten_requests, RequestSummary},
    utils::{EditTarget, LeaveFormState, MessageState, OvertimeFormState, RequestFilterState},
};
use leptos::*;

#[component]
pub fn RequestsPage() -> impl IntoView {
    let repository = store_value(RequestsRepository::new());
    let leave_state = LeaveFormState::default();
    let overtime_state = OvertimeFormState::default();
    let filter_state = RequestFilterState::default();
    let leave_message = create_rw_signal(MessageState::default());
    let overtime_message = create_rw_signal(MessageState::default());
    let list_message = create_rw_signal(MessageState::default());
    let selected_request = create_rw_signal(None::<RequestSummary>);
    let editing_request = create_rw_signal(None::<EditTarget>);
    let reload = create_rw_signal(0u32);

    let requests_resource = create_resource(
        move || reload.get(),
        move |_| {
            let repo = repository.get_value();
            async move { repo.list_my_requests().await }
        },
    );
    let requests_loading = requests_resource.loading();
    let requests_error =
        Signal::derive(move || requests_resource.get().and_then(|result| result.err()));
    let all_summaries = Signal::derive(move || {
        requests_resource
            .get()
            .and_then(|result| result.ok())
            .map(|data| flatten_requests(&data))
            .unwrap_or_default()
    });
    let filter_state_for_signal = filter_state.clone();
    let filtered_summaries = Signal::derive(move || {
        let status = filter_state_for_signal.status_filter();
        if status.is_empty() {
            return all_summaries.get();
        }
        all_summaries
            .get()
            .into_iter()
            .filter(|summary| summary.status == status)
            .collect::<Vec<_>>()
    });

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

    {
        let reload = reload.clone();
        let editing_request = editing_request.clone();
        let leave_state = leave_state.clone();
        let overtime_state = overtime_state.clone();
        create_effect(move |_| {
            if let Some(result) = update_action.value().get() {
                match result {
                    Ok(_) => {
                        list_message.update(|msg| msg.set_success("申請内容を更新しました。"));
                        editing_request.set(None);
                        leave_state.reset();
                        overtime_state.reset();
                        reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => list_message.update(|msg| msg.set_error(err)),
                }
            }
        });
    }
    {
        let cancel_action = cancel_action.clone();
        let list_message = list_message.clone();
        let reload = reload.clone();
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

    let on_edit = Callback::new({
        let editing_request = editing_request.clone();
        let list_message = list_message.clone();
        let leave_state = leave_state.clone();
        let overtime_state = overtime_state.clone();
        move |summary: RequestSummary| {
            list_message.update(|msg| msg.clear());
            let target = EditTarget {
                id: summary.id.clone(),
                kind: summary.kind,
            };
            editing_request.set(Some(target));
            match summary.kind {
                crate::pages::requests::types::RequestKind::Leave => {
                    leave_state.load_from_value(&summary.details)
                }
                crate::pages::requests::types::RequestKind::Overtime => {
                    overtime_state.load_from_value(&summary.details)
                }
            }
        }
    });
    let on_cancel_request = Callback::new({
        let cancel_action = cancel_action.clone();
        let editing_request = editing_request.clone();
        let leave_state = leave_state.clone();
        let overtime_state = overtime_state.clone();
        move |summary: RequestSummary| {
            editing_request.set(None);
            leave_state.reset();
            overtime_state.reset();
            cancel_action.dispatch(summary.id.clone());
        }
    });
    {
        let reload = reload.clone();
        let leave_message = leave_message.clone();
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
        let reload = reload.clone();
        let overtime_message = overtime_message.clone();
        create_effect(move |_| {
            if let Some(result) = overtime_action.value().get() {
                match result {
                    Ok(_) => {
                        overtime_message.update(|msg| msg.set_success("残業申請を送信しました。"));
                        reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => overtime_message.update(|msg| msg.set_error(err)),
                }
            }
        });
    }

    let on_select = Callback::new({
        let selected_request = selected_request.clone();
        move |summary: RequestSummary| {
            selected_request.set(Some(summary));
        }
    });

    view! {
        <>
            <RequestsLayout>
                <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <LeaveRequestForm
                        state=leave_state.clone()
                        message=leave_message
                        action=leave_action
                        update_action=update_action
                        editing=editing_request
                        on_cancel_edit=Callback::new({
                            let editing_request = editing_request.clone();
                            let list_message = list_message.clone();
                            let leave_state = leave_state.clone();
                            move |_| {
                                editing_request.set(None);
                                list_message.update(|msg| msg.clear());
                                leave_state.reset();
                            }
                        })
                    />
                    <OvertimeRequestForm
                        state=overtime_state.clone()
                        message=overtime_message
                        action=overtime_action
                        update_action=update_action
                        editing=editing_request
                        on_cancel_edit=Callback::new({
                            let editing_request = editing_request.clone();
                            let list_message = list_message.clone();
                            let overtime_state = overtime_state.clone();
                            move |_| {
                                editing_request.set(None);
                                list_message.update(|msg| msg.clear());
                                overtime_state.reset();
                            }
                        })
                    />
                </div>
                <RequestsFilter filter_state=filter_state.clone() />
                <RequestsList
                    summaries=filtered_summaries
                    loading=requests_loading
                    error=requests_error
                    on_select=on_select
                    on_edit=on_edit
                    on_cancel=on_cancel_request
                    message=list_message
                />
            </RequestsLayout>
            <RequestDetailModal selected=selected_request />
        </>
    }
}

#[derive(Clone)]
pub struct EditPayload {
    id: String,
    body: serde_json::Value,
}

impl From<(String, serde_json::Value)> for EditPayload {
    fn from(value: (String, serde_json::Value)) -> Self {
        Self {
            id: value.0,
            body: value.1,
        }
    }
}
