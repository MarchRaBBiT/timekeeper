use crate::{
    api::LoginRequest, pages::login::components::messages::InlineErrorMessage, pages::login::utils,
    state::auth as auth_state,
};
use leptos::*;
use web_sys::HtmlInputElement;

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
        if loading.get() {
            return;
        }
        let uname = username.get();
        let pword = password.get();

        if let Err(msg) = utils::validate_credentials(&uname, &pword) {
            set_error.set(Some(msg));
            return;
        }

        let totp_payload = utils::normalize_totp_code(&totp_code.get());
        set_loading.set(true);
        set_error.set(None);

        let set_loading2 = set_loading.clone();
        let set_error2 = set_error.clone();
        let set_auth2 = set_auth.clone();
        let set_totp_code2 = set_totp_code.clone();

        spawn_local(async move {
            let request = LoginRequest {
                username: uname,
                password: pword,
                totp_code: totp_payload,
                device_label: None,
            };
            match auth_state::login_request(request, set_auth2).await {
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
                                placeholder="MFAコード (必要な場合)"
                                prop:value=totp_code
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    set_totp_code.set(target.value());
                                }
                            />
                        </div>
                    </div>

                    <InlineErrorMessage error=error />

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
