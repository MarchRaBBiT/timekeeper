use crate::api::CreateOvertimeRequest;
use crate::components::layout::{ErrorMessage, SuccessMessage};
use crate::pages::requests::utils::{MessageState, OvertimeFormState};
use leptos::*;

#[component]
pub fn OvertimeRequestForm(
    state: OvertimeFormState,
    message: RwSignal<MessageState>,
    action: Action<CreateOvertimeRequest, Result<(), String>>,
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
                    disabled=move || pending.get()
                >
                    {move || if pending.get() { "送信中..." } else { "残業申請を送信" }}
                </button>
            </form>
        </div>
    }
}
