use crate::pages::requests::{
    components::{
        detail_modal::RequestDetailModal, filter::RequestsFilter, leave_form::LeaveRequestForm,
        list::RequestsList, overtime_form::OvertimeRequestForm,
    },
    layout::RequestsLayout,
    view_model::{use_requests_view_model, RequestFormKind},
};
use leptos::*;

#[component]
pub fn RequestsPage() -> impl IntoView {
    let vm = use_requests_view_model();
    let leave_state = vm.leave_state.clone();
    let overtime_state = vm.overtime_state.clone();
    let active_form = vm.active_form;
    let set_active_form = vm.set_active_form;
    let leave_message = vm.leave_message;
    let overtime_message = vm.overtime_message;
    let list_message = vm.list_message;
    let leave_action = vm.leave_action;
    let overtime_action = vm.overtime_action;
    let update_action = vm.update_action;
    let editing_request = vm.editing_request;
    let filter_state = vm.filter_state.clone();
    let filtered_summaries = vm.filtered_summaries();
    let requests_resource = vm.requests_resource;
    let selected_request = vm.selected_request;
    let on_edit = vm.on_edit();
    let on_cancel_request = vm.on_cancel_request();

    let on_cancel_leave = Callback::new({
        let leave_state = leave_state.clone();
        move |_| {
            editing_request.set(None);
            list_message.update(|msg| msg.clear());
            leave_state.reset();
        }
    });

    let on_cancel_overtime = Callback::new({
        let overtime_state = overtime_state.clone();
        move |_| {
            editing_request.set(None);
            list_message.update(|msg| msg.clear());
            overtime_state.reset();
        }
    });

    view! {
        <>
            <RequestsLayout>
                <div class="lg:hidden space-y-6">
                    <div class="flex p-1.5 gap-1.5 rounded-2xl bg-slate-100/50 border border-slate-200/50 shadow-inner">
                        <button
                            class=move || {
                                let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                                if matches!(active_form.get(), RequestFormKind::Leave) {
                                    format!("{base} bg-white text-slate-900 shadow-sm transition-all duration-300")
                                } else {
                                    format!("{base} text-slate-500 hover:text-slate-700")
                                }
                            }
                            on:click=move |_| set_active_form.set(RequestFormKind::Leave)
                        >
                            {"休暇申請"}
                        </button>
                        <button
                            class=move || {
                                let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                                if matches!(active_form.get(), RequestFormKind::Overtime) {
                                    format!("{base} bg-white text-slate-900 shadow-sm transition-all duration-300")
                                } else {
                                    format!("{base} text-slate-500 hover:text-slate-700")
                                }
                            }
                            on:click=move |_| set_active_form.set(RequestFormKind::Overtime)
                        >
                            {"残業申請"}
                        </button>
                    </div>
                    <Show when=move || matches!(active_form.get(), RequestFormKind::Leave)>
                        <LeaveRequestForm
                            state=leave_state.clone()
                            message=leave_message
                            action=leave_action
                            update_action=update_action
                            editing=editing_request
                            on_cancel_edit=on_cancel_leave
                        />
                    </Show>
                    <Show when=move || matches!(active_form.get(), RequestFormKind::Overtime)>
                        <OvertimeRequestForm
                            state=overtime_state.clone()
                            message=overtime_message
                            action=overtime_action
                            update_action=update_action
                            editing=editing_request
                            on_cancel_edit=on_cancel_overtime
                        />
                    </Show>
                </div>
                <div class="hidden lg:grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <LeaveRequestForm
                        state=leave_state.clone()
                        message=leave_message
                        action=leave_action
                        update_action=update_action
                        editing=editing_request
                        on_cancel_edit=on_cancel_leave
                    />
                    <OvertimeRequestForm
                        state=overtime_state.clone()
                        message=overtime_message
                        action=overtime_action
                        update_action=update_action
                        editing=editing_request
                        on_cancel_edit=on_cancel_overtime
                    />
                </div>
                <RequestsFilter filter_state=filter_state.clone() />
                <RequestsList
                    summaries=filtered_summaries
                    loading=requests_resource.loading()
                    error=Signal::derive(move || requests_resource.get().and_then(|result| result.err()))
                    on_select=Callback::new(move |s| selected_request.set(Some(s)))
                    on_edit=on_edit
                    on_cancel=on_cancel_request
                    message=list_message
                />
            </RequestsLayout>
            <RequestDetailModal selected=selected_request />
        </>
    }
}
