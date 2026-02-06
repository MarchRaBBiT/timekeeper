use crate::{
    api::{ApiError, CreateHolidayRequest, HolidayResponse},
    components::{
        error::InlineErrorMessage,
        forms::DatePicker,
        layout::{LoadingSpinner, SuccessMessage},
    },
    pages::admin::repository::{AdminRepository, HolidayListQuery, HolidayListResult},
    utils::time::now_in_app_tz,
};
use chrono::{Datelike, Duration, NaiveDate};
use leptos::{ev, *};
use std::collections::HashSet;

fn compute_total_pages(per_page: i64, total: i64) -> i64 {
    if total == 0 {
        1
    } else {
        ((total + per_page - 1) / per_page).max(1)
    }
}

fn compute_page_bounds(page: i64, per_page: i64, total: i64) -> (i64, i64, i64) {
    if total == 0 {
        (0, 0, 0)
    } else {
        let start = ((page - 1).max(0) * per_page) + 1;
        let end = (page * per_page).min(total);
        (start, end, total)
    }
}

fn parse_optional_filter_date(value: &str, label: &str) -> Result<Option<NaiveDate>, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            ApiError::validation(format!("{}は YYYY-MM-DD 形式で入力してください。", label))
        })
}

fn validate_filter_window(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<(), ApiError> {
    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(ApiError::validation(
                "開始日は終了日以前である必要があります。",
            ));
        }
    }
    Ok(())
}

fn parse_calendar_month_range(month_raw: &str) -> Result<(NaiveDate, NaiveDate), ApiError> {
    let trimmed = month_raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::validation("月を選択してください。"));
    }
    let first_day = NaiveDate::parse_from_str(&format!("{}-01", trimmed), "%Y-%m-%d")
        .map_err(|_| ApiError::validation("月は YYYY-MM 形式で入力してください。"))?;
    let next_month = if first_day.month() == 12 {
        NaiveDate::from_ymd_opt(first_day.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first_day.year(), first_day.month() + 1, 1)
    }
    .expect("next month boundary must exist");
    Ok((first_day, next_month - Duration::days(1)))
}

fn parse_holiday_form(
    date_raw: &str,
    name_raw: &str,
    desc_raw: &str,
) -> Result<CreateHolidayRequest, ApiError> {
    if date_raw.trim().is_empty() || name_raw.trim().is_empty() {
        return Err(ApiError::validation("日付と名称を入力してください。"));
    }
    let parsed_date = NaiveDate::parse_from_str(date_raw.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::validation("日付は YYYY-MM-DD 形式で入力してください。"))?;
    Ok(CreateHolidayRequest {
        holiday_date: parsed_date,
        name: name_raw.trim().to_string(),
        description: if desc_raw.trim().is_empty() {
            None
        } else {
            Some(desc_raw.trim().to_string())
        },
    })
}

fn parse_google_year(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

fn filter_new_google_holidays(
    existing_dates: impl IntoIterator<Item = NaiveDate>,
    candidates: Vec<CreateHolidayRequest>,
) -> Vec<CreateHolidayRequest> {
    let existing: HashSet<NaiveDate> = existing_dates.into_iter().collect();
    candidates
        .into_iter()
        .filter(|candidate| !existing.contains(&candidate.holiday_date))
        .collect()
}

fn parse_per_page_value(raw: &str) -> Option<i64> {
    raw.parse::<i64>().ok().map(|value| value.max(1))
}

fn parse_filter_inputs(
    from_raw: &str,
    to_raw: &str,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), ApiError> {
    let parsed_from = parse_optional_filter_date(from_raw, "開始日")?;
    let parsed_to = parse_optional_filter_date(to_raw, "終了日")?;
    validate_filter_window(parsed_from, parsed_to)?;
    Ok((parsed_from, parsed_to))
}

fn import_result_message(count: usize) -> String {
    if count == 0 {
        "追加対象の祝日はありません。".into()
    } else {
        format!("{} 件の祝日を追加しました。", count)
    }
}

fn page_bounds_message(bounds: Option<(i64, i64, i64)>) -> String {
    bounds
        .map(|(start, end, total)| match (start, end, total) {
            (0, 0, 0) => "該当する祝日はありません。".to_string(),
            _ => format!("{} 件中 {} - {} 件を表示中", total, start, end),
        })
        .unwrap_or_else(|| "祝日一覧を取得しています...".into())
}

fn create_holiday_feedback(
    result: Result<crate::api::HolidayResponse, ApiError>,
) -> (Option<String>, Option<ApiError>, bool) {
    match result {
        Ok(created) => (
            Some(format!(
                "{} ({}) を登録しました。",
                created.name,
                created.holiday_date.format("%Y-%m-%d")
            )),
            None,
            true,
        ),
        Err(err) => (None, Some(err), false),
    }
}

fn delete_holiday_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some("祝日を削除しました。".into()), None),
        Err(err) => (None, Some(err)),
    }
}

fn import_holidays_feedback(result: Result<usize, ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(count) => (Some(import_result_message(count)), None),
        Err(err) => (None, Some(err)),
    }
}

fn prepare_import_candidates(
    existing_dates: impl IntoIterator<Item = NaiveDate>,
    google_holidays: Vec<CreateHolidayRequest>,
) -> Result<Vec<CreateHolidayRequest>, &'static str> {
    let candidates = filter_new_google_holidays(existing_dates, google_holidays);
    if candidates.is_empty() {
        Err("追加対象の祝日はありません。")
    } else {
        Ok(candidates)
    }
}

fn build_per_page_query_update(current: &HolidayListQuery, raw: &str) -> Option<HolidayListQuery> {
    let value = parse_per_page_value(raw)?;
    let mut next = current.clone();
    next.per_page = value;
    next.page = 1;
    Some(next)
}

