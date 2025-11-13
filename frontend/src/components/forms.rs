use chrono::Utc;
use leptos::*;
use web_sys::HtmlInputElement;

use crate::api::ApiClient;
use crate::state::attendance::{
    self as attendance_state, load_attendance_range, load_today_status, AttendanceState,
};
use crate::state::auth as auth_state;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AttendanceButtonFlags {
    clock_in: bool,
    break_start: bool,
    break_end: bool,
    clock_out: bool,
}

fn button_flags_for(status: Option<&str>, loading: bool) -> AttendanceButtonFlags {
    if loading {
        return AttendanceButtonFlags::default();
    }

    match status.unwrap_or("not_started") {
        "not_started" => AttendanceButtonFlags {
            clock_in: true,
            ..Default::default()
        },
        "clocked_in" => AttendanceButtonFlags {
            break_start: true,
            clock_out: true,
            ..Default::default()
        },
        "on_break" => AttendanceButtonFlags {
            break_end: true,
            clock_out: true,
            ..Default::default()
        },
        "clocked_out" => AttendanceButtonFlags::default(),
        _ => AttendanceButtonFlags::default(),
    }
}

#[component]
pub fn LoginForm() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (totp_code, set_totp_code) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(false);
    let (_auth, set_auth) = auth_state::use_auth();

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_loading.set(true);
        set_error.set(None);

        let uname = username.get();
        let pword = password.get();
        let code = totp_code.get();
        let totp_payload = {
            let trimmed = code.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        let set_loading2 = set_loading.clone();
        let set_error2 = set_error.clone();
        let set_auth2 = set_auth.clone();
        let set_totp_code2 = set_totp_code.clone();

        spawn_local(async move {
            match auth_state::login(uname, pword, totp_payload, set_auth2).await {
                Ok(_) => {
                    set_loading2.set(false);
                    set_totp_code2.set(String::new());
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/dashboard");
                    }
                }
                Err(err) => {
                    set_loading2.set(false);
                    set_error2.set(Some(err));
                }
            }
        });
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        {"Timekeeper にログイン"}
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        {"勤怠管理システム"}
                    </p>
                </div>
                <form class="mt-8 space-y-6" on:submit=handle_submit>
                    <div class="rounded-md shadow-sm -space-y-px">
                        <div>
                            <label for="username" class="sr-only">{"ユーザー名"}</label>
                            <input
                                id="username"
                                name="username"
                                type="text"
                                required
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                                placeholder="ユーザー名"
                                prop:value=username
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    set_username.set(target.value());
                                }
                            />
                        </div>
                        <div>
                            <label for="password" class="sr-only">{"パスワード"}</label>
                            <input
                                id="password"
                                name="password"
                                type="password"
                                required
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                                placeholder="パスワード"
                                prop:value=password
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    set_password.set(target.value());
                                }
                            />
                        </div>
                        <div>
                            <label for="totp_code" class="sr-only">{"MFAコード"}</label>
                            <input
                                id="totp_code"
                                name="totp_code"
                                type="text"
                                inputmode="numeric"
                                autocomplete="one-time-code"
                                class="appearance-none rounded relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                                placeholder="MFAコード (有効化済みの場合)"
                                prop:value=totp_code
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    set_totp_code.set(target.value());
                                }
                            />
                        </div>
                    </div>

                    {move || {
                        if let Some(error_msg) = error.get() {
                            view! {
                                <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
                                    {error_msg}
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}

                    <div>
                        <button
                            type="submit"
                            disabled=loading
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                        >
                            {move || if loading.get() { "ログイン中..." } else { "ログイン" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
pub fn AttendanceActionButtons(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    let (loading, set_loading) = create_signal(false);
    let (message, set_message) = create_signal(None::<String>);

    let refresh_state = {
        let set_attendance_state = set_attendance_state.clone();
        move || {
            let set_state = set_attendance_state.clone();
            async move {
                let today = Utc::now().date_naive();
                let _ = load_today_status(set_state.clone()).await;
                let _ = load_attendance_range(set_state, Some(today), Some(today)).await;
            }
        }
    };

    let handle_clock_in = {
        let set_attendance_state = set_attendance_state.clone();
        let set_message = set_message.clone();
        let set_loading = set_loading.clone();
        move |_| {
            if loading.get() {
                return;
            }
            set_loading.set(true);
            set_message.set(None);
            let set_state = set_attendance_state.clone();
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let result = attendance_state::clock_in(set_state.clone()).await;
                match result {
                    Ok(_) => {
                        refresh_state().await;
                        set_message.set(Some("出勤が完了しました。".into()));
                    }
                    Err(err) => set_message.set(Some(format!("出勤に失敗しました: {}", err))),
                }
                set_loading.set(false);
            });
        }
    };

    let handle_break_start = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        move |_| {
            if loading.get() {
                return;
            }
            let Some(status) = attendance_state.get().today_status.clone() else {
                set_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "clocked_in" {
                set_message.set(Some("勤務中のみ休憩を開始できます。".into()));
                return;
            }
            let Some(att_id) = status.attendance_id.clone() else {
                set_message.set(Some("勤怠レコードが見つかりません。".into()));
                return;
            };
            set_loading.set(true);
            set_message.set(None);
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.break_start(&att_id).await {
                    Ok(_) => {
                        refresh_state().await;
                        set_message.set(Some("休憩を開始しました。".into()));
                    }
                    Err(err) => set_message.set(Some(format!("休憩開始に失敗しました: {}", err))),
                }
                set_loading.set(false);
            });
        }
    };

    let handle_break_end = {
        let attendance_state = attendance_state.clone();
        let set_message = set_message.clone();
        move |_| {
            if loading.get() {
                return;
            }
            let Some(status) = attendance_state.get().today_status.clone() else {
                set_message.set(Some("ステータスを取得できません。".into()));
                return;
            };
            if status.status != "on_break" {
                set_message.set(Some("休憩中のみ休憩を終了できます。".into()));
                return;
            }
            let Some(break_id) = status.active_break_id.clone() else {
                set_message.set(Some("休憩レコードが見つかりません。".into()));
                return;
            };
            set_loading.set(true);
            set_message.set(None);
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let api = ApiClient::new();
                match api.break_end(&break_id).await {
                    Ok(_) => {
                        refresh_state().await;
                        set_message.set(Some("休憩を終了しました。".into()));
                    }
                    Err(err) => set_message.set(Some(format!("休憩終了に失敗しました: {}", err))),
                }
                set_loading.set(false);
            });
        }
    };

    let handle_clock_out = {
        let set_attendance_state = set_attendance_state.clone();
        let set_message = set_message.clone();
        let set_loading = set_loading.clone();
        move |_| {
            if loading.get() {
                return;
            }
            set_loading.set(true);
            set_message.set(None);
            let set_state = set_attendance_state.clone();
            let set_message = set_message.clone();
            let set_loading = set_loading.clone();
            spawn_local(async move {
                let result = attendance_state::clock_out(set_state.clone()).await;
                match result {
                    Ok(_) => {
                        refresh_state().await;
                        set_message.set(Some("退勤が完了しました。".into()));
                    }
                    Err(err) => set_message.set(Some(format!("退勤に失敗しました: {}", err))),
                }
                set_loading.set(false);
            });
        }
    };

    let status_snapshot = move || attendance_state.get().today_status.clone();
    let button_state = move || {
        button_flags_for(
            status_snapshot().as_ref().map(|s| s.status.as_str()),
            loading.get(),
        )
    };

    view! {
        <div class="space-y-3">
            <div class="flex flex-wrap items-center gap-4">
                {move || {
                    let status = status_snapshot();
                    let (label, color) = match status.as_ref().map(|s| s.status.as_str()) {
                        Some("clocked_in") => ("勤務中", "bg-blue-100 text-blue-800"),
                        Some("on_break") => ("休憩中", "bg-yellow-100 text-yellow-800"),
                        Some("clocked_out") => ("退勤済み", "bg-green-100 text-green-800"),
                        _ => ("未出勤", "bg-gray-100 text-gray-800"),
                    };
                    view! {
                        <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", color)>
                            {label}
                        </span>
                    }.into_view()
                }}
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-4 py-2 rounded bg-indigo-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_in}
                        on:click=handle_clock_in
                    >{"出勤"}</button>
                    <button
                        class="px-4 py-2 rounded bg-amber-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_start}
                        on:click=handle_break_start
                    >{"休憩開始"}</button>
                    <button
                        class="px-4 py-2 rounded bg-amber-700 text-white disabled:opacity-50"
                        disabled={move || !button_state().break_end}
                        on:click=handle_break_end
                    >{"休憩終了"}</button>
                    <button
                        class="px-4 py-2 rounded bg-red-600 text-white disabled:opacity-50"
                        disabled={move || !button_state().clock_out}
                        on:click=handle_clock_out
                    >{"退勤"}</button>
                </div>
            </div>
            {move || {
                message
                    .get()
                    .map(|msg| view! { <p class="text-sm text-gray-700">{msg}</p> }.into_view())
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn not_started_only_allows_clock_in() {
        let flags = button_flags_for(None, false);
        assert!(flags.clock_in);
        assert!(!flags.break_start);
        assert!(!flags.break_end);
        assert!(!flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn clocked_in_allows_break_start_and_clock_out() {
        let flags = button_flags_for(Some("clocked_in"), false);
        assert!(!flags.clock_in);
        assert!(flags.break_start);
        assert!(!flags.break_end);
        assert!(flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn on_break_allows_break_end_and_clock_out() {
        let flags = button_flags_for(Some("on_break"), false);
        assert!(!flags.clock_in);
        assert!(!flags.break_start);
        assert!(flags.break_end);
        assert!(flags.clock_out);
    }

    #[wasm_bindgen_test]
    fn clocked_out_disables_all_buttons() {
        let flags = button_flags_for(Some("clocked_out"), false);
        assert_eq!(flags, AttendanceButtonFlags::default());
    }

    #[wasm_bindgen_test]
    fn loading_state_disables_everything() {
        let flags = button_flags_for(Some("clocked_in"), true);
        assert_eq!(flags, AttendanceButtonFlags::default());
    }
}
