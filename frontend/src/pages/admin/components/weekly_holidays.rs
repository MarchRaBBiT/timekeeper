use crate::{
    api::{CreateWeeklyHolidayRequest, WeeklyHolidayResponse},
    components::{
        forms::DatePicker,
        layout::{ErrorMessage, LoadingSpinner, SuccessMessage},
    },
    pages::admin::utils::WeeklyHolidayFormState,
    utils::time::today_in_app_tz,
};
use leptos::{ev, *};

#[component]
pub fn WeeklyHolidaySection(
    state: WeeklyHolidayFormState,
    resource: Resource<(bool, u32), Result<Vec<WeeklyHolidayResponse>, String>>,
    action: Action<CreateWeeklyHolidayRequest, Result<WeeklyHolidayResponse, String>>,
    reload: RwSignal<u32>,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
    admin_allowed: Memo<bool>,
    system_admin_allowed: Memo<bool>,
) -> impl IntoView {
    let weekday_signal = state.weekday_signal();
    let starts_on_signal = state.starts_on_signal();
    let ends_on_signal = state.ends_on_signal();

    let holidays_loading = resource.loading();
    let holidays_data = Signal::derive(move || {
        resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default()
    });
    let holidays_error = Signal::derive(move || resource.get().and_then(|result| result.err()));

    let create_pending = action.pending();

    // Effects and submission logic
    create_effect(move |_| {
        if let Some(result) = action.value().get() {
            match result {
                Ok(created) => {
                    message.set(Some(format!(
                        "{} ({}) を登録しました。",
                        crate::pages::admin::utils::weekday_label(created.weekday),
                        created.starts_on.format("%Y-%m-%d")
                    )));
                    error.set(None);
                    state.reset_starts_on(created.starts_on);
                    state.reset_ends_on();
                    reload.update(|value| *value = value.wrapping_add(1));
                }
                Err(err) => error.set(Some(err)),
            }
        }
    });

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        if !admin_allowed.get_untracked() {
            return;
        }
        let min_start = crate::pages::admin::utils::next_allowed_weekly_start(
            today_in_app_tz(),
            system_admin_allowed.get_untracked(),
        );
        message.set(None);
        match state.to_payload(min_start) {
            Ok(payload) => {
                error.set(None);
                action.dispatch(payload);
            }
            Err(err) => error.set(Some(err)),
        }
    };

    let on_refresh = move |_| reload.update(|value| *value = value.wrapping_add(1));

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900">{"週次休日"}</h2>
                    <p class="text-sm text-gray-600">
                        {"週単位の休日を登録します。システム管理者は即日開始も設定できます。"}
                    </p>
                </div>
                <button
                    class="px-3 py-1 rounded border text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                    disabled={move || holidays_loading.get()}
                    on:click=on_refresh
                >
                    {"再取得"}
                </button>
            </div>
            <form class="grid gap-3 lg:grid-cols-3" on:submit=on_submit>
                <div class="lg:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"曜日"}</label>
                    <select
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || weekday_signal.get()}
                        on:change=move |ev| weekday_signal.set(event_target_value(&ev))
                    >
                        <option value="0">{"日 (0)"}</option>
                        <option value="1">{"月 (1)"}</option>
                        <option value="2">{"火 (2)"}</option>
                        <option value="3">{"水 (3)"}</option>
                        <option value="4">{"木 (4)"}</option>
                        <option value="5">{"金 (5)"}</option>
                        <option value="6">{"土 (6)"}</option>
                    </select>
                </div>
                <div class="lg:col-span-1">
                    <DatePicker
                        label=Some("稼働開始日")
                        value=starts_on_signal
                    />
                    <p class="text-xs text-gray-500 mt-1">
                        {move || {
                            if system_admin_allowed.get() {
                                "システム管理者は本日から設定できます。"
                            } else {
                                "通常管理者は翌日以降が設定可能です。"
                            }
                        }}
                    </p>
                </div>
                <div class="lg:col-span-1">
                    <DatePicker
                        label=Some("稼働終了日（任意）")
                        value=ends_on_signal
                    />
                </div>
                <div class="lg:col-span-3">
                    <button
                        type="submit"
                        class="w-full lg:w-auto px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                        disabled={move || create_pending.get()}
                    >
                        {move || if create_pending.get() { "登録中..." } else { "週次休日を登録" }}
                    </button>
                </div>
            </form>
            <Show when=move || error.get().is_some()>
                <ErrorMessage message={error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || message.get().is_some()>
                <SuccessMessage message={message.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_error.get().is_some()>
                <ErrorMessage message={holidays_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_loading.get()>
                <div class="flex items-center gap-2 text-sm text-gray-600">
                    <LoadingSpinner />
                    <span>{"週次休日を読み込み中です..."}</span>
                </div>
            </Show>
            <Show when=move || !holidays_loading.get() && holidays_data.get().is_empty()>
                <p class="text-sm text-gray-500">{"登録済みの週次休日はありません。"} </p>
            </Show>
            <Show when=move || !holidays_loading.get() && !holidays_data.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200 text-sm">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left text-gray-600">{"曜日"}</th>
                                <th class="px-4 py-2 text-left text-gray-600">{"指定期間"}</th>
                                <th class="px-4 py-2 text-left text-gray-600">{"適用期間"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-100">
                            <For
                                each=move || holidays_data.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    view! {
                                        <tr>
                                            <td class="px-4 py-2">{crate::pages::admin::utils::weekday_label(item.weekday)}</td>
                                            <td class="px-4 py-2 text-gray-600">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.starts_on.format("%Y-%m-%d"),
                                                    item
                                                        .ends_on
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "未設定".into())
                                                )}
                                            </td>
                                            <td class="px-4 py-2 text-gray-600">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.enforced_from.format("%Y-%m-%d"),
                                                    item
                                                        .enforced_to
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "適用中".into())
                                                )}
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}