fn build_filter_query_update(
    current: &HolidayListQuery,
    from_raw: &str,
    to_raw: &str,
) -> Result<HolidayListQuery, ApiError> {
    let (parsed_from, parsed_to) = parse_filter_inputs(from_raw, to_raw)?;
    let mut next = current.clone();
    next.page = 1;
    next.from = parsed_from;
    next.to = parsed_to;
    Ok(next)
}

fn clear_filter_query(current: &HolidayListQuery) -> HolidayListQuery {
    let mut next = current.clone();
    next.page = 1;
    next.from = None;
    next.to = None;
    next
}

fn build_calendar_range_query_update(
    current: &HolidayListQuery,
    month_raw: &str,
) -> Result<(HolidayListQuery, String, String), ApiError> {
    let (first_day, last_day) = parse_calendar_month_range(month_raw)?;
    let mut next = current.clone();
    next.page = 1;
    next.from = Some(first_day);
    next.to = Some(last_day);
    Ok((
        next,
        first_day.format("%Y-%m-%d").to_string(),
        last_day.format("%Y-%m-%d").to_string(),
    ))
}

fn google_fetch_feedback(
    result: Result<Vec<CreateHolidayRequest>, ApiError>,
) -> (Vec<CreateHolidayRequest>, Option<ApiError>) {
    match result {
        Ok(list) => (list, None),
        Err(err) => (Vec::new(), Some(err)),
    }
}

fn bump_reload(holidays_reload: RwSignal<u32>) {
    holidays_reload.update(|value| *value = value.wrapping_add(1));
}

async fn load_holiday_page(
    repository: AdminRepository,
    allowed: bool,
    query: HolidayListQuery,
) -> Result<HolidayListResult, ApiError> {
    if !allowed {
        Ok(HolidayListResult::empty(query.page, query.per_page))
    } else {
        repository.list_holidays(query).await
    }
}

fn sorted_holiday_items(page: Option<HolidayListResult>) -> Vec<HolidayResponse> {
    page.map(|page| {
        let mut list = page.items.clone();
        list.sort_by_key(|holiday| holiday.holiday_date);
        list
    })
    .unwrap_or_default()
}

fn can_go_next_from_page_total(page_total: Option<(i64, i64, i64)>) -> bool {
    page_total
        .map(|(page, per_page, total)| page < compute_total_pages(per_page, total))
        .unwrap_or(false)
}

fn apply_prev_page(query: &mut HolidayListQuery) {
    if query.page > 1 {
        query.page -= 1;
    }
}

fn apply_next_page(query: &mut HolidayListQuery, can_go_next: bool) {
    if can_go_next {
        query.page += 1;
    }
}

fn apply_per_page_change_signal(holiday_query: RwSignal<HolidayListQuery>, raw: &str) {
    if let Some(next_query) = build_per_page_query_update(&holiday_query.get_untracked(), raw) {
        holiday_query.set(next_query);
    }
}

fn apply_filters_signal(
    holiday_query: RwSignal<HolidayListQuery>,
    filter_from_input: RwSignal<String>,
    filter_to_input: RwSignal<String>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
) {
    let from_raw = filter_from_input.get();
    let to_raw = filter_to_input.get();
    match build_filter_query_update(&holiday_query.get_untracked(), &from_raw, &to_raw) {
        Ok(next_query) => {
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_query.set(next_query);
        }
        Err(err) => {
            holiday_error.set(Some(err));
        }
    }
}

fn clear_filters_signal(
    holiday_query: RwSignal<HolidayListQuery>,
    filter_from_input: RwSignal<String>,
    filter_to_input: RwSignal<String>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
) {
    filter_from_input.set(String::new());
    filter_to_input.set(String::new());
    holiday_error.set(None);
    holiday_message.set(None);
    holiday_query.set(clear_filter_query(&holiday_query.get_untracked()));
}

fn apply_calendar_range_signal(
    holiday_query: RwSignal<HolidayListQuery>,
    calendar_month_input: RwSignal<String>,
    filter_from_input: RwSignal<String>,
    filter_to_input: RwSignal<String>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
) {
    match build_calendar_range_query_update(
        &holiday_query.get_untracked(),
        &calendar_month_input.get(),
    ) {
        Ok((next_query, from_input, to_input)) => {
            filter_from_input.set(from_input);
            filter_to_input.set(to_input);
            holiday_error.set(None);
            holiday_message.set(None);
            holiday_query.set(next_query);
        }
        Err(err) => {
            holiday_error.set(Some(err));
        }
    }
}

fn apply_create_effect_signal(
    result: Result<HolidayResponse, ApiError>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
    holiday_date_input: RwSignal<String>,
    holiday_name_input: RwSignal<String>,
    holiday_desc_input: RwSignal<String>,
    holidays_reload: RwSignal<u32>,
) {
    let (message, error, should_reload) = create_holiday_feedback(result);
    holiday_message.set(message);
    holiday_error.set(error);
    if should_reload {
        holiday_date_input.set(String::new());
        holiday_name_input.set(String::new());
        holiday_desc_input.set(String::new());
        bump_reload(holidays_reload);
    }
}

fn apply_delete_effect_signal(
    result: Result<(), ApiError>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
    deleting_id: RwSignal<Option<String>>,
    holidays_reload: RwSignal<u32>,
) {
    let (message, error) = delete_holiday_feedback(result);
    let should_reload = error.is_none();
    holiday_message.set(message);
    holiday_error.set(error);
    deleting_id.set(None);
    if should_reload {
        bump_reload(holidays_reload);
    }
}

fn apply_google_fetch_effect_signal(
    result: Result<Vec<CreateHolidayRequest>, ApiError>,
    google_holidays: RwSignal<Vec<CreateHolidayRequest>>,
    google_error: RwSignal<Option<ApiError>>,
) {
    let (list, error) = google_fetch_feedback(result);
    google_error.set(error);
    google_holidays.set(list);
}

fn apply_import_effect_signal(
    result: Result<usize, ApiError>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
    holidays_reload: RwSignal<u32>,
) {
    let (message, error) = import_holidays_feedback(result);
    let should_reload = error.is_none();
    holiday_message.set(message);
    holiday_error.set(error);
    if should_reload {
        bump_reload(holidays_reload);
    }
}

