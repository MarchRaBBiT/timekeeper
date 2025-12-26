use crate::{
    api::LoginRequest,
    pages::login::{components::messages::InlineErrorMessage, utils::LoginFormState},
};
use leptos::{ev::SubmitEvent, *};
use web_sys::HtmlInputElement;

#[component]
pub fn LoginForm(
    form: LoginFormState,
    error: RwSignal<Option<String>>,
    login_action: Action<LoginRequest, Result<(), String>>,
) -> impl IntoView {
    let pending = login_action.pending();

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        if pending.get_untracked() {
            return;
        }

        if let Err(msg) = form.validate() {
            error.set(Some(msg));
            return;
        }

        let request = LoginRequest {
            username: form.username.get_untracked(),
            password: form.password.get_untracked(),
            totp_code: form.normalize_totp(),
            device_label: None,
        };
        error.set(None);
        login_action.dispatch(request);
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
                <form class="mt-8 space-y-6" on:submit=on_submit>
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
                                prop:value=form.username
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    form.username.set(target.value());
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
                                prop:value=form.password
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    form.password.set(target.value());
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
                                prop:value=form.totp_code
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    form.totp_code.set(target.value());
                                }
                            />
                        </div>
                    </div>

                    <InlineErrorMessage error=error.read_only() />

                    <div>
                        <button
                            type="submit"
                            disabled={move || pending.get()}
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                        >
                            {move || if pending.get() { "ログイン中..." } else { "ログイン" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
