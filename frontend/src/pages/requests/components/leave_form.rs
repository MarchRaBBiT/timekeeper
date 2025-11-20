use crate::api::CreateLeaveRequest;
use crate::components::layout::{ErrorMessage, SuccessMessage};
use crate::pages::requests::utils::{LeaveFormState, MessageState};
use leptos::*;

#[component]
pub fn LeaveRequestForm(
    state: LeaveFormState,
    message: RwSignal<MessageState>,
    action: Action<CreateLeaveRequest, Result<(), String>>,
) -> impl IntoView {
    let pending = action.pending();
    let on_submit = {
        let state = state.clone();
        let message = message.clone();
        let action = action.clone();
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            match state.to_payload() {
                Ok(payload) => {
                    message.update(|msg| msg.clear());
                    action.dispatch(payload);
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
            </div>
            <Show when=move || message.get().error.is_some()>
                <ErrorMessage message={message.get().error.clone().unwrap_or_default()} />
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
                <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <div>
                        <label class="block text-sm font-medium text-gray-700">{"開始日"}</label>
                        <input
                            type="date"
                            class="mt-1 block w-full border rounded px-2 py-1"
                            prop:value=move || start_signal.get()
                            on:input=move |ev| start_signal.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700">{"終了日"}</label>
                        <input
                            type="date"
                            class="mt-1 block w-full border rounded px-2 py-1"
                            prop:value=move || end_signal.get()
                            on:input=move |ev| end_signal.set(event_target_value(&ev))
                        />
                    </div>
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
                    disabled=move || pending.get()
                >
                    {move || if pending.get() { "送信中..." } else { "休暇申請を送信" }}
                </button>
            </form>
        </div>
    }
}
