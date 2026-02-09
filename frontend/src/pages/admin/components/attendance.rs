use crate::{
    api::{AdminAttendanceUpsert, AdminBreakItem, ApiError},
    components::{error::InlineErrorMessage, forms::DatePicker, layout::SuccessMessage},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        repository::AdminRepository,
    },
};
use chrono::{NaiveDate, NaiveDateTime};
use leptos::{ev, *};

fn parse_dt_local(input: &str) -> Option<NaiveDateTime> {
    if input.len() == 16 {
        NaiveDateTime::parse_from_str(&format!("{}:00", input), "%Y-%m-%dT%H:%M:%S").ok()
    } else {
        NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S").ok()
    }
}

fn build_break_items(raw_breaks: Vec<(String, String)>) -> Vec<AdminBreakItem> {
    let mut break_items: Vec<AdminBreakItem> = vec![];
    for (start, end) in raw_breaks {
        if start.trim().is_empty() {
            continue;
        }
        let start_dt = parse_dt_local(&start);
        let end_dt = if end.trim().is_empty() {
            None
        } else {
            parse_dt_local(&end)
        };
        if let Some(start_dt) = start_dt {
            break_items.push(AdminBreakItem {
                break_start_time: start_dt,
                break_end_time: end_dt,
            });
        }
    }
    break_items
}

fn build_attendance_payload(
    user_id_raw: &str,
    date_raw: &str,
    clock_in_raw: &str,
    clock_out_raw: &str,
    raw_breaks: Vec<(String, String)>,
) -> Result<AdminAttendanceUpsert, ApiError> {
    let user_id = user_id_raw.trim();
    let date_raw = date_raw.trim();
    let clock_in = parse_dt_local(clock_in_raw.trim());
    if user_id.is_empty() || date_raw.is_empty() || clock_in.is_none() {
        return Err(ApiError::validation(
            "ユーザーID・日付・出勤時刻を入力してください。",
        ));
    }
    let date = NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .map_err(|_| ApiError::validation("日付は YYYY-MM-DD 形式で入力してください。"))?;
    let clock_out = if clock_out_raw.trim().is_empty() {
        None
    } else {
        parse_dt_local(clock_out_raw.trim())
    };
    let break_items = build_break_items(raw_breaks);
    Ok(AdminAttendanceUpsert {
        user_id: user_id.to_string(),
        date,
        clock_in_time: clock_in.expect("clock in checked above"),
        clock_out_time: clock_out,
        breaks: if break_items.is_empty() {
            None
        } else {
            Some(break_items)
        },
    })
}

fn validate_force_break_id(break_id_raw: &str) -> Result<String, ApiError> {
    let id = break_id_raw.trim();
    if id.is_empty() {
        Err(ApiError::validation("Break ID を入力してください。"))
    } else {
        Ok(id.to_string())
    }
}

fn attendance_upsert_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some("勤怠データを登録しました。".into()), None),
        Err(err) => (None, Some(err)),
    }
}

fn force_break_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some("休憩を強制終了しました。".into()), None),
        Err(err) => (None, Some(err)),
    }
}

fn append_empty_break_row(rows: &mut Vec<(String, String)>) {
    rows.push((String::new(), String::new()));
}

fn break_start_value(rows: &[(String, String)], idx: usize) -> String {
    rows.get(idx).map(|item| item.0.clone()).unwrap_or_default()
}

fn break_end_value(rows: &[(String, String)], idx: usize) -> String {
    rows.get(idx).map(|item| item.1.clone()).unwrap_or_default()
}

fn update_break_start(rows: &mut [(String, String)], idx: usize, value: String) {
    if let Some(item) = rows.get_mut(idx) {
        item.0 = value;
    }
}

fn update_break_end(rows: &mut [(String, String)], idx: usize, value: String) {
    if let Some(item) = rows.get_mut(idx) {
        item.1 = value;
    }
}

