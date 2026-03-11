use crate::api::{ApiError, CreateLeaveRequest, UpdateLeaveRequest};
use crate::components::error::InlineErrorMessage;
use crate::components::forms::DatePicker;
use crate::components::layout::SuccessMessage;
use crate::pages::requests::types::RequestKind;
use crate::pages::requests::{
    utils::{EditTarget, LeaveFormState, MessageState},
    view_model::EditPayload,
};
use leptos::*;

#[component]
pub fn LeaveRequestForm(
    state: LeaveFormState,
    message: RwSignal<MessageState>,
    action: Action<CreateLeaveRequest, Result<(), ApiError>>,
    update_action: Action<EditPayload, Result<(), ApiError>>,
    editing: RwSignal<Option<EditTarget>>,
    on_cancel_edit: Callback<()>,
) -> impl IntoView {
    let pending = action.pending();
    let updating = update_action.pending();

    let on_submit = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            match state.to_payload() {
                Ok(payload) => {
                    message.update(|msg| msg.clear());
                    if let Some(target) = editing.get() {
                        update_action.dispatch(
                            (
                                target.id,
                                UpdateLeaveRequest {
                                    leave_type: payload.leave_type,
                                    start_date: payload.start_date,
                                    end_date: payload.end_date,
                                    reason: payload.reason,
                                },
                            )
                                .into(),
                        );
                    } else {
                        action.dispatch(payload);
                    }
                }
                Err(err) => {
                    message.update(|msg| msg.set_error(err));
                }
            }
        }
    };

    let leave_type = state.leave_type_signal();
    let start_signal = state.start_signal();
    let end_signal = state.end_signal();
    let reason_signal = state.reason_signal();
    view! {
        <div class="bg-surface-elevated rounded-2xl shadow-sm border border-border p-6 space-y-4">
            <div>
                <h3 class="text-lg font-display font-bold text-fg">{rust_i18n::t!("pages.requests.leave_form.title")}</h3>
                <p class="text-sm text-fg-muted">{rust_i18n::t!("pages.requests.leave_form.description")} </p>
                <Show
                    when=move || {
                        editing
                            .get()
                            .map(|target| target.kind == RequestKind::Leave)
                            .unwrap_or(false)
                    }
                >
                    <p class="mt-1 text-xs text-status-info-text bg-status-info-bg border border-status-info-border rounded px-2 py-1 inline-flex items-center gap-2">
                        {rust_i18n::t!("pages.requests.forms.editing_leave")}
                        <button class="text-status-info-text underline" on:click=move |_| on_cancel_edit.call(())>
                            {rust_i18n::t!("common.actions.cancel")}
                        </button>
                    </p>
                </Show>
            </div>
            <Show when=move || message.get().error.is_some()>
                <InlineErrorMessage error={Signal::derive(move || message.get().error)} />
            </Show>
            <Show when=move || message.get().success.is_some()>
                <SuccessMessage message={message.get().success.clone().unwrap_or_default()} />
            </Show>
            <form class="space-y-4" on:submit=on_submit>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.requests.leave_form.type_label")}</label>
                    <select
                        class="mt-1 block w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        prop:value=move || leave_type.get()
                        on:change=move |ev| leave_type.set(event_target_value(&ev))
                    >
                        <option value="annual">{rust_i18n::t!("pages.requests.leave_form.types.annual")}</option>
                        <option value="sick">{rust_i18n::t!("pages.requests.leave_form.types.sick")}</option>
                        <option value="personal">{rust_i18n::t!("pages.requests.leave_form.types.personal")}</option>
                        <option value="other">{rust_i18n::t!("pages.requests.leave_form.types.other")}</option>
                    </select>
                </div>
                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
                    <DatePicker
                        label=Some("pages.requests.leave_form.start_date")
                        value=start_signal
                    />
                    <DatePicker
                        label=Some("pages.requests.leave_form.end_date")
                        value=end_signal
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.requests.leave_form.reason_label")}</label>
                    <textarea
                        rows=3
                        class="mt-1 block w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        prop:value=move || reason_signal.get()
                        on:input=move |ev| reason_signal.set(event_target_value(&ev))
                    ></textarea>
                </div>
                <button
                    type="submit"
                    class="px-4 py-2 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                    disabled=move || pending.get() || updating.get()
                >
                    {move || {
                        if pending.get() || updating.get() {
                            rust_i18n::t!("pages.requests.forms.submitting").into_owned()
                        } else if editing
                            .get()
                            .map(|target| target.kind == RequestKind::Leave)
                            .unwrap_or(false)
                        {
                            rust_i18n::t!("pages.requests.leave_form.actions.update").into_owned()
                        } else {
                            rust_i18n::t!("pages.requests.leave_form.actions.submit").into_owned()
                        }
                    }}
                </button>
            </form>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn leave_request_form_renders_editing_state() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let state = LeaveFormState::default();
            let message = create_rw_signal(MessageState::default());
            message.update(|msg| msg.set_success("ok".to_string()));
            let action = create_action(|_| async move { Ok::<(), ApiError>(()) });
            let update_action = create_action(|_| async move { Ok::<(), ApiError>(()) });
            let editing = create_rw_signal(Some(EditTarget {
                id: "req-1".into(),
                kind: RequestKind::Leave,
            }));
            view! {
                <LeaveRequestForm
                    state=state
                    message=message
                    action=action
                    update_action=update_action
                    editing=editing
                    on_cancel_edit=Callback::new(|_| {})
                />
            }
        });
        assert!(html.contains("Leave Request"));
        assert!(html.contains("Editing"));
        assert!(html.contains("Update Leave Request"));
    }
}
