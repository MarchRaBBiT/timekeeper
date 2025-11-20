use crate::api::{CreateLeaveRequest, CreateOvertimeRequest};
use crate::pages::requests::{
    components::{
        detail_modal::RequestDetailModal, filter::RequestsFilter, leave_form::LeaveRequestForm,
        list::RequestsList, overtime_form::OvertimeRequestForm,
    },
    layout::RequestsLayout,
    repository::RequestsRepository,
    types::{flatten_requests, RequestSummary},
    utils::{LeaveFormState, MessageState, OvertimeFormState, RequestFilterState},
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
    let selected_request = create_rw_signal(None::<RequestSummary>);
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
                        state=leave_state
                        message=leave_message
                        action=leave_action
                    />
                    <OvertimeRequestForm
                        state=overtime_state
                        message=overtime_message
                        action=overtime_action
                    />
                </div>
                <RequestsFilter filter_state=filter_state.clone() />
                <RequestsList
                    summaries=filtered_summaries
                    loading=requests_loading
                    error=requests_error
                    on_select=on_select
                />
            </RequestsLayout>
            <RequestDetailModal selected=selected_request />
        </>
    }
}