fn prepare_attendance_submission(
    system_admin_allowed: bool,
    user_id: &str,
    date: &str,
    clock_in: &str,
    clock_out: &str,
    breaks: Vec<(String, String)>,
) -> Result<Option<AdminAttendanceUpsert>, ApiError> {
    if !system_admin_allowed {
        Ok(None)
    } else {
        build_attendance_payload(user_id, date, clock_in, clock_out, breaks).map(Some)
    }
}

fn prepare_force_break_submission(
    system_admin_allowed: bool,
    break_id_raw: &str,
) -> Result<Option<String>, ApiError> {
    if !system_admin_allowed {
        Ok(None)
    } else {
        validate_force_break_id(break_id_raw).map(Some)
    }
}

fn add_break_row_signal(breaks: RwSignal<Vec<(String, String)>>) {
    breaks.update(append_empty_break_row);
}

async fn upsert_attendance_with_repo(
    repository: AdminRepository,
    payload: AdminAttendanceUpsert,
) -> Result<(), ApiError> {
    repository.upsert_attendance(payload).await
}

async fn force_end_break_with_repo(
    repository: AdminRepository,
    break_id: String,
) -> Result<(), ApiError> {
    repository.force_end_break(&break_id).await
}

fn apply_attendance_action_result(
    result: Result<(), ApiError>,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<ApiError>>,
) {
    let (next_message, next_error) = attendance_upsert_feedback(result);
    message.set(next_message);
    error.set(next_error);
}

fn apply_force_break_action_result(
    result: Result<(), ApiError>,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<ApiError>>,
) {
    let (next_message, next_error) = force_break_feedback(result);
    message.set(next_message);
    error.set(next_error);
}

fn resolve_attendance_dispatch_payload(
    system_admin_allowed: bool,
    user_id: &str,
    date: &str,
    clock_in: &str,
    clock_out: &str,
    breaks: Vec<(String, String)>,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<ApiError>>,
) -> Option<AdminAttendanceUpsert> {
    match prepare_attendance_submission(
        system_admin_allowed,
        user_id,
        date,
        clock_in,
        clock_out,
        breaks,
    ) {
        Ok(Some(payload)) => {
            message.set(None);
            error.set(None);
            Some(payload)
        }
        Ok(None) => None,
        Err(err) => {
            error.set(Some(err));
            message.set(None);
            None
        }
    }
}

fn resolve_force_break_dispatch_id(
    system_admin_allowed: bool,
    break_id_raw: &str,
    message: RwSignal<Option<String>>,
    error: RwSignal<Option<ApiError>>,
) -> Option<String> {
    match prepare_force_break_submission(system_admin_allowed, break_id_raw) {
        Ok(Some(id)) => {
            message.set(None);
            error.set(None);
            Some(id)
        }
        Ok(None) => None,
        Err(err) => {
            error.set(Some(err));
            message.set(None);
            None
        }
    }
}

fn set_input_signal(signal: RwSignal<String>, value: String) {
    signal.set(value);
}

fn update_break_start_signal(breaks: RwSignal<Vec<(String, String)>>, idx: usize, value: String) {
    breaks.update(|list| update_break_start(list, idx, value));
}

fn update_break_end_signal(breaks: RwSignal<Vec<(String, String)>>, idx: usize, value: String) {
    breaks.update(|list| update_break_end(list, idx, value));
}

fn attendance_submit_label(pending: bool) -> &'static str {
    if pending {
        "登録中..."
    } else {
        "勤怠を登録"
    }
}

fn force_break_button_label(pending: bool) -> &'static str {
    if pending {
        "終了中..."
    } else {
        "強制終了"
    }
}

