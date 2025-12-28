use crate::{
    api::CreateHolidayRequest,
    components::{
        forms::DatePicker,
        layout::{ErrorMessage, LoadingSpinner, SuccessMessage},
    },
    pages::admin::repository::{AdminRepository, HolidayListQuery, HolidayListResult},
    utils::time::now_in_app_tz,
};
use chrono::{Datelike, Duration, NaiveDate};
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
    let holiday_query = create_rw_signal(HolidayListQuery::default());
    let filter_from_input = create_rw_signal(String::new());
    let filter_to_input = create_rw_signal(String::new());
    let calendar_month_input = create_rw_signal(format!(
        "{:04}-{:02}",
        now_in_app_tz().year(),
        now_in_app_tz().month()
    ));

    let repo_for_holidays = repository.clone();
    let holidays_resource = create_resource(
        move || {
            (
                admin_allowed.get(),
                holiday_query.get(),
                holidays_reload.get(),
            )
        },
        move |(allowed, query, _)| {
            let repo = repo_for_holidays.clone();
            async move {
                if !allowed {
                    Ok(HolidayListResult::empty(query.page, query.per_page))
                } else {
                    repo.list_holidays(query).await
                }
            }
        },
    );
    let holidays_loading = holidays_resource.loading();
    let holidays_fetch_error =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.err()));
    let holidays_page =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.ok()));
    let holidays_data = Signal::derive(move || {
        holidays_page
            .get()
            .map(|page| {
                let mut list = page.items.clone();
                list.sort_by_key(|h| h.holiday_date);
                list
            })
            .unwrap_or_default()
    });
    let page_total = Signal::derive(move || {
        holidays_page
            .get()
            .map(|page| (page.page, page.per_page, page.total))
    });
    let total_pages = Signal::derive(move || {
        page_total
            .get()
            .map(|(_, per_page, total)| {
                if total == 0 {
                    1
                } else {
                    ((total + per_page - 1) / per_page).max(1)
                }
            })
            .unwrap_or(1)
    });
    let can_go_prev = Signal::derive(move || {
        page_total
            .get()
            .map(|(page, _, _)| page > 1)
            .unwrap_or(false)
    });
    let can_go_next = Signal::derive(move || {
        page_total
            .get()
            .map(|(page, per_page, total)| {
                let max_page = if total == 0 {
                    1
                } else {
                    ((total + per_page - 1) / per_page).max(1)
                };
                page < max_page
            })
            .unwrap_or(false)
    });
    let page_bounds = Signal::derive(move || {
        page_total.get().map(|(page, per_page, total)| {
            if total == 0 {
                (0, 0, 0)
            } else {
                let start = ((page - 1).max(0) * per_page) + 1;
                let end = (page * per_page).min(total);
                (start, end, total)
            }
        })
    });
    let on_prev_page = {
        move |_| {
            holiday_query.update(|query| {
                if query.page > 1 {
                    query.page -= 1;
                }
            });
        }
    };
    let on_next_page = {
        move |_| {
            if can_go_next.get_untracked() {
                holiday_query.update(|query| query.page += 1);
            }
        }
    };
    let on_per_page_change = {
        move |ev: ev::Event| {
            if let Ok(value) = event_target_value(&ev).parse::<i64>() {
                holiday_query.update(|query| {
                    query.per_page = value.max(1);
                    query.page = 1;
                });
            }
        }
    };
    let on_apply_filters = {
        move |_| {
            let from_raw = filter_from_input.get();
            let to_raw = filter_to_input.get();
            let parse_input = |value: &str, label: &str| -> Result<Option<NaiveDate>, String> {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                    .map(Some)
                    .map_err(|_| format!("{}は YYYY-MM-DD 形式で入力してください。", label))
            };
            let parsed_from = match parse_input(&from_raw, "開始日") {
                Ok(date) => date,
                Err(err) => {
                    holiday_error.set(Some(err));
                    return;
                }
            };
            let parsed_to = match parse_input(&to_raw, "終了日") {
                Ok(date) => date,
                Err(err) => {
                    holiday_error.set(Some(err));
                    return;
                }
            };
            if let (Some(from), Some(to)) = (parsed_from, parsed_to) {
                if from > to {
                    holiday_error.set(Some("開始日は終了日以前である必要があります。".into()));
                    return;
                }
            }
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_query.update(|query| {
                query.page = 1;
                query.from = parsed_from;
                query.to = parsed_to;
            });
        }
    };
    let on_clear_filters = {
        move |_| {
            filter_from_input.set(String::new());
            filter_to_input.set(String::new());
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_query.update(|query| {
                query.page = 1;
                query.from = None;
                query.to = None;
            });
        }
    };
    let on_apply_calendar_range = {
        move |_| {
            let month_raw = calendar_month_input.get();
            let trimmed = month_raw.trim();
            if trimmed.is_empty() {
                holiday_error.set(Some("月を選択してください。".into()));
                return;
            }
            let first_day = match NaiveDate::parse_from_str(&format!("{}-01", trimmed), "%Y-%m-%d")
            {
                Ok(date) => date,
                Err(_) => {
                    holiday_error.set(Some("月は YYYY-MM 形式で入力してください。".into()));
                    return;
                }
            };
            let next_month = if first_day.month() == 12 {
                NaiveDate::from_ymd_opt(first_day.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(first_day.year(), first_day.month() + 1, 1)
            }
            .expect("next month boundary must exist");
            let last_day = next_month - Duration::days(1);
            filter_from_input.set(first_day.format("%Y-%m-%d").to_string());
            filter_to_input.set(last_day.format("%Y-%m-%d").to_string());
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_query.update(|query| {
                query.page = 1;
                query.from = Some(first_day);
                query.to = Some(last_day);
            });
        }
    };

    let repo_for_create = repository.clone();
    let create_holiday_action = create_action(move |payload: &CreateHolidayRequest| {
        let repo = repo_for_create.clone();
        let payload = payload.clone();
        async move { repo.create_holiday(payload).await }
    });
    let create_pending = create_holiday_action.pending();
    {
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
        move |_| {
            let parsed_year = google_year_input.get().trim().parse::<i32>().ok();
            fetch_google_action.dispatch(parsed_year);
        }
    };

    let on_create_holiday = {
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
        move |id: String| {
            deleting_id.set(Some(id.clone()));
            delete_holiday_action.dispatch(id);
        }
    };

    let on_import_google = {
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
            <form class="grid gap-3 lg:grid-cols-3" on:submit=on_create_holiday>
                <DatePicker
                    label=Some("日付")
                    value=holiday_date_input
                />
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"名称"}</label>
                    <input class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| holiday_name_input.set(event_target_value(&ev)) />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">{"備考（任意）"}</label>
                    <input class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| holiday_desc_input.set(event_target_value(&ev)) />
                </div>
                <div class="lg:col-span-3">
                    <button
                        type="submit"
                        class="px-4 py-2 rounded bg-blue-600 text-white disabled:opacity-50"
                        disabled={move || create_pending.get()}
                    >
                        {move || if create_pending.get() { "登録中..." } else { "祝日を登録" }}
                    </button>
                </div>
            </form>
            <div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:gap-4">
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
            <div class="space-y-3 rounded-lg border border-dashed border-gray-200 p-4">
                <div class="flex flex-col gap-1">
                    <h4 class="text-sm font-medium text-gray-900">{"祝日一覧フィルター"}</h4>
                    <p class="text-xs text-gray-500">{"期間を指定すると一致する祝日だけを表示します。"}</p>
                </div>
                <div class="grid gap-3 lg:grid-cols-4">
                    <DatePicker
                        label=Some("開始日")
                        value=filter_from_input
                    />
                    <DatePicker
                        label=Some("終了日")
                        value=filter_to_input
                    />
                    <div class="lg:col-span-2 flex items-end gap-2">
                        <button class="px-3 py-1 rounded bg-gray-800 text-white" on:click=on_apply_filters>
                            {"日付で絞り込み"}
                        </button>
                        <button class="px-3 py-1 rounded border" on:click=on_clear_filters>
                            {"条件クリア"}
                        </button>
                    </div>
                </div>
                <div class="grid gap-3 lg:grid-cols-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600">{"カレンダー範囲 (YYYY-MM)"}</label>
                        <input
                            type="month"
                            class="mt-1 w-full border rounded px-2 py-1"
                            prop:value={move || calendar_month_input.get()}
                            on:input=move |ev| calendar_month_input.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="lg:col-span-2 flex items-end">
                        <button class="px-3 py-1 rounded border border-blue-500 text-blue-600" on:click=on_apply_calendar_range>
                            {"選択月の範囲を適用"}
                        </button>
                    </div>
                </div>
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
            <div class="flex flex-col gap-2 rounded-lg border border-gray-100 p-3 text-sm text-gray-700 lg:flex-row lg:items-center lg:justify-between">
                <div>
                    {move || {
                        page_bounds
                            .get()
                            .map(|bounds| match bounds {
                                (0, 0, 0) => "該当する祝日はありません。".to_string(),
                                (start, end, total) => {
                                    format!("{} 件中 {} - {} 件を表示中", total, start, end)
                                }
                            })
                            .unwrap_or_else(|| "祝日一覧を取得しています...".into())
                    }}
                </div>
                <div class="flex flex-wrap items-center gap-3">
                    <label class="flex items-center gap-1">
                        <span class="text-xs uppercase tracking-wide text-gray-500">
                            {"件数/ページ"}
                        </span>
                        <select
                            class="border rounded px-2 py-1"
                            prop:value={move || holiday_query.get().per_page.to_string()}
                            on:change=on_per_page_change
                        >
                            <option value="10">{"10"}</option>
                            <option value="25">{"25"}</option>
                            <option value="50">{"50"}</option>
                        </select>
                    </label>
                    <div class="inline-flex items-center gap-2">
                        <button
                            class="px-3 py-1 rounded border disabled:opacity-50"
                            disabled={move || holidays_loading.get() || !can_go_prev.get()}
                            on:click=on_prev_page
                        >
                            {"前へ"}
                        </button>
                        <span class="text-xs text-gray-500">
                            {move || {
                                let current = page_total.get().map(|(page, _, _)| page).unwrap_or(1);
                                format!("ページ {}/{}", current, total_pages.get())
                            }}
                        </span>
                        <button
                            class="px-3 py-1 rounded border disabled:opacity-50"
                            disabled={move || holidays_loading.get() || !can_go_next.get()}
                            on:click=on_next_page
                        >
                            {"次へ"}
                        </button>
                    </div>
                </div>
            </div>
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
