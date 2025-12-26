use crate::api::CreateOvertimeRequest;
use crate::components::layout::{ErrorMessage, SuccessMessage};
use crate::pages::requests::types::RequestKind;
use crate::pages::requests::{
    utils::{EditTarget, MessageState, OvertimeFormState},
    view_model::EditPayload,
};
use leptos::*;
use serde_json::to_value;

#[component]
pub fn OvertimeRequestForm(
    state: OvertimeFormState,
    message: RwSignal<MessageState>,
    action: Action<CreateOvertimeRequest, Result<(), String>>,
    update_action: Action<EditPayload, Result<(), String>>,
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
                Err(err) => message.update(|msg| msg.set_error(err)),
            }
        }
    };

    let date_signal = state.date_signal();
    let hours_signal = state.hours_signal();
    let reason_signal = state.reason_signal();
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-lg font-medium text-gray-900">{"残業申請"}</h3>
                <p class="text-sm text-gray-600">{"残業予定日と時間を入力して申請を送信します。"} </p>
                <Show when=move || editing
                    .get()
                    .map(|target| target.kind == RequestKind::Overtime)
                    .unwrap_or(false)>
                    <p class="mt-1 text-xs text-indigo-700 bg-indigo-50 border border-indigo-200 rounded px-2 py-1 inline-flex items-center gap-2">
                        {"編集中: 既存の残業申請を更新します。"}
                        <button class="text-indigo-700 underline" on:click=move |_| on_cancel_edit.call(())>
                            {"キャンセル"}
                        </button>
                    </p>
                </Show>
            </div>
            <Show when=move || message.get().error.is_some()>
                <ErrorMessage message={message.get().error.clone().unwrap_or_default()} />
            </Show>
            <Show when=move || message.get().success.is_some()>
                <SuccessMessage message={message.get().success.clone().unwrap_or_default()} />
            </Show>
            <form class="space-y-4" on:submit=on_submit>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"残業日"}</label>
                    <input
                        type="date"
                        class="mt-1 block w-full border rounded px-2 py-1"
                        prop:value=move || date_signal.get()
                        on:input=move |ev| date_signal.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"予定時間（時間）"}</label>
                    <input
                        type="number"
                        step="0.25"
                        min="0.25"
                        class="mt-1 block w-full border rounded px-2 py-1"
                        prop:value=move || hours_signal.get()
                        on:input=move |ev| hours_signal.set(event_target_value(&ev))
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
                    class="px-4 py-2 rounded bg-indigo-600 text-white disabled:opacity-50"
                    disabled=move || pending.get() || updating.get()
                >
                    {move || {
                        if pending.get() || updating.get() {
                            "送信中..."
                        } else if editing
                            .get()
                            .map(|target| target.kind == RequestKind::Overtime)
                            .unwrap_or(false)
                        {
                            "残業申請を更新"
                        } else {
                            "残業申請を送信"
                        }
                    }}
                </button>
            </form>
        </div>
    }
}
