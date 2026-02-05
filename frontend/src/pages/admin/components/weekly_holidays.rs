use crate::{
    api::{ApiError, CreateWeeklyHolidayRequest, WeeklyHolidayResponse},
    components::{
        error::InlineErrorMessage,
        forms::DatePicker,
        layout::{LoadingSpinner, SuccessMessage},
    },
    pages::admin::utils::WeeklyHolidayFormState,
    utils::time::today_in_app_tz,
};
use leptos::{ev, *};

#[component]
pub fn WeeklyHolidaySection(
    state: WeeklyHolidayFormState,
    resource: Resource<(bool, u32), Result<Vec<WeeklyHolidayResponse>, ApiError>>,
    action: Action<CreateWeeklyHolidayRequest, Result<WeeklyHolidayResponse, ApiError>>,
    delete_action: Action<String, Result<(), ApiError>>,
    reload: RwSignal<u32>,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<ApiError>>,
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

    create_effect(move |_| {
        if let Some(result) = delete_action.value().get() {
            match result {
                Ok(_) => {
                    message.set(Some("週次休日を削除しました。".into()));
                    error.set(None);
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
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-1 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-fg">{"週次休日"}</h2>
                    <p class="text-sm text-fg-muted">
                        {"週単位の休日を登録します。システム管理者は即日開始も設定できます。"}
                    </p>
                </div>
                <button
                    class="px-3 py-1 rounded border border-border text-sm text-fg hover:bg-action-ghost-bg-hover disabled:opacity-50"
                    disabled={move || holidays_loading.get()}
                    on:click=on_refresh
                >
                    {"再取得"}
                </button>
            </div>
            <form class="grid gap-3 lg:grid-cols-3" on:submit=on_submit>
                <div class="lg:col-span-1">
                    <label class="block text-sm font-bold text-fg-muted ml-1 mb-1.5">{"曜日"}</label>
                    <div class="relative">
                        <select
                            class="appearance-none w-full rounded-xl border-2 border-form-control-border bg-form-control-bg text-fg py-2.5 px-4 shadow-sm focus:outline-none focus:border-action-primary-border-hover focus:ring-4 focus:ring-action-primary-focus transition-all duration-200"
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
                        <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-4 text-fg-muted">
                            <i class="fas fa-chevron-down text-xs"></i>
                        </div>
                    </div>
                </div>
                <div class="lg:col-span-1">
                    <DatePicker
                        label=Some("稼働開始日")
                        value=starts_on_signal
                    />
                    <p class="text-xs text-fg-muted mt-1">
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
                        class="w-full lg:w-auto px-4 py-2 rounded bg-action-primary-bg text-action-primary-text hover:bg-action-primary-bg-hover disabled:opacity-50"
                        disabled={move || create_pending.get()}
                    >
                        {move || if create_pending.get() { "登録中..." } else { "週次休日を登録" }}
                    </button>
                </div>
            </form>
            <Show when=move || error.get().is_some()>
                <InlineErrorMessage error={error.into()} />
            </Show>
            <Show when=move || message.get().is_some()>
                <SuccessMessage message={message.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_error.get().is_some()>
                <InlineErrorMessage error={holidays_error} />
            </Show>
            <Show when=move || holidays_loading.get()>
                <div class="flex items-center gap-2 text-sm text-fg-muted">
                    <LoadingSpinner />
                    <span>{"週次休日を読み込み中です..."}</span>
                </div>
            </Show>
            <Show when=move || !holidays_loading.get() && holidays_data.get().is_empty()>
                <p class="text-sm text-fg-muted">{"登録済みの週次休日はありません。"} </p>
            </Show>
            <Show when=move || !holidays_loading.get() && !holidays_data.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-border text-sm">
                        <thead class="bg-surface-muted">
                            <tr>
                                <th class="px-4 py-2 text-left text-fg-muted">{"曜日"}</th>
                                <th class="px-4 py-2 text-left text-fg-muted">{"指定期間"}</th>
                                <th class="px-4 py-2 text-left text-fg-muted">{"適用期間"}</th>
                                <th class="px-4 py-2 text-right text-fg-muted">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border">
                            <For
                                each=move || holidays_data.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    view! {
                                        <tr>
                                            <td class="px-4 py-2 text-fg">{crate::pages::admin::utils::weekday_label(item.weekday)}</td>
                                            <td class="px-4 py-2 text-fg-muted">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.starts_on.format("%Y-%m-%d"),
                                                    item
                                                        .ends_on
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "未設定".into())
                                                )}
                                            </td>
                                            <td class="px-4 py-2 text-fg-muted">
                                                {format!(
                                                    "{} 〜 {}",
                                                    item.enforced_from.format("%Y-%m-%d"),
                                                    item
                                                        .enforced_to
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "適用中".into())
                                                )}
                                            </td>
                                            <td class="px-4 py-2 text-right">
                                                <button
                                                    class="text-action-danger-bg hover:text-action-danger-bg-hover disabled:opacity-50"
                                                    disabled={move || delete_action.pending().get()}
                                                    on:click={
                                                        let id = item.id.clone();
                                                        move |_| {
                                                            if let Some(window) = web_sys::window() {
                                                                if let Ok(true) = window.confirm_with_message("この週次休日を削除してもよろしいですか？") {
                                                                    delete_action.dispatch(id.clone());
                                                                }
                                                            }
                                                        }
                                                    }
                                                >
                                                    {"削除"}
                                                </button>
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    fn sample_item() -> WeeklyHolidayResponse {
        WeeklyHolidayResponse {
            id: "wh1".into(),
            weekday: 1,
            starts_on: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            ends_on: None,
            enforced_from: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            enforced_to: None,
        }
    }

    fn render_with_resource(
        items: Vec<WeeklyHolidayResponse>,
        admin_allowed: bool,
        system_admin_allowed: bool,
    ) -> String {
        render_to_string(move || {
            let state = WeeklyHolidayFormState::new("1", "2025-01-02".into());
            let resource = Resource::new(|| (true, 0u32), |_| async move { Ok(Vec::new()) });
            resource.set(Ok(items.clone()));
            let action =
                create_action(|_: &CreateWeeklyHolidayRequest| async move { Ok(sample_item()) });
            let delete_action = create_action(|_: &String| async move { Ok(()) });
            let reload = create_rw_signal(0u32);
            let message = create_rw_signal(None::<String>);
            let error = create_rw_signal(None::<ApiError>);
            let admin_allowed = create_memo(move |_| admin_allowed);
            let system_admin_allowed = create_memo(move |_| system_admin_allowed);
            view! {
                <WeeklyHolidaySection
                    state=state
                    resource=resource
                    action=action
                    delete_action=delete_action
                    reload=reload
                    message=message
                    error=error
                    admin_allowed=admin_allowed
                    system_admin_allowed=system_admin_allowed
                />
            }
        })
    }

    #[test]
    fn weekly_holiday_section_renders_empty_state() {
        let html = render_with_resource(Vec::new(), true, false);
        assert!(html.contains("週次休日"));
        assert!(html.contains("登録済みの週次休日はありません。"));
    }

    #[test]
    fn weekly_holiday_section_renders_table() {
        let html = render_with_resource(vec![sample_item()], true, true);
        assert!(html.contains("週次休日"));
        assert!(html.contains("月"));
    }
}
