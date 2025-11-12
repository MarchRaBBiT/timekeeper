use chrono::Utc;
use leptos::*;
use web_sys::HtmlInputElement;

use crate::state::attendance::{
    self as attendance_state, load_attendance_range, load_today_status, AttendanceState,
};
use crate::state::auth as auth_state;

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
pub fn ClockInButton(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    let (loading, set_loading) = create_signal(false);
    let (message, set_message) = create_signal(None::<String>);

    let handle_clock_in = move |_| {
        if loading.get() {
            return;
        }
        set_loading.set(true);
        set_message.set(None);

        let set_state = set_attendance_state;
        spawn_local(async move {
            let today = Utc::now().date_naive();
            match attendance_state::clock_in(set_state).await {
                Ok(_) => {
                    let _ = load_today_status(set_state).await;
                    let _ = load_attendance_range(set_state, Some(today), Some(today)).await;
                    set_message.set(Some("出勤しました".to_string()));
                }
                Err(err) => {
                    set_message.set(Some(format!("出勤に失敗しました: {}", err)));
                }
            }
            set_loading.set(false);
        });
    };

    let disabled = move || {
        let state = attendance_state.get();
        let status = state.today_status.as_ref().map(|s| s.status.as_str());
        loading.get()
            || matches!(
                status,
                Some("clocked_in") | Some("on_break") | Some("clocked_out")
            )
    };

    view! {
        <div class="text-center">
            <button
                on:click=handle_clock_in
                disabled=disabled()
                class="bg-green-600 hover:bg-green-700 text-white font-bold py-4 px-8 rounded-lg text-lg disabled:opacity-50"
            >
                {move || if loading.get() { "処理中..." } else { "出勤" }}
            </button>
            {move || {
                if let Some(msg) = message.get() {
                    view! { <p class="mt-2 text-green-600 text-sm">{msg}</p> }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}

#[component]
pub fn ClockOutButton(
    attendance_state: ReadSignal<AttendanceState>,
    set_attendance_state: WriteSignal<AttendanceState>,
) -> impl IntoView {
    let (loading, set_loading) = create_signal(false);
    let (message, set_message) = create_signal(None::<String>);

    let handle_clock_out = move |_| {
        if loading.get() {
            return;
        }
        set_loading.set(true);
        set_message.set(None);

        let set_state = set_attendance_state;
        spawn_local(async move {
            let today = Utc::now().date_naive();
            match attendance_state::clock_out(set_state).await {
                Ok(_) => {
                    let _ = load_today_status(set_state).await;
                    let _ = load_attendance_range(set_state, Some(today), Some(today)).await;
                    set_message.set(Some("退勤しました".to_string()));
                }
                Err(err) => {
                    set_message.set(Some(format!("退勤に失敗しました: {}", err)));
                }
            }
            set_loading.set(false);
        });
    };

    let disabled = move || {
        let state = attendance_state.get();
        let status = state.today_status.as_ref().map(|s| s.status.as_str());
        loading.get() || !matches!(status, Some("clocked_in") | Some("on_break"))
    };

    view! {
        <div class="text-center">
            <button
                on:click=handle_clock_out
                disabled=disabled()
                class="bg-red-600 hover:bg-red-700 text-white font-bold py-4 px-8 rounded-lg text-lg disabled:opacity-50"
            >
                {move || if loading.get() { "処理中..." } else { "退勤" }}
            </button>
            {move || {
                if let Some(msg) = message.get() {
                    view! { <p class="mt-2 text-red-600 text-sm">{msg}</p> }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}
