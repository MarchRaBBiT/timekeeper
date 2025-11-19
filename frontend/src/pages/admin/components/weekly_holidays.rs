use crate::{
    components::layout::{ErrorMessage, LoadingSpinner, SuccessMessage},
    pages::admin::{
        repository,
        utils::{next_allowed_weekly_start, weekday_label, WeeklyHolidayFormState},
    },
    utils::time::today_in_app_tz,
};
use leptos::{ev, *};

#[component]
pub fn WeeklyHolidaySection(
    admin_allowed: Memo<bool>,
    system_admin_allowed: Memo<bool>,
) -> impl IntoView {
    let default_start = next_allowed_weekly_start(
        today_in_app_tz(),
        system_admin_allowed.try_with(|flag| *flag).unwrap_or(false),
    )
    .format("%Y-%m-%d")
    .to_string();
    let form_state = WeeklyHolidayFormState::new("0", default_start);
    let weekday_signal = form_state.weekday_signal();
    let starts_on_signal = form_state.starts_on_signal();
    let ends_on_signal = form_state.ends_on_signal();
    let message = create_rw_signal(None::<String>);
    let form_error = create_rw_signal(None::<String>);
    let reload = create_rw_signal(0u32);

    let holidays_resource = create_resource(
        move || (admin_allowed.get(), reload.get()),
        move |(allowed, _)| async move {
            if !allowed {
                Ok(Vec::new())
            } else {
                repository::list_weekly_holidays().await
            }
        },
    );
    let holidays_loading = holidays_resource.loading();
    let holidays_data = Signal::derive(move || {
        holidays_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default()
    });
    let holidays_error =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.err()));

    let create_action = create_action(move |payload: &crate::api::CreateWeeklyHolidayRequest| {
        let payload = payload.clone();
        async move { repository::create_weekly_holiday(payload).await }
    });
    let create_pending = create_action.pending();
    {
        let message = message.clone();
        let form_error = form_error.clone();
        let reload = reload.clone();
        let form_state = form_state.clone();
        create_effect(move |_| {
            if let Some(result) = create_action.value().get() {
                match result {
                    Ok(created) => {
                        message.set(Some(format!(
                            "{} ({}) を登録しました。",
                            weekday_label(created.weekday),
                            created.starts_on.format("%Y-%m-%d")
                        )));
                        form_error.set(None);
                        form_state.reset_starts_on(created.starts_on);
                        form_state.reset_ends_on();
                        reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => form_error.set(Some(err)),
                }
            }
        });
    }

    let on_submit = {
        let form_state = form_state.clone();
        let form_error = form_error.clone();
        let message = message.clone();
        let create_action = create_action.clone();
        let system_admin_allowed = system_admin_allowed.clone();
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            if !admin_allowed.get_untracked() {
                return;
            }
            let min_start =
                next_allowed_weekly_start(today_in_app_tz(), system_admin_allowed.get_untracked());
            message.set(None);
            match form_state.to_payload(min_start) {
                Ok(payload) => {
                    form_error.set(None);
                    create_action.dispatch(payload);
                }
                Err(err) => form_error.set(Some(err)),
            }
        }
    };

    let on_refresh = {
        let reload = reload.clone();
        move |_| reload.update(|value| *value = value.wrapping_add(1))
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
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
            <form class="grid gap-3 md:grid-cols-3" on:submit=on_submit>
                <div class="md:col-span-1">
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
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"稼働開始日"}</label>
                    <input
                        type="date"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || starts_on_signal.get()}
                        on:input=move |ev| starts_on_signal.set(event_target_value(&ev))
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
                <div class="md:col-span-1">
                    <label class="block text-sm font-medium text-gray-700">{"稼働終了日（任意）"}</label>
                    <input
                        type="date"
                        class="mt-1 w-full border rounded px-2 py-1"
                        prop:value={move || ends_on_signal.get()}
                        on:input=move |ev| ends_on_signal.set(event_target_value(&ev))
                    />
                </div>
                <div class="md:col-span-3">
                    <button
                        type="submit"
                        class="w-full md:w-auto px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50"
                        disabled={move || create_pending.get()}
                    >
                        {move || if create_pending.get() { "登録中..." } else { "週次休日を登録" }}
                    </button>
                </div>
            </form>
            <Show when=move || form_error.get().is_some()>
                <ErrorMessage message={form_error.get().unwrap_or_default()} />
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
                                            <td class="px-4 py-2">{weekday_label(item.weekday)}</td>
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
