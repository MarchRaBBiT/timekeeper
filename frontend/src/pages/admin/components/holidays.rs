use crate::{
    api::CreateHolidayRequest,
    components::layout::{ErrorMessage, LoadingSpinner, SuccessMessage},
    pages::admin::repository::AdminRepository,
    utils::time::now_in_app_tz,
};
use chrono::{Datelike, NaiveDate};
use leptos::{ev, *};
use std::collections::HashSet;

#[component]
pub fn HolidayManagementSection(
    repository: AdminRepository,
    admin_allowed: Memo<bool>,
) -> impl IntoView {
    let holidays_reload = create_rw_signal(0u32);
    let holiday_date_input = create_rw_signal(String::new());
    let holiday_name_input = create_rw_signal(String::new());
    let holiday_desc_input = create_rw_signal(String::new());
    let holiday_message = create_rw_signal(None::<String>);
    let holiday_error = create_rw_signal(None::<String>);
    let deleting_id = create_rw_signal(None::<String>);

    let repo_for_holidays = repository.clone();
    let holidays_resource = create_resource(
        move || (admin_allowed.get(), holidays_reload.get()),
        move |(allowed, _)| {
            let repo = repo_for_holidays.clone();
            async move {
                if !allowed {
                    Ok(Vec::new())
                } else {
                    repo.list_holidays().await
                }
            }
        },
    );
    let holidays_loading = holidays_resource.loading();
    let holidays_data = Signal::derive(move || {
        let mut list = holidays_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or_default();
        list.sort_by_key(|h| h.holiday_date);
        list
    });
    let holidays_fetch_error =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.err()));

    let repo_for_create = repository.clone();
    let create_holiday_action = create_action(move |payload: &CreateHolidayRequest| {
        let repo = repo_for_create.clone();
        let payload = payload.clone();
        async move { repo.create_holiday(payload).await }
    });
    let create_pending = create_holiday_action.pending();
    {
        let holiday_message = holiday_message.clone();
        let holiday_error = holiday_error.clone();
        let holidays_reload = holidays_reload.clone();
        let holiday_date_input = holiday_date_input.clone();
        let holiday_name_input = holiday_name_input.clone();
        let holiday_desc_input = holiday_desc_input.clone();
        create_effect(move |_| {
            if let Some(result) = create_holiday_action.value().get() {
                match result {
                    Ok(created) => {
                        holiday_message.set(Some(format!(
                            "{} ({}) を登録しました。",
                            created.name,
                            created.holiday_date.format("%Y-%m-%d")
                        )));
                        holiday_error.set(None);
                        holiday_date_input.set(String::new());
                        holiday_name_input.set(String::new());
                        holiday_desc_input.set(String::new());
                        holidays_reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => {
                        holiday_message.set(None);
                        holiday_error.set(Some(err));
                    }
                }
            }
        });
    }

    let repo_for_delete = repository.clone();
    let delete_holiday_action = create_action(move |id: &String| {
        let repo = repo_for_delete.clone();
        let id = id.clone();
        async move { repo.delete_holiday(&id).await }
    });
    {
        let deleting_id = deleting_id.clone();
        let holidays_reload = holidays_reload.clone();
        let holiday_message = holiday_message.clone();
        let holiday_error = holiday_error.clone();
        create_effect(move |_| {
            if let Some(result) = delete_holiday_action.value().get() {
                match result {
                    Ok(_) => {
                        holiday_message.set(Some("祝日を削除しました。".into()));
                        holiday_error.set(None);
                        deleting_id.set(None);
                        holidays_reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => {
                        holiday_message.set(None);
                        holiday_error.set(Some(err));
                        deleting_id.set(None);
                    }
                }
            }
        });
    }

    let google_year_input = create_rw_signal(now_in_app_tz().year().to_string());
    let google_holidays = create_rw_signal(Vec::<CreateHolidayRequest>::new());
    let google_error = create_rw_signal(None::<String>);
    let repo_for_google = repository.clone();
    let fetch_google_action = create_action(move |year: &Option<i32>| {
        let repo = repo_for_google.clone();
        let year = *year;
        async move { repo.fetch_google_holidays(year).await }
    });
    let google_loading = fetch_google_action.pending();
    {
        let google_holidays = google_holidays.clone();
        let google_error = google_error.clone();
        create_effect(move |_| {
            if let Some(result) = fetch_google_action.value().get() {
                match result {
                    Ok(list) => {
                        google_error.set(None);
                        google_holidays.set(list);
                    }
                    Err(err) => {
                        google_error.set(Some(err));
                        google_holidays.set(Vec::new());
                    }
                }
            }
        });
    }

    let repo_for_import = repository.clone();
    let import_action = create_action(move |payload: &Vec<CreateHolidayRequest>| {
        let repo = repo_for_import.clone();
        let payload = payload.clone();
        async move {
            let mut imported = 0usize;
            for item in payload {
                repo.create_holiday(item.clone()).await?;
                imported += 1;
            }
            Ok(imported)
        }
    });
    {
        let holiday_message = holiday_message.clone();
        let holiday_error = holiday_error.clone();
        let holidays_reload = holidays_reload.clone();
        create_effect(move |_| {
            if let Some(result) = import_action.value().get() {
                match result {
                    Ok(count) => {
                        if count == 0 {
                            holiday_message.set(Some("追加対象の祝日はありません。".into()));
                        } else {
                            holiday_message
                                .set(Some(format!("{} 件の祝日を追加しました。", count)));
                        }
                        holiday_error.set(None);
                        holidays_reload.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(err) => {
                        holiday_message.set(None);
                        holiday_error.set(Some(err));
                    }
                }
            }
        });
    }

    let on_fetch_google = {
        let google_year_input = google_year_input.clone();
        let fetch_google_action = fetch_google_action.clone();
        move |_| {
            let parsed_year = google_year_input.get().trim().parse::<i32>().ok();
            fetch_google_action.dispatch(parsed_year);
        }
    };

    let on_create_holiday = {
        let holiday_date_input = holiday_date_input.clone();
        let holiday_name_input = holiday_name_input.clone();
        let holiday_desc_input = holiday_desc_input.clone();
        let holiday_error = holiday_error.clone();
        let holiday_message = holiday_message.clone();
        let create_holiday_action = create_holiday_action.clone();
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            let date_raw = holiday_date_input.get();
            let name_raw = holiday_name_input.get();
            let desc_raw = holiday_desc_input.get();
            if date_raw.trim().is_empty() || name_raw.trim().is_empty() {
                holiday_error.set(Some("日付と名称を入力してください。".into()));
                holiday_message.set(None);
                return;
            }
            let parsed_date = match NaiveDate::parse_from_str(date_raw.trim(), "%Y-%m-%d") {
                Ok(date) => date,
                Err(_) => {
                    holiday_error.set(Some("日付は YYYY-MM-DD 形式で入力してください。".into()));
                    holiday_message.set(None);
                    return;
                }
            };
            let payload = CreateHolidayRequest {
                holiday_date: parsed_date,
                name: name_raw.trim().to_string(),
                description: if desc_raw.trim().is_empty() {
                    None
                } else {
                    Some(desc_raw.trim().to_string())
                },
            };
            holiday_error.set(None);
            holiday_message.set(None);
            create_holiday_action.dispatch(payload);
        }
    };

    let on_delete_holiday = {
        let delete_holiday_action = delete_holiday_action.clone();
        let deleting_id = deleting_id.clone();
        move |id: String| {
            deleting_id.set(Some(id.clone()));
            delete_holiday_action.dispatch(id);
        }
    };

    let on_import_google = {
        let google_holidays = google_holidays.clone();
        let holidays_data = holidays_data.clone();
        let import_action = import_action.clone();
        let holiday_error = holiday_error.clone();
        let holiday_message = holiday_message.clone();
        move |_| {
            let existing: HashSet<NaiveDate> = holidays_data
                .get()
                .into_iter()
                .map(|h| h.holiday_date)
                .collect();
            let candidates: Vec<CreateHolidayRequest> = google_holidays
                .get()
                .into_iter()
                .filter(|candidate| !existing.contains(&candidate.holiday_date))
                .collect();
            if candidates.is_empty() {
                holiday_message.set(Some("追加対象の祝日はありません。".into()));
                holiday_error.set(None);
                return;
            }
            holiday_error.set(None);
            holiday_message.set(None);
            import_action.dispatch(candidates);
        }
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-gray-900">{"祝日管理"}</h3>
            <form class="grid gap-3 md:grid-cols-3" on:submit=on_create_holiday>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"日付"}</label>
                    <input type="date" class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| holiday_date_input.set(event_target_value(&ev)) />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"名称"}</label>
                    <input class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| holiday_name_input.set(event_target_value(&ev)) />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"備考（任意）"}</label>
                    <input class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| holiday_desc_input.set(event_target_value(&ev)) />
                </div>
                <div class="md:col-span-3">
                    <button
                        type="submit"
                        class="px-4 py-2 rounded bg-blue-600 text-white disabled:opacity-50"
                        disabled={move || create_pending.get()}
                    >
                        {move || if create_pending.get() { "登録中..." } else { "祝日を登録" }}
                    </button>
                </div>
            </form>
            <div class="flex flex-col gap-2 md:flex-row md:items-center md:gap-4">
                <div class="flex items-center gap-2">
                    <input
                        type="number"
                        class="border rounded px-2 py-1 w-32"
                        prop:value={move || google_year_input.get()}
                        on:input=move |ev| google_year_input.set(event_target_value(&ev))
                    />
                    <button
                        class="px-3 py-1 rounded border disabled:opacity-50"
                        disabled={move || google_loading.get()}
                        on:click=on_fetch_google
                    >
                        {move || if google_loading.get() { "取得中..." } else { "Google 祝日取得" }}
                    </button>
                </div>
                <button
                    class="px-3 py-1 rounded bg-emerald-600 text-white disabled:opacity-50"
                    disabled={move || google_holidays.get().is_empty()}
                    on:click=on_import_google
                >
                    {"一覧から登録"}
                </button>
            </div>
            <Show when=move || holiday_error.get().is_some()>
                <ErrorMessage message={holiday_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holiday_message.get().is_some()>
                <SuccessMessage message={holiday_message.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_fetch_error.get().is_some()>
                <ErrorMessage message={holidays_fetch_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || google_error.get().is_some()>
                <ErrorMessage message={google_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_loading.get()>
                <div class="flex items-center gap-2 text-sm text-gray-600">
                    <LoadingSpinner />
                    <span>{"祝日一覧を読み込み中..."}</span>
                </div>
            </Show>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200 text-sm">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-4 py-2 text-left text-gray-600">{"日付"}</th>
                            <th class="px-4 py-2 text-left text-gray-600">{"名称"}</th>
                            <th class="px-4 py-2 text-left text-gray-600">{"備考"}</th>
                            <th class="px-4 py-2 text-right text-gray-600">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-100">
                        <For
                            each=move || holidays_data.get()
                            key=|item| item.id.clone()
                            children=move |item| {
                                let remove = {
                                    let item_id = item.id.clone();
                                    move |_| on_delete_holiday(item_id.clone())
                                };
                                view! {
                                    <tr>
                                        <td class="px-4 py-2">{item.holiday_date.format("%Y-%m-%d").to_string()}</td>
                                        <td class="px-4 py-2">{item.name.clone()}</td>
                                        <td class="px-4 py-2 text-gray-600">{item.description.clone().unwrap_or_default()}</td>
                                        <td class="px-4 py-2 text-right">
                                            <button
                                                class="px-3 py-1 rounded border text-sm disabled:opacity-50"
                                                disabled={move || deleting_id.get().as_deref() == Some(&item.id)}
                                                on:click=remove
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
            <Show when=move || !google_holidays.get().is_empty()>
                <div class="border rounded-lg p-4 space-y-2">
                    <h4 class="text-sm font-medium text-gray-900">{"Google 祝日候補"}</h4>
                    <ul class="space-y-1 text-sm text-gray-700">
                        <For
                            each=move || google_holidays.get()
                            key=|item| (item.name.clone(), item.holiday_date)
                            children=move |item| {
                                view! {
                                    <li class="flex justify-between">
                                        <span>{item.holiday_date.format("%Y-%m-%d").to_string()}</span>
                                        <span>{item.name.clone()}</span>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </div>
            </Show>
        </div>
    }
}
