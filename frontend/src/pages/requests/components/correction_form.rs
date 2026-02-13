use crate::api::{ApiError, CreateAttendanceCorrectionRequest};
use crate::components::error::InlineErrorMessage;
use crate::components::forms::DatePicker;
use crate::components::layout::SuccessMessage;
use crate::pages::requests::types::RequestKind;
use crate::pages::requests::{
    utils::{AttendanceCorrectionFormState, EditTarget, MessageState},
    view_model::AttendanceCorrectionEditPayload,
};
use leptos::*;

#[component]
pub fn AttendanceCorrectionRequestForm(
    state: AttendanceCorrectionFormState,
    message: RwSignal<MessageState>,
    action: Action<CreateAttendanceCorrectionRequest, Result<(), ApiError>>,
    update_action: Action<AttendanceCorrectionEditPayload, Result<(), ApiError>>,
    editing: RwSignal<Option<EditTarget>>,
    on_cancel_edit: Callback<()>,
) -> impl IntoView {
    let pending = action.pending();
    let updating = update_action.pending();

    let on_submit = {
        let state = state;
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            message.update(|msg| msg.clear());
            if let Some(target) = editing.get() {
                match state.to_update_payload() {
                    Ok(payload) => update_action.dispatch((target.id, payload).into()),
                    Err(err) => message.update(|msg| msg.set_error(err)),
                }
            } else {
                match state.to_create_payload() {
                    Ok(payload) => action.dispatch(payload),
                    Err(err) => message.update(|msg| msg.set_error(err)),
                }
            }
        }
    };

    let date_signal = state.date_signal();
    let clock_in_signal = state.clock_in_signal();
    let clock_out_signal = state.clock_out_signal();
    let break_rows_signal = state.break_rows_signal();
    let reason_signal = state.reason_signal();
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-lg font-medium text-fg">{"勤怠修正依頼"}</h3>
                <p class="text-sm text-fg-muted">{"出勤・退勤・休憩を入力して修正依頼を送信します。"} </p>
                <Show when=move || editing
                    .get()
                    .map(|target| target.kind == RequestKind::AttendanceCorrection)
                    .unwrap_or(false)>
                    <p class="mt-1 text-xs text-status-info-text bg-status-info-bg border border-status-info-border rounded px-2 py-1 inline-flex items-center gap-2">
                        {"編集中: 既存の勤怠修正依頼を更新します。"}
                        <button class="text-status-info-text underline" on:click=move |_| on_cancel_edit.call(())>
                            {"キャンセル"}
                        </button>
                    </p>
                </Show>
            </div>
            <Show when=move || message.get().error.is_some()>
                <InlineErrorMessage error={Signal::derive(move || message.get().error).into()} />
            </Show>
            <Show when=move || message.get().success.is_some()>
                <SuccessMessage message={message.get().success.clone().unwrap_or_default()} />
            </Show>
            <form class="space-y-4" on:submit=on_submit>
                <DatePicker
                    label=Some("対象日")
                    value=date_signal
                />
                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
                    <div>
                        <label class="block text-sm font-medium text-fg-muted">{"出勤時刻"}</label>
                        <input
                            type="datetime-local"
                            class="mt-1 block w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                            prop:value=move || clock_in_signal.get()
                            on:input=move |ev| clock_in_signal.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-fg-muted">{"退勤時刻"}</label>
                        <input
                            type="datetime-local"
                            class="mt-1 block w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                            prop:value=move || clock_out_signal.get()
                            on:input=move |ev| clock_out_signal.set(event_target_value(&ev))
                        />
                    </div>
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {"休憩（1行ごとに `開始,終了(任意)`）"}
                    </label>
                    <textarea
                        rows=4
                        class="mt-1 block w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 font-mono text-sm"
                        placeholder="2026-02-13T12:00,2026-02-13T13:00"
                        prop:value=move || break_rows_signal.get()
                        on:input=move |ev| break_rows_signal.set(event_target_value(&ev))
                    ></textarea>
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">{"修正理由"}</label>
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
                            "送信中..."
                        } else if editing
                            .get()
                            .map(|target| target.kind == RequestKind::AttendanceCorrection)
                            .unwrap_or(false)
                        {
                            "勤怠修正依頼を更新"
                        } else {
                            "勤怠修正依頼を送信"
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
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn correction_form_renders_editing_state() {
        let html = render_to_string(move || {
            let state = AttendanceCorrectionFormState::default();
            let message = create_rw_signal(MessageState::default());
            let action = create_action(|_| async move { Ok::<(), ApiError>(()) });
            let update_action = create_action(|_| async move { Ok::<(), ApiError>(()) });
            let editing = create_rw_signal(Some(EditTarget {
                id: "req-corr-1".into(),
                kind: RequestKind::AttendanceCorrection,
            }));
            view! {
                <AttendanceCorrectionRequestForm
                    state=state
                    message=message
                    action=action
                    update_action=update_action
                    editing=editing
                    on_cancel_edit=Callback::new(|_| {})
                />
            }
        });
        assert!(html.contains("勤怠修正依頼"));
        assert!(html.contains("編集中"));
        assert!(html.contains("勤怠修正依頼を更新"));
    }
}