#[component]
pub fn AdminAttendanceToolsSection(
    repository: AdminRepository,
    system_admin_allowed: Memo<bool>,
    users: UsersResource,
) -> impl IntoView {
    let att_user = create_rw_signal(String::new());
    let att_date = create_rw_signal(String::new());
    let att_in = create_rw_signal(String::new());
    let att_out = create_rw_signal(String::new());
    let breaks = create_rw_signal(Vec::<(String, String)>::new());
    let break_force_id = create_rw_signal(String::new());
    let message = create_rw_signal(None::<String>);
    let error = create_rw_signal(None::<ApiError>);

    let add_break = { move |_| add_break_row_signal(breaks) };

    let repo_for_attendance = repository.clone();
    let attendance_action = create_action(move |payload: &AdminAttendanceUpsert| {
        let repo = repo_for_attendance.clone();
        let payload = payload.clone();
        async move { upsert_attendance_with_repo(repo, payload).await }
    });
    let attendance_pending = attendance_action.pending();

    let repo_for_break = repository.clone();
    let force_break_action = create_action(move |break_id: &String| {
        let repo = repo_for_break.clone();
        let break_id = break_id.clone();
        async move { force_end_break_with_repo(repo, break_id).await }
    });
    let force_pending = force_break_action.pending();

    {
        create_effect(move |_| {
            if let Some(result) = attendance_action.value().get() {
                apply_attendance_action_result(result, message, error);
            }
        });
    }
    {
        create_effect(move |_| {
            if let Some(result) = force_break_action.value().get() {
                apply_force_break_action_result(result, message, error);
            }
        });
    }

    let on_submit_attendance = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            let Some(payload) = resolve_attendance_dispatch_payload(
                system_admin_allowed.get_untracked(),
                &att_user.get(),
                &att_date.get(),
                &att_in.get(),
                &att_out.get(),
                breaks.get(),
                message,
                error,
            ) else {
                return;
            };
            attendance_action.dispatch(payload);
        }
    };

    let on_force_end = {
        move |_| {
            let Some(id) = resolve_force_break_dispatch_id(
                system_admin_allowed.get_untracked(),
                &break_force_id.get(),
                message,
                error,
            ) else {
                return;
            };
            force_break_action.dispatch(id);
        }
    };

    view! {
        <Show when=move || system_admin_allowed.get()>
            <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
                <h3 class="text-lg font-medium text-fg">{"勤怠ツール"}</h3>
                <form class="space-y-3" on:submit=on_submit_attendance>
                    <AdminUserSelect
                        users=users
                        selected=att_user
                        label=Some("対象ユーザー".into())
                        placeholder="ユーザーを選択してください".into()
                    />
                    <DatePicker
                        label=Some("対象日")
                        value=att_date
                    />
                    <input type="datetime-local" class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1" on:input=move |ev| set_input_signal(att_in, event_target_value(&ev)) />
                    <input type="datetime-local" class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1" on:input=move |ev| set_input_signal(att_out, event_target_value(&ev)) />
                    <div>
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-sm text-fg-muted">{"休憩（任意）"}</span>
                            <button type="button" class="text-link hover:text-link-hover text-sm" on:click=add_break>{"行を追加"}</button>
                        </div>
                        <For
                            each=move || breaks.get().into_iter().enumerate()
                            key=|(idx, _)| *idx
                            children=move |(idx, _)| {
                                let start_value = {
                                    let breaks = breaks;
                                    move || breaks.with(|list| break_start_value(list, idx))
                                };
                                let end_value = {
                                    let breaks = breaks;
                                    move || breaks.with(|list| break_end_value(list, idx))
                                };
                                let on_start = {
                                    let breaks = breaks;
                                    move |ev| {
                                        update_break_start_signal(breaks, idx, event_target_value(&ev));
                                    }
                                };
                                let on_end = {
                                    let breaks = breaks;
                                    move |ev| {
                                        update_break_end_signal(breaks, idx, event_target_value(&ev));
                                    }
                                };
                                view! {
                                    <div class="flex space-x-2 mb-2">
                                        <input type="datetime-local" class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full" prop:value=start_value on:input=on_start />
                                        <input type="datetime-local" class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full" prop:value=end_value on:input=on_end />
                                    </div>
                                }
                            }
                        />
                    </div>
                    <button
                        type="submit"
                        class="w-full bg-action-primary-bg text-action-primary-text rounded py-2 disabled:opacity-50"
                        disabled={move || attendance_pending.get()}
                    >
                        {move || attendance_submit_label(attendance_pending.get())}
                    </button>
                </form>
                <div class="mt-4">
                    <h4 class="text-sm font-medium text-fg mb-2">{"休憩の強制終了"}</h4>
                    <div class="flex space-x-2">
                        <input
                            placeholder="Break ID"
                            class="border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 w-full"
                            on:input=move |ev| set_input_signal(break_force_id, event_target_value(&ev))
                        />
                        <button
                            class="px-3 py-1 bg-action-danger-bg text-action-danger-text rounded disabled:opacity-50"
                            disabled={move || force_pending.get()}
                            on:click=on_force_end
                        >
                            {move || force_break_button_label(force_pending.get())}
                        </button>
                    </div>
                </div>
                <Show when=move || error.get().is_some()>
                    <InlineErrorMessage error={error.into()} />
                </Show>
                <Show when=move || message.get().is_some()>
                    <SuccessMessage message={message.get().unwrap_or_default()} />
                </Show>
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::ApiClient;
    use crate::test_support::ssr::{render_to_string, with_local_runtime_async, with_runtime};

    #[test]
    fn parse_dt_local_accepts_minute_precision() {
        let parsed = parse_dt_local("2025-01-02T09:30");
        assert!(parsed.is_some());
    }

    #[test]
    fn parse_dt_local_accepts_second_precision() {
        let parsed = parse_dt_local("2025-01-02T09:30:00");
        assert!(parsed.is_some());
    }

    #[test]
    fn parse_dt_local_rejects_invalid_input() {
        assert!(parse_dt_local("not-a-datetime").is_none());
        assert!(parse_dt_local("2025-01-02").is_none());
        assert!(parse_dt_local("2025-13-02T09:30").is_none());
    }

    #[test]
    fn helper_build_break_items_skips_blank_and_invalid_rows() {
        let items = build_break_items(vec![
            ("".into(), "2025-01-01T13:00".into()),
            ("invalid".into(), "".into()),
            ("2025-01-01T12:00".into(), "invalid".into()),
            ("2025-01-01T15:00".into(), "2025-01-01T15:30".into()),
        ]);
        assert_eq!(items.len(), 2);
        assert!(items[0].break_end_time.is_none());
        assert!(items[1].break_end_time.is_some());
    }

    #[test]
    fn helper_build_attendance_payload_validates_inputs() {
        assert!(
            build_attendance_payload("", "2025-01-01", "2025-01-01T09:00", "", vec![]).is_err()
        );
        assert!(
            build_attendance_payload("u1", "bad-date", "2025-01-01T09:00", "", vec![]).is_err()
        );
        assert!(build_attendance_payload("u1", "2025-01-01", "bad-time", "", vec![]).is_err());
    }

    #[test]
    fn helper_build_attendance_payload_parses_breaks() {
        let payload = build_attendance_payload(
            "u1",
            "2025-01-01",
            "2025-01-01T09:00",
            "2025-01-01T18:00",
            vec![
                ("2025-01-01T12:00".into(), "2025-01-01T13:00".into()),
                ("".into(), "".into()),
                ("invalid".into(), "".into()),
            ],
        )
        .expect("payload");
        assert_eq!(payload.user_id, "u1");
        assert_eq!(payload.date.to_string(), "2025-01-01");
        assert_eq!(payload.breaks.as_ref().map(|v| v.len()), Some(1));
    }

    #[test]
    fn helper_build_attendance_payload_sets_optional_fields_to_none() {
        let payload = build_attendance_payload("u1", "2025-01-01", "2025-01-01T09:00", "", vec![])
            .expect("payload");
        assert!(payload.clock_out_time.is_none());
        assert!(payload.breaks.is_none());
    }

    #[test]
    fn helper_force_break_id_validation() {
        assert!(validate_force_break_id("").is_err());
        assert!(validate_force_break_id("   ").is_err());
        assert_eq!(
            validate_force_break_id("  break-1 ").expect("id"),
            "break-1"
        );
    }

    #[test]
    fn helper_build_attendance_payload_trims_and_accepts_second_precision() {
        let payload = build_attendance_payload(
            "  u1  ",
            " 2025-01-01 ",
            "2025-01-01T09:00:30",
            "2025-01-01T18:00:15",
            vec![],
        )
        .expect("payload");
        assert_eq!(payload.user_id, "u1");
        assert_eq!(payload.date.to_string(), "2025-01-01");
        assert_eq!(
            payload.clock_out_time.map(|dt| dt.to_string()),
            Some("2025-01-01 18:00:15".to_string())
        );
    }

    #[test]
    fn helper_build_attendance_payload_treats_invalid_clock_out_as_none() {
        let payload = build_attendance_payload(
            "u1",
            "2025-01-01",
            "2025-01-01T09:00",
            "invalid-clock-out",
            vec![],
        )
        .expect("payload");
        assert!(payload.clock_out_time.is_none());
    }

    #[test]
    fn helper_feedback_mappings_cover_success_and_error() {
        let (upsert_ok_msg, upsert_ok_err) = attendance_upsert_feedback(Ok(()));
        assert_eq!(upsert_ok_msg.as_deref(), Some("勤怠データを登録しました。"));
        assert!(upsert_ok_err.is_none());

        let (upsert_err_msg, upsert_err) =
            attendance_upsert_feedback(Err(ApiError::unknown("upsert failed")));
        assert!(upsert_err_msg.is_none());
        assert_eq!(upsert_err.expect("error").error, "upsert failed");

        let (force_ok_msg, force_ok_err) = force_break_feedback(Ok(()));
        assert_eq!(force_ok_msg.as_deref(), Some("休憩を強制終了しました。"));
        assert!(force_ok_err.is_none());

        let (force_err_msg, force_err) =
            force_break_feedback(Err(ApiError::unknown("force failed")));
        assert!(force_err_msg.is_none());
        assert_eq!(force_err.expect("error").error, "force failed");
    }

    #[test]
    fn helper_break_row_access_and_update_cover_paths() {
        let mut rows = vec![(
            "2025-01-01T12:00".to_string(),
            "2025-01-01T13:00".to_string(),
        )];
        append_empty_break_row(&mut rows);
        assert_eq!(rows.len(), 2);
        assert_eq!(break_start_value(&rows, 0), "2025-01-01T12:00");
        assert_eq!(break_end_value(&rows, 0), "2025-01-01T13:00");
        assert_eq!(break_start_value(&rows, 9), "");
        assert_eq!(break_end_value(&rows, 9), "");

        update_break_start(&mut rows, 1, "2025-01-01T14:00".to_string());
        update_break_end(&mut rows, 1, "2025-01-01T14:30".to_string());
        assert_eq!(rows[1].0, "2025-01-01T14:00");
        assert_eq!(rows[1].1, "2025-01-01T14:30");
    }

    #[test]
    fn helper_prepare_submission_handles_permission_and_validation() {
        let blocked = prepare_attendance_submission(
            false,
            "u1",
            "2025-01-01",
            "2025-01-01T09:00",
            "",
            vec![],
        )
        .expect("admin check should return none");
        assert!(blocked.is_none());

        let accepted =
            prepare_attendance_submission(true, "u1", "2025-01-01", "2025-01-01T09:00", "", vec![])
                .expect("valid payload")
                .expect("dispatch payload");
        assert_eq!(accepted.user_id, "u1");

        let invalid =
            prepare_attendance_submission(true, "", "2025-01-01", "2025-01-01T09:00", "", vec![]);
        assert!(invalid.is_err());

        let force_blocked =
            prepare_force_break_submission(false, "br-1").expect("admin check should return none");
        assert!(force_blocked.is_none());

        let force_ok = prepare_force_break_submission(true, "  br-1 ")
            .expect("valid id")
            .expect("dispatch id");
        assert_eq!(force_ok, "br-1");

        let force_err = prepare_force_break_submission(true, "   ");
        assert!(force_err.is_err());
    }

    #[test]
    fn helper_signal_and_label_logic_cover_paths() {
        with_runtime(|| {
            let breaks_signal = create_rw_signal(Vec::<(String, String)>::new());
            add_break_row_signal(breaks_signal);
            assert_eq!(breaks_signal.get().len(), 1);

            update_break_start_signal(breaks_signal, 0, "2025-01-01T12:00".to_string());
            update_break_end_signal(breaks_signal, 0, "2025-01-01T12:30".to_string());
            assert_eq!(
                break_start_value(&breaks_signal.get(), 0),
                "2025-01-01T12:00"
            );
            assert_eq!(break_end_value(&breaks_signal.get(), 0), "2025-01-01T12:30");

            update_break_start_signal(breaks_signal, 99, "ignored".to_string());
            update_break_end_signal(breaks_signal, 99, "ignored".to_string());
            assert_eq!(breaks_signal.get().len(), 1);

            let text_signal = create_rw_signal(String::new());
            set_input_signal(text_signal, "updated".to_string());
            assert_eq!(text_signal.get(), "updated");

            assert_eq!(attendance_submit_label(true), "登録中...");
            assert_eq!(attendance_submit_label(false), "勤怠を登録");
            assert_eq!(force_break_button_label(true), "終了中...");
            assert_eq!(force_break_button_label(false), "強制終了");
        });
    }

    #[test]
    fn helper_resolve_dispatch_payload_paths_cover_permission_and_validation() {
        with_runtime(|| {
            let message = create_rw_signal(Some("old".to_string()));
            let error = create_rw_signal(None::<ApiError>);

            let blocked = resolve_attendance_dispatch_payload(
                false,
                "u1",
                "2025-01-01",
                "2025-01-01T09:00",
                "",
                vec![],
                message,
                error,
            );
            assert!(blocked.is_none());
            assert_eq!(message.get().as_deref(), Some("old"));
            assert!(error.get().is_none());

            let invalid = resolve_attendance_dispatch_payload(
                true,
                "",
                "2025-01-01",
                "2025-01-01T09:00",
                "",
                vec![],
                message,
                error,
            );
            assert!(invalid.is_none());
            assert_eq!(message.get(), None);
            assert_eq!(
                error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );

            message.set(Some("to-clear".to_string()));
            error.set(Some(ApiError::unknown("to-clear")));
            let valid = resolve_attendance_dispatch_payload(
                true,
                "u1",
                "2025-01-01",
                "2025-01-01T09:00",
                "",
                vec![],
                message,
                error,
            )
            .expect("payload");
            assert_eq!(valid.user_id, "u1");
            assert!(message.get().is_none());
            assert!(error.get().is_none());

            message.set(Some("old-break".to_string()));
            let blocked_break = resolve_force_break_dispatch_id(false, "br-1", message, error);
            assert!(blocked_break.is_none());
            assert_eq!(message.get().as_deref(), Some("old-break"));

            let invalid_break = resolve_force_break_dispatch_id(true, "   ", message, error);
            assert!(invalid_break.is_none());
            assert_eq!(message.get(), None);
            assert_eq!(
                error.get().as_ref().expect("validation").code,
                "VALIDATION_ERROR"
            );

            message.set(Some("to-clear".to_string()));
            error.set(Some(ApiError::unknown("to-clear")));
            let valid_break =
                resolve_force_break_dispatch_id(true, "  br-1 ", message, error).expect("id");
            assert_eq!(valid_break, "br-1");
            assert!(message.get().is_none());
            assert!(error.get().is_none());
        });
    }

    #[test]
    fn helper_apply_action_result_updates_message_and_error_signals() {
        with_runtime(|| {
            let message = create_rw_signal(None::<String>);
            let error = create_rw_signal(None::<ApiError>);

            apply_attendance_action_result(Ok(()), message, error);
            assert_eq!(message.get().as_deref(), Some("勤怠データを登録しました。"));
            assert!(error.get().is_none());

            apply_attendance_action_result(Err(ApiError::unknown("upsert failed")), message, error);
            assert!(message.get().is_none());
            assert_eq!(error.get().as_ref().expect("error").error, "upsert failed");

            apply_force_break_action_result(Ok(()), message, error);
            assert_eq!(message.get().as_deref(), Some("休憩を強制終了しました。"));
            assert!(error.get().is_none());

            apply_force_break_action_result(Err(ApiError::unknown("force failed")), message, error);
            assert!(message.get().is_none());
            assert_eq!(error.get().as_ref().expect("error").error, "force failed");
        });
    }

    #[test]
    fn helper_repo_async_wrappers_cover_success_and_error() {
        with_local_runtime_async(|| async {
            let server = MockServer::start_async().await;
            server.mock(|when, then| {
                when.method(PUT).path("/api/admin/attendance");
                then.status(200).json_body(serde_json::json!({
                    "id": "att-1",
                    "user_id": "u1",
                    "date": "2025-01-02",
                    "clock_in_time": "2025-01-02T09:00:00",
                    "clock_out_time": null,
                    "status": "clocked_in",
                    "total_work_hours": null,
                    "break_records": []
                }));
            });
            server.mock(|when, then| {
                when.method(PUT).path("/api/admin/breaks/br-1/force-end");
                then.status(200).json_body(serde_json::json!({
                    "id": "br-1",
                    "attendance_id": "att-1",
                    "break_start_time": "2025-01-02T12:00:00",
                    "break_end_time": null,
                    "duration_minutes": null
                }));
            });

            let repo = AdminRepository::new_with_client(std::rc::Rc::new(
                ApiClient::new_with_base_url(&server.url("/api")),
            ));
            let payload = AdminAttendanceUpsert {
                user_id: "u1".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 1, 2).expect("valid date"),
                clock_in_time: NaiveDateTime::parse_from_str(
                    "2025-01-02T09:00:00",
                    "%Y-%m-%dT%H:%M:%S",
                )
                .expect("valid datetime"),
                clock_out_time: None,
                breaks: None,
            };

            assert!(upsert_attendance_with_repo(repo.clone(), payload)
                .await
                .is_ok());
            assert!(force_end_break_with_repo(repo.clone(), "br-1".to_string())
                .await
                .is_ok());
            assert!(force_end_break_with_repo(repo, "br-missing".to_string())
                .await
                .is_err());
        });
    }

    #[test]
    fn attendance_tools_section_renders() {
        let html = render_to_string(move || {
            let api = ApiClient::new();
            let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let allowed = create_memo(|_| true);
            view! { <AdminAttendanceToolsSection repository=repo system_admin_allowed=allowed users=users /> }
        });
        assert!(html.contains("勤怠ツール"));
    }

    #[test]
    fn attendance_tools_section_hidden_when_not_system_admin() {
        let html = render_to_string(move || {
            let api = ApiClient::new();
            let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let allowed = create_memo(|_| false);
            view! { <AdminAttendanceToolsSection repository=repo system_admin_allowed=allowed users=users /> }
        });
        assert!(!html.contains("勤怠ツール"));
        assert!(!html.contains("休憩の強制終了"));
    }
}