fn dispatch_google_fetch(
    fetch_google_action: Action<Option<i32>, Result<Vec<CreateHolidayRequest>, ApiError>>,
    year_input: &str,
) {
    fetch_google_action.dispatch(parse_google_year(year_input));
}

fn resolve_create_payload(
    date_raw: &str,
    name_raw: &str,
    desc_raw: &str,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
) -> Option<CreateHolidayRequest> {
    match parse_holiday_form(date_raw, name_raw, desc_raw) {
        Ok(payload) => {
            holiday_error.set(None);
            holiday_message.set(None);
            Some(payload)
        }
        Err(err) => {
            holiday_error.set(Some(err));
            holiday_message.set(None);
            None
        }
    }
}

fn dispatch_delete_holiday(
    id: String,
    deleting_id: RwSignal<Option<String>>,
    delete_holiday_action: Action<String, Result<(), ApiError>>,
) {
    deleting_id.set(Some(id.clone()));
    delete_holiday_action.dispatch(id);
}

fn resolve_import_payload(
    existing_dates: impl IntoIterator<Item = NaiveDate>,
    google_holidays: Vec<CreateHolidayRequest>,
    holiday_message: RwSignal<Option<String>>,
    holiday_error: RwSignal<Option<ApiError>>,
) -> Option<Vec<CreateHolidayRequest>> {
    match prepare_import_candidates(existing_dates, google_holidays) {
        Ok(candidates) => {
            holiday_error.set(None);
            holiday_message.set(None);
            Some(candidates)
        }
        Err(message) => {
            holiday_message.set(Some(message.into()));
            holiday_error.set(None);
            None
        }
    }
}

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
    let holiday_error = create_rw_signal(None::<ApiError>);
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
            async move { load_holiday_page(repo, allowed, query).await }
        },
    );
    let holidays_loading = holidays_resource.loading();
    let holidays_fetch_error =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.err()));
    let holidays_page =
        Signal::derive(move || holidays_resource.get().and_then(|result| result.ok()));
    let holidays_data = Signal::derive(move || sorted_holiday_items(holidays_page.get()));
    let page_total = Signal::derive(move || {
        holidays_page
            .get()
            .map(|page| (page.page, page.per_page, page.total))
    });
    let total_pages = Signal::derive(move || {
        page_total
            .get()
            .map(|(_, per_page, total)| compute_total_pages(per_page, total))
            .unwrap_or(1)
    });
    let can_go_prev = Signal::derive(move || {
        page_total
            .get()
            .map(|(page, _, _)| page > 1)
            .unwrap_or(false)
    });
    let can_go_next = Signal::derive(move || can_go_next_from_page_total(page_total.get()));
    let page_bounds = Signal::derive(move || {
        page_total
            .get()
            .map(|(page, per_page, total)| compute_page_bounds(page, per_page, total))
    });
    let on_prev_page = { move |_| holiday_query.update(apply_prev_page) };
    let on_next_page = {
        move |_| holiday_query.update(|query| apply_next_page(query, can_go_next.get_untracked()))
    };
    let on_per_page_change = {
        move |ev: ev::Event| apply_per_page_change_signal(holiday_query, &event_target_value(&ev))
    };
    let on_apply_filters = {
        move |_| {
            apply_filters_signal(
                holiday_query,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            )
        }
    };
    let on_clear_filters = {
        move |_| {
            clear_filters_signal(
                holiday_query,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            )
        }
    };
    let on_apply_calendar_range = {
        move |_| {
            apply_calendar_range_signal(
                holiday_query,
                calendar_month_input,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            )
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
                apply_create_effect_signal(
                    result,
                    holiday_message,
                    holiday_error,
                    holiday_date_input,
                    holiday_name_input,
                    holiday_desc_input,
                    holidays_reload,
                );
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
                apply_delete_effect_signal(
                    result,
                    holiday_message,
                    holiday_error,
                    deleting_id,
                    holidays_reload,
                );
            }
        });
    }

    let google_year_input = create_rw_signal(now_in_app_tz().year().to_string());
    let google_holidays = create_rw_signal(Vec::<CreateHolidayRequest>::new());
    let google_error = create_rw_signal(None::<ApiError>);
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
                apply_google_fetch_effect_signal(result, google_holidays, google_error);
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
                apply_import_effect_signal(result, holiday_message, holiday_error, holidays_reload);
            }
        });
    }

    let on_fetch_google =
        { move |_| dispatch_google_fetch(fetch_google_action, &google_year_input.get()) };

    let on_create_holiday = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            if let Some(payload) = resolve_create_payload(
                &holiday_date_input.get(),
                &holiday_name_input.get(),
                &holiday_desc_input.get(),
                holiday_message,
                holiday_error,
            ) {
                create_holiday_action.dispatch(payload);
            }
        }
    };

    let on_delete_holiday =
        { move |id: String| dispatch_delete_holiday(id, deleting_id, delete_holiday_action) };

    let on_import_google = {
        move |_| {
            let existing_dates = holidays_data.get().into_iter().map(|h| h.holiday_date);
            if let Some(candidates) = resolve_import_payload(
                existing_dates,
                google_holidays.get(),
                holiday_message,
                holiday_error,
            ) {
                import_action.dispatch(candidates);
            }
        }
    };

    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-fg">{"祝日管理"}</h3>
            <form class="grid gap-3 lg:grid-cols-3" on:submit=on_create_holiday>
                <DatePicker
                    label=Some("日付")
                    value=holiday_date_input
                />
                <div>
                    <label class="block text-sm font-bold text-fg-muted ml-1 mb-1.5">{"名称"}</label>
                    <input class="w-full rounded-xl border-2 border-form-control-border bg-form-control-bg text-fg py-2.5 px-4 shadow-sm focus:outline-none focus:border-action-primary-border-hover focus:ring-4 focus:ring-action-primary-focus transition-all duration-200" on:input=move |ev| holiday_name_input.set(event_target_value(&ev)) />
                </div>
                <div>
                    <label class="block text-sm font-bold text-fg-muted ml-1 mb-1.5">{"備考（任意）"}</label>
                    <input class="w-full rounded-xl border-2 border-form-control-border bg-form-control-bg text-fg py-2.5 px-4 shadow-sm focus:outline-none focus:border-action-primary-border-hover focus:ring-4 focus:ring-action-primary-focus transition-all duration-200" on:input=move |ev| holiday_desc_input.set(event_target_value(&ev)) />
                </div>
                <div class="lg:col-span-3">
                    <button
                        type="submit"
                        class="px-4 py-2 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
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
                        class="w-32 rounded-xl border-2 border-form-control-border bg-form-control-bg text-fg py-2.5 px-4 shadow-sm focus:outline-none focus:border-action-primary-border-hover focus:ring-4 focus:ring-action-primary-focus transition-all duration-200"
                        prop:value={move || google_year_input.get()}
                        on:input=move |ev| google_year_input.set(event_target_value(&ev))
                    />
                    <button
                        class="px-3 py-2.5 rounded-xl border-2 border-border text-fg hover:bg-action-ghost-bg-hover disabled:opacity-50 font-medium transition-colors"
                        disabled={move || google_loading.get()}
                        on:click=on_fetch_google
                    >
                        {move || if google_loading.get() { "取得中..." } else { "Google 祝日取得" }}
                    </button>
                </div>
                <button
                    class="px-3 py-2.5 rounded-xl border-2 border-status-success-border text-status-success-text bg-status-success-bg disabled:opacity-50 font-bold transition-colors"
                    disabled={move || google_holidays.get().is_empty()}
                    on:click=on_import_google
                >
                    {"一覧から登録"}
                </button>
            </div>
            <div class="space-y-3 rounded-lg border border-dashed border-border p-4">
                <div class="flex flex-col gap-1">
                    <h4 class="text-sm font-medium text-fg">{"祝日一覧フィルター"}</h4>
                    <p class="text-xs text-fg-muted">{"期間を指定すると一致する祝日だけを表示します。"}</p>
                </div>
                <div class="grid gap-3 lg:grid-cols-4 align-bottom">
                    <DatePicker
                        label=Some("開始日")
                        value=filter_from_input
                    />
                    <DatePicker
                        label=Some("終了日")
                        value=filter_to_input
                    />
                    <div class="lg:col-span-2 flex items-end gap-2 mb-0.5">
                        <button class="h-[50px] px-4 rounded-xl border-2 border-border text-fg hover:bg-action-ghost-bg-hover font-medium transition-colors" on:click=on_apply_filters>
                            {"日付で絞り込み"}
                        </button>
                        <button class="h-[50px] px-4 rounded-xl text-fg-muted hover:text-fg font-medium transition-colors" on:click=on_clear_filters>
                            {"条件クリア"}
                        </button>
                    </div>
                </div>
                <div class="grid gap-3 lg:grid-cols-3">
                    <div>
                        <label class="block text-sm font-bold text-fg-muted ml-1 mb-1.5">{"カレンダー範囲 (YYYY-MM)"}</label>
                        <input
                            type="month"
                            class="w-full rounded-xl border-2 border-form-control-border bg-form-control-bg text-fg py-2.5 px-4 shadow-sm focus:outline-none focus:border-action-primary-border-hover focus:ring-4 focus:ring-action-primary-focus transition-all duration-200"
                            prop:value={move || calendar_month_input.get()}
                            on:input=move |ev| calendar_month_input.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="lg:col-span-2 flex items-end mb-0.5">
                        <button class="h-[50px] px-4 rounded-xl border-2 border-border text-fg hover:bg-action-ghost-bg-hover font-medium transition-colors" on:click=on_apply_calendar_range>
                            {"選択月の範囲を適用"}
                        </button>
                    </div>
                </div>
            </div>
            <Show when=move || holiday_error.get().is_some()>
                <InlineErrorMessage error={holiday_error.into()} />
            </Show>
            <Show when=move || holiday_message.get().is_some()>
                <SuccessMessage message={holiday_message.get().unwrap_or_default()} />
            </Show>
            <Show when=move || holidays_fetch_error.get().is_some()>
                <InlineErrorMessage error={holidays_fetch_error} />
            </Show>
            <Show when=move || google_error.get().is_some()>
                <InlineErrorMessage error={google_error.into()} />
            </Show>
            <Show when=move || holidays_loading.get()>
                <div class="flex items-center gap-2 text-sm text-fg-muted">
                    <LoadingSpinner />
                    <span>{"祝日一覧を読み込み中..."}</span>
                </div>
            </Show>
            <div class="flex flex-col gap-2 rounded-lg border border-border p-3 text-sm text-fg lg:flex-row lg:items-center lg:justify-between">
                <div>
                    {move || page_bounds_message(page_bounds.get())}
                </div>
                <div class="flex flex-wrap items-center gap-3">
                    <label class="flex items-center gap-1">
                        <span class="text-xs uppercase tracking-wide text-fg-muted">
                            {"件数/ページ"}
                        </span>
                        <select
                            class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
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
                            class="px-3 py-1 rounded border border-border text-fg disabled:opacity-50"
                            disabled={move || holidays_loading.get() || !can_go_prev.get()}
                            on:click=on_prev_page
                        >
                            {"前へ"}
                        </button>
                        <span class="text-xs text-fg-muted">
                            {move || {
                                let current = page_total.get().map(|(page, _, _)| page).unwrap_or(1);
                                format!("ページ {}/{}", current, total_pages.get())
                            }}
                        </span>
                        <button
                            class="px-3 py-1 rounded border border-border text-fg disabled:opacity-50"
                            disabled={move || holidays_loading.get() || !can_go_next.get()}
                            on:click=on_next_page
                        >
                            {"次へ"}
                        </button>
                    </div>
                </div>
            </div>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-border text-sm">
                    <thead class="bg-surface-muted">
                        <tr>
                            <th class="px-4 py-2 text-left text-fg-muted">{"日付"}</th>
                            <th class="px-4 py-2 text-left text-fg-muted">{"名称"}</th>
                            <th class="px-4 py-2 text-left text-fg-muted">{"備考"}</th>
                            <th class="px-4 py-2 text-right text-fg-muted">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border">
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
                                        <td class="px-4 py-2 text-fg">{item.holiday_date.format("%Y-%m-%d").to_string()}</td>
                                        <td class="px-4 py-2 text-fg">{item.name.clone()}</td>
                                        <td class="px-4 py-2 text-fg-muted">{item.description.clone().unwrap_or_default()}</td>
                                        <td class="px-4 py-2 text-right">
                                            <button
                                                class="px-3 py-1 rounded border border-border text-sm text-fg disabled:opacity-50"
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
                <div class="border border-border rounded-lg p-4 space-y-2">
                    <h4 class="text-sm font-medium text-fg">{"Google 祝日候補"}</h4>
                    <ul class="space-y-1 text-sm text-fg">
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::{ApiClient, HolidayResponse};
    use crate::test_support::ssr::{
        render_to_string, with_local_runtime, with_local_runtime_async, with_runtime,
    };
    use chrono::Datelike;

    fn repo(server: &MockServer) -> AdminRepository {
        AdminRepository::new_with_client(std::rc::Rc::new(ApiClient::new_with_base_url(
            &server.url("/api"),
        )))
    }

    fn sample_holiday(id: &str, name: &str, year: i32, month: u32, day: u32) -> HolidayResponse {
        HolidayResponse {
            id: id.to_string(),
            holiday_date: NaiveDate::from_ymd_opt(year, month, day).expect("valid"),
            name: name.to_string(),
            description: None,
        }
    }

    #[test]
    fn holiday_management_section_renders() {
        let html = render_to_string(move || {
            let repo = AdminRepository::new_with_client(std::rc::Rc::new(ApiClient::new()));
            let allowed = create_memo(|_| false);
            view! { <HolidayManagementSection repository=repo admin_allowed=allowed /> }
        });
        assert!(html.contains("祝日管理"));
    }

    #[test]
    fn pagination_helpers_cover_zero_and_non_zero_totals() {
        assert_eq!(compute_total_pages(10, 0), 1);
        assert_eq!(compute_total_pages(10, 21), 3);
        assert_eq!(compute_page_bounds(1, 10, 0), (0, 0, 0));
        assert_eq!(compute_page_bounds(2, 10, 35), (11, 20, 35));
        assert_eq!(compute_page_bounds(4, 10, 35), (31, 35, 35));
    }

    #[test]
    fn filter_date_parsing_and_window_validation() {
        assert_eq!(
            parse_optional_filter_date("   ", "開始日").expect("empty is none"),
            None
        );
        let from = parse_optional_filter_date("2026-01-01", "開始日").expect("from");
        let to = parse_optional_filter_date("2026-01-31", "終了日").expect("to");
        assert!(validate_filter_window(from, to).is_ok());
        assert!(parse_optional_filter_date("bad", "開始日").is_err());
        let from = Some(NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid"));
        let to = Some(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"));
        assert!(validate_filter_window(from, to).is_err());

        let from_only = Some(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"));
        assert!(validate_filter_window(from_only, None).is_ok());
        let to_only = Some(NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid"));
        assert!(validate_filter_window(None, to_only).is_ok());
    }

    #[test]
    fn calendar_month_range_and_google_year_parsing() {
        let (first, last) = parse_calendar_month_range("2026-02").expect("month range");
        assert_eq!(first.to_string(), "2026-02-01");
        assert_eq!(last.to_string(), "2026-02-28");

        let (first_leap, last_leap) = parse_calendar_month_range("2024-02").expect("leap range");
        assert_eq!(first_leap.to_string(), "2024-02-01");
        assert_eq!(last_leap.to_string(), "2024-02-29");

        let (first_dec, last_dec) = parse_calendar_month_range("2025-12").expect("dec range");
        assert_eq!(first_dec.month(), 12);
        assert_eq!(last_dec.to_string(), "2025-12-31");

        assert!(parse_calendar_month_range("").is_err());
        assert!(parse_calendar_month_range("2026/01").is_err());

        assert_eq!(parse_google_year(" 2026 "), Some(2026));
        assert_eq!(parse_google_year(""), None);
        assert_eq!(parse_google_year("invalid"), None);
    }

    #[test]
    fn holiday_form_and_google_candidate_filtering() {
        let payload =
            parse_holiday_form("2026-01-02", "  New Year  ", "  optional  ").expect("payload");
        assert_eq!(payload.holiday_date.to_string(), "2026-01-02");
        assert_eq!(payload.name, "New Year");
        assert_eq!(payload.description.as_deref(), Some("optional"));

        let no_desc = parse_holiday_form("2026-01-03", "Holiday", " ").expect("payload");
        assert!(no_desc.description.is_none());
        assert!(parse_holiday_form("", "Holiday", "").is_err());
        assert!(parse_holiday_form("bad-date", "Holiday", "").is_err());

        let existing = vec![NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid")];
        let filtered = filter_new_google_holidays(
            existing,
            vec![
                CreateHolidayRequest {
                    holiday_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"),
                    name: "Existing".into(),
                    description: None,
                },
                CreateHolidayRequest {
                    holiday_date: NaiveDate::from_ymd_opt(2026, 1, 2).expect("valid"),
                    name: "New".into(),
                    description: None,
                },
            ],
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "New");
    }

    #[test]
    fn filter_new_google_holidays_keeps_duplicate_candidates_when_not_existing() {
        let existing = vec![NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid")];
        let duplicate_date = NaiveDate::from_ymd_opt(2026, 1, 2).expect("valid");
        let filtered = filter_new_google_holidays(
            existing,
            vec![
                CreateHolidayRequest {
                    holiday_date: duplicate_date,
                    name: "A".into(),
                    description: None,
                },
                CreateHolidayRequest {
                    holiday_date: duplicate_date,
                    name: "B".into(),
                    description: None,
                },
            ],
        );
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "A");
        assert_eq!(filtered[1].name, "B");
    }

    #[test]
    fn helper_parses_per_page_and_filter_inputs() {
        assert_eq!(parse_per_page_value("25"), Some(25));
        assert_eq!(parse_per_page_value("0"), Some(1));
        assert_eq!(parse_per_page_value("-10"), Some(1));
        assert_eq!(parse_per_page_value("bad"), None);

        let parsed = parse_filter_inputs("2026-01-01", "2026-01-31").expect("valid");
        assert_eq!(parsed.0.expect("from").to_string(), "2026-01-01");
        assert_eq!(parsed.1.expect("to").to_string(), "2026-01-31");
        assert!(parse_filter_inputs("bad", "2026-01-31").is_err());
        assert!(parse_filter_inputs("2026-01-31", "2026-01-01").is_err());
    }

    #[test]
    fn helper_import_and_page_bounds_messages_cover_edges() {
        assert_eq!(import_result_message(0), "追加対象の祝日はありません。");
        assert_eq!(import_result_message(3), "3 件の祝日を追加しました。");

        assert_eq!(page_bounds_message(None), "祝日一覧を取得しています...");
        assert_eq!(
            page_bounds_message(Some((0, 0, 0))),
            "該当する祝日はありません。"
        );
        assert_eq!(
            page_bounds_message(Some((11, 20, 35))),
            "35 件中 11 - 20 件を表示中"
        );
    }

    #[test]
    fn helper_feedback_and_import_candidate_selection_cover_branches() {
        let created = crate::api::HolidayResponse {
            id: "h1".into(),
            holiday_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"),
            name: "New Year".into(),
            description: None,
        };
        let (create_ok_msg, create_ok_err, create_ok_reload) = create_holiday_feedback(Ok(created));
        assert!(create_ok_msg.expect("msg").contains("New Year"));
        assert!(create_ok_err.is_none());
        assert!(create_ok_reload);

        let (create_err_msg, create_err, create_err_reload) =
            create_holiday_feedback(Err(ApiError::unknown("create failed")));
        assert!(create_err_msg.is_none());
        assert_eq!(create_err.expect("error").error, "create failed");
        assert!(!create_err_reload);

        let (delete_ok_msg, delete_ok_err) = delete_holiday_feedback(Ok(()));
        assert_eq!(delete_ok_msg.as_deref(), Some("祝日を削除しました。"));
        assert!(delete_ok_err.is_none());

        let (delete_err_msg, delete_err) =
            delete_holiday_feedback(Err(ApiError::unknown("delete failed")));
        assert!(delete_err_msg.is_none());
        assert_eq!(delete_err.expect("error").error, "delete failed");

        let (import_ok_msg, import_ok_err) = import_holidays_feedback(Ok(2));
        assert_eq!(import_ok_msg.as_deref(), Some("2 件の祝日を追加しました。"));
        assert!(import_ok_err.is_none());

        let (import_err_msg, import_err) =
            import_holidays_feedback(Err(ApiError::unknown("import failed")));
        assert!(import_err_msg.is_none());
        assert_eq!(import_err.expect("error").error, "import failed");

        let existing = vec![NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid")];
        let candidates = prepare_import_candidates(
            existing.clone(),
            vec![CreateHolidayRequest {
                holiday_date: NaiveDate::from_ymd_opt(2026, 1, 2).expect("valid"),
                name: "new".into(),
                description: None,
            }],
        )
        .expect("non-empty");
        assert_eq!(candidates.len(), 1);

        assert!(prepare_import_candidates(
            existing,
            vec![CreateHolidayRequest {
                holiday_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"),
                name: "existing".into(),
                description: None,
            }],
        )
        .is_err());
    }

    #[test]
    fn helper_query_update_builders_cover_success_and_error_paths() {
        let current = HolidayListQuery {
            page: 3,
            per_page: 10,
            from: Some(NaiveDate::from_ymd_opt(2025, 12, 1).expect("valid")),
            to: Some(NaiveDate::from_ymd_opt(2025, 12, 31).expect("valid")),
        };

        let per_page = build_per_page_query_update(&current, "25").expect("valid per-page");
        assert_eq!(per_page.page, 1);
        assert_eq!(per_page.per_page, 25);
        assert_eq!(per_page.from, current.from);
        assert_eq!(per_page.to, current.to);
        assert!(build_per_page_query_update(&current, "bad").is_none());

        let per_page_min = build_per_page_query_update(&current, "0").expect("min per-page");
        assert_eq!(per_page_min.per_page, 1);

        let filtered =
            build_filter_query_update(&current, "2026-01-01", "2026-01-31").expect("valid filters");
        assert_eq!(filtered.page, 1);
        assert_eq!(
            filtered.from,
            Some(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"))
        );
        assert_eq!(
            filtered.to,
            Some(NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid"))
        );
        assert!(build_filter_query_update(&current, "bad-date", "2026-01-31").is_err());

        let cleared = clear_filter_query(&current);
        assert_eq!(cleared.page, 1);
        assert_eq!(cleared.from, None);
        assert_eq!(cleared.to, None);

        let (calendar_query, from_input, to_input) =
            build_calendar_range_query_update(&current, "2024-02").expect("calendar update");
        assert_eq!(
            calendar_query.from,
            Some(NaiveDate::from_ymd_opt(2024, 2, 1).expect("valid"))
        );
        assert_eq!(
            calendar_query.to,
            Some(NaiveDate::from_ymd_opt(2024, 2, 29).expect("valid"))
        );
        assert_eq!(from_input, "2024-02-01");
        assert_eq!(to_input, "2024-02-29");
        assert_eq!(calendar_query.page, 1);
        assert!(build_calendar_range_query_update(&current, "").is_err());
    }

    #[test]
    fn helper_google_fetch_feedback_maps_success_and_error() {
        let holiday = CreateHolidayRequest {
            holiday_date: NaiveDate::from_ymd_opt(2026, 7, 20).expect("valid"),
            name: "Marine Day".into(),
            description: None,
        };

        let (ok_list, ok_error) = google_fetch_feedback(Ok(vec![holiday.clone()]));
        assert_eq!(ok_list.len(), 1);
        assert_eq!(ok_list[0].name, holiday.name);
        assert!(ok_error.is_none());

        let (err_list, err) = google_fetch_feedback(Err(ApiError::unknown("google failed")));
        assert!(err_list.is_empty());
        assert_eq!(err.expect("error").error, "google failed");
    }

    #[test]
    fn helper_signal_update_paths_cover_filters_and_pagination() {
        with_runtime(|| {
            let holiday_query = create_rw_signal(HolidayListQuery {
                page: 3,
                per_page: 10,
                from: None,
                to: None,
            });
            let filter_from_input = create_rw_signal("2026-01-01".to_string());
            let filter_to_input = create_rw_signal("2026-01-31".to_string());
            let calendar_month_input = create_rw_signal("2026-02".to_string());
            let holiday_message = create_rw_signal(Some("old-message".to_string()));
            let holiday_error = create_rw_signal(Some(ApiError::unknown("old-error")));

            apply_filters_signal(
                holiday_query,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            );
            assert_eq!(holiday_query.get().page, 1);
            assert!(holiday_message.get().is_none());
            assert!(holiday_error.get().is_none());

            filter_from_input.set("bad-date".to_string());
            apply_filters_signal(
                holiday_query,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );

            clear_filters_signal(
                holiday_query,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            );
            assert!(filter_from_input.get().is_empty());
            assert!(filter_to_input.get().is_empty());
            assert_eq!(holiday_query.get().from, None);
            assert_eq!(holiday_query.get().to, None);

            apply_calendar_range_signal(
                holiday_query,
                calendar_month_input,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            );
            assert_eq!(filter_from_input.get(), "2026-02-01");
            assert_eq!(filter_to_input.get(), "2026-02-28");
            assert_eq!(
                holiday_query.get().from,
                Some(NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid"))
            );

            calendar_month_input.set("invalid".to_string());
            apply_calendar_range_signal(
                holiday_query,
                calendar_month_input,
                filter_from_input,
                filter_to_input,
                holiday_message,
                holiday_error,
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );

            apply_per_page_change_signal(holiday_query, "25");
            assert_eq!(holiday_query.get().per_page, 25);
            apply_per_page_change_signal(holiday_query, "invalid");
            assert_eq!(holiday_query.get().per_page, 25);

            let mut query = HolidayListQuery::default();
            apply_prev_page(&mut query);
            assert_eq!(query.page, 1);
            apply_next_page(&mut query, false);
            assert_eq!(query.page, 1);
            apply_next_page(&mut query, true);
            assert_eq!(query.page, 2);

            let sorted = sorted_holiday_items(Some(HolidayListResult {
                page: 1,
                per_page: 10,
                total: 2,
                items: vec![
                    sample_holiday("h2", "Later", 2026, 1, 2),
                    sample_holiday("h1", "Earlier", 2026, 1, 1),
                ],
            }));
            assert_eq!(sorted[0].name, "Earlier");
            assert_eq!(sorted[1].name, "Later");

            assert!(can_go_next_from_page_total(Some((1, 10, 21))));
            assert!(!can_go_next_from_page_total(Some((3, 10, 21))));
            assert!(!can_go_next_from_page_total(None));
        });
    }

    #[test]
    fn helper_effect_and_dispatch_paths_cover_state_updates() {
        with_local_runtime(|| {
            let runtime = leptos::create_runtime();
            let holiday_message = create_rw_signal(None::<String>);
            let holiday_error = create_rw_signal(None::<ApiError>);
            let holiday_date_input = create_rw_signal("2026-01-01".to_string());
            let holiday_name_input = create_rw_signal("Name".to_string());
            let holiday_desc_input = create_rw_signal("Memo".to_string());
            let holidays_reload = create_rw_signal(0u32);
            let deleting_id = create_rw_signal(Some("h1".to_string()));
            let google_holidays = create_rw_signal(Vec::<CreateHolidayRequest>::new());
            let google_error = create_rw_signal(None::<ApiError>);

            let created = sample_holiday("h1", "New Year", 2026, 1, 1);
            apply_create_effect_signal(
                Ok(created),
                holiday_message,
                holiday_error,
                holiday_date_input,
                holiday_name_input,
                holiday_desc_input,
                holidays_reload,
            );
            assert!(holiday_message
                .get()
                .as_deref()
                .unwrap_or_default()
                .contains("New Year"));
            assert!(holiday_error.get().is_none());
            assert!(holiday_date_input.get().is_empty());
            assert!(holiday_name_input.get().is_empty());
            assert!(holiday_desc_input.get().is_empty());
            assert_eq!(holidays_reload.get(), 1);

            holiday_date_input.set("2026-01-02".to_string());
            holiday_name_input.set("Keep".to_string());
            holiday_desc_input.set("Keep".to_string());
            apply_create_effect_signal(
                Err(ApiError::unknown("create failed")),
                holiday_message,
                holiday_error,
                holiday_date_input,
                holiday_name_input,
                holiday_desc_input,
                holidays_reload,
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("error").error,
                "create failed"
            );
            assert_eq!(holiday_date_input.get(), "2026-01-02");
            assert_eq!(holidays_reload.get(), 1);

            apply_delete_effect_signal(
                Ok(()),
                holiday_message,
                holiday_error,
                deleting_id,
                holidays_reload,
            );
            assert!(holiday_error.get().is_none());
            assert!(deleting_id.get().is_none());
            assert_eq!(holidays_reload.get(), 2);

            deleting_id.set(Some("h2".to_string()));
            apply_delete_effect_signal(
                Err(ApiError::unknown("delete failed")),
                holiday_message,
                holiday_error,
                deleting_id,
                holidays_reload,
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("error").error,
                "delete failed"
            );
            assert!(deleting_id.get().is_none());
            assert_eq!(holidays_reload.get(), 2);

            let fetched = CreateHolidayRequest {
                holiday_date: NaiveDate::from_ymd_opt(2026, 7, 20).expect("valid"),
                name: "Marine Day".into(),
                description: None,
            };
            apply_google_fetch_effect_signal(
                Ok(vec![fetched.clone()]),
                google_holidays,
                google_error,
            );
            assert!(google_error.get().is_none());
            assert_eq!(google_holidays.get().len(), 1);
            assert_eq!(google_holidays.get()[0].name, fetched.name);

            apply_google_fetch_effect_signal(
                Err(ApiError::unknown("google failed")),
                google_holidays,
                google_error,
            );
            assert!(google_holidays.get().is_empty());
            assert_eq!(
                google_error.get().as_ref().expect("error").error,
                "google failed"
            );

            apply_import_effect_signal(Ok(2), holiday_message, holiday_error, holidays_reload);
            assert!(holiday_error.get().is_none());
            assert_eq!(
                holiday_message.get().as_deref(),
                Some("2 件の祝日を追加しました。")
            );
            assert_eq!(holidays_reload.get(), 3);

            apply_import_effect_signal(
                Err(ApiError::unknown("import failed")),
                holiday_message,
                holiday_error,
                holidays_reload,
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("error").error,
                "import failed"
            );
            assert_eq!(holidays_reload.get(), 3);

            let payload = resolve_create_payload(
                "2026-01-03",
                "New Holiday",
                "  ",
                holiday_message,
                holiday_error,
            )
            .expect("payload");
            assert_eq!(payload.name, "New Holiday");
            assert!(payload.description.is_none());
            assert!(holiday_error.get().is_none());
            assert!(holiday_message.get().is_none());

            assert!(
                resolve_create_payload("", "name", "", holiday_message, holiday_error).is_none()
            );
            assert_eq!(
                holiday_error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );

            let existing = vec![NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid")];
            let import_candidates = resolve_import_payload(
                existing.clone(),
                vec![CreateHolidayRequest {
                    holiday_date: NaiveDate::from_ymd_opt(2026, 1, 2).expect("valid"),
                    name: "Import".into(),
                    description: None,
                }],
                holiday_message,
                holiday_error,
            )
            .expect("candidates");
            assert_eq!(import_candidates.len(), 1);
            assert!(holiday_error.get().is_none());
            assert!(holiday_message.get().is_none());

            let no_candidates = resolve_import_payload(
                existing,
                vec![CreateHolidayRequest {
                    holiday_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"),
                    name: "Existing".into(),
                    description: None,
                }],
                holiday_message,
                holiday_error,
            );
            assert!(no_candidates.is_none());
            assert_eq!(
                holiday_message.get().as_deref(),
                Some("追加対象の祝日はありません。")
            );
            assert!(holiday_error.get().is_none());

            let last_year = create_rw_signal(None::<Option<i32>>);
            let fetch_action = create_action(move |year: &Option<i32>| {
                last_year.set(Some(*year));
                async move { Ok(Vec::<CreateHolidayRequest>::new()) }
            });
            dispatch_google_fetch(fetch_action, "2026");
            assert_eq!(last_year.get(), Some(Some(2026)));
            dispatch_google_fetch(fetch_action, "bad");
            assert_eq!(last_year.get(), Some(None));

            let dispatched_delete = create_rw_signal(None::<String>);
            let delete_action = create_action(move |id: &String| {
                dispatched_delete.set(Some(id.clone()));
                async move { Ok(()) }
            });
            let deleting = create_rw_signal(None::<String>);
            dispatch_delete_holiday("h9".to_string(), deleting, delete_action);
            assert_eq!(deleting.get().as_deref(), Some("h9"));
            assert_eq!(dispatched_delete.get().as_deref(), Some("h9"));
            runtime.dispose();
        });
    }

    #[test]
    fn helper_load_holiday_page_respects_admin_flag() {
        with_local_runtime_async(|| async {
            let server = MockServer::start_async().await;
            server.mock(|when, then| {
                when.method(GET).path("/api/admin/holidays");
                then.status(200).json_body(serde_json::json!({
                    "page": 1,
                    "per_page": 10,
                    "total": 1,
                    "items": [{
                        "id": "h1",
                        "kind": "public",
                        "applies_from": "2026-01-02",
                        "applies_to": null,
                        "date": "2026-01-02",
                        "weekday": null,
                        "starts_on": null,
                        "ends_on": null,
                        "name": "Holiday",
                        "description": null,
                        "user_id": null,
                        "reason": null,
                        "created_by": null,
                        "created_at": "2026-01-01T00:00:00Z",
                        "is_override": null
                    }]
                }));
            });

            let repository = repo(&server);
            let query = HolidayListQuery {
                page: 2,
                per_page: 25,
                from: None,
                to: None,
            };

            let denied = load_holiday_page(repository.clone(), false, query.clone())
                .await
                .expect("denied result");
            assert_eq!(denied.page, 2);
            assert_eq!(denied.per_page, 25);
            assert_eq!(denied.total, 0);
            assert!(denied.items.is_empty());

            let allowed = load_holiday_page(repository, true, query)
                .await
                .expect("allowed result");
            assert_eq!(allowed.page, 1);
            assert_eq!(allowed.per_page, 10);
            assert_eq!(allowed.total, 1);
            assert_eq!(allowed.items.len(), 1);
            assert_eq!(allowed.items[0].name, "Holiday");
        });
    }
}
