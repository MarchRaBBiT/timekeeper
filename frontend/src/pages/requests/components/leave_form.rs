use crate::api::{ApiError, CreateLeaveRequest};
use crate::components::forms::DatePicker;
use crate::components::layout::SuccessMessage;
use crate::components::error::InlineErrorMessage;
use crate::pages::requests::types::RequestKind;
use crate::pages::requests::{
    utils::{EditTarget, LeaveFormState, MessageState},
    view_model::EditPayload,
};
use leptos::*;
use serde_json::to_value;

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
        let state = state.clone();
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            match state.to_payload() {
                Ok(payload) => {
                    message.update(|msg| msg.clear());
                    if let Some(target) = editing.get() {
                        update_action
                            .dispatch((target.id, to_value(payload).unwrap_or_default()).into());
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
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-lg font-medium text-gray-900">{"休暇申請"}</h3>
                <p class="text-sm text-gray-600">{"休暇の種類と期間を入力して申請を送信します。"} </p>
                <Show
                    when=move || {
                        editing
                            .get()
                            .map(|target| target.kind == RequestKind::Leave)
                            .unwrap_or(false)
                    }
                >
                    <p class="mt-1 text-xs text-blue-700 bg-blue-50 border border-blue-200 rounded px-2 py-1 inline-flex items-center gap-2">
                        {"編集中: 既存の休暇申請を更新します。"}
                        <button class="text-blue-700 underline" on:click=move |_| on_cancel_edit.call(())>
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
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"種類"}</label>
                    <select
                        class="mt-1 block w-full border rounded px-2 py-1"
                        prop:value=move || leave_type.get()
                        on:change=move |ev| leave_type.set(event_target_value(&ev))
                    >
                        <option value="annual">{"年次有給"}</option>
                        <option value="sick">{"病気"}</option>
                        <option value="personal">{"私用"}</option>
                        <option value="other">{"その他"}</option>
                    </select>
                </div>
                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
                    <DatePicker
                        label=Some("開始日")
                        value=start_signal
                    />
                    <DatePicker
                        label=Some("終了日")
                        value=end_signal
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"理由（任意）"}</label>
                    <textarea
                        rows=3
                        class="mt-1 block w-full border rounded px-2 py-1"
                        prop:value=move || reason_signal.get()
                        on:input=move |ev| reason_signal.set(event_target_value(&ev))
                    ></textarea>
                </div>
                <button
                    type="submit"
                    class="px-4 py-2 rounded bg-blue-600 text-white disabled:opacity-50"
                    disabled=move || pending.get() || updating.get()
                >
                    {move || {
                        if pending.get() || updating.get() {
                            "送信中..."
                        } else if editing
                            .get()
                            .map(|target| target.kind == RequestKind::Leave)
                            .unwrap_or(false)
                        {
                            "休暇申請を更新"
                        } else {
                            "休暇申請を送信"
                        }
                    }}
                </button>
            </form>
        </div>
    }
}
