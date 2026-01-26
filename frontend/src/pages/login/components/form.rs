use crate::{
    api::{ApiError, LoginRequest},
    components::error::InlineErrorMessage,
    pages::login::utils::LoginFormState,
};
use leptos::{ev::SubmitEvent, *};
use leptos_router::A;
use web_sys::HtmlInputElement;

#[component]
pub fn LoginForm(
    form: LoginFormState,
    error: RwSignal<Option<ApiError>>,
    login_action: Action<LoginRequest, Result<(), ApiError>>,
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
        <div class="min-h-screen flex items-center justify-center bg-surface py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-fg">
                        {"Timekeeper にログイン"}
                    </h2>
                    <p class="mt-2 text-center text-sm text-fg-muted">
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
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-form-control-border bg-form-control-bg placeholder-form-control-placeholder text-form-control-text rounded-t-md focus:outline-none focus:ring-2 focus:ring-action-primary-focus focus:border-action-primary-border focus:z-10 sm:text-sm"
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
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-form-control-border bg-form-control-bg placeholder-form-control-placeholder text-form-control-text rounded-b-md focus:outline-none focus:ring-2 focus:ring-action-primary-focus focus:border-action-primary-border focus:z-10 sm:text-sm"
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
                                class="appearance-none rounded relative block w-full px-3 py-2 border border-form-control-border bg-form-control-bg placeholder-form-control-placeholder text-form-control-text focus:outline-none focus:ring-2 focus:ring-action-primary-focus focus:border-action-primary-border focus:z-10 sm:text-sm"
                                placeholder="MFAコード (必要な場合)"
                                prop:value=form.totp_code
                                on:input=move |ev| {
                                    let target = event_target::<HtmlInputElement>(&ev);
                                    form.totp_code.set(target.value());
                                }
                            />
                        </div>
                    </div>

                    <div class="flex items-center justify-between">
                        <div class="text-sm">
                            <A href="/forgot-password" class="font-medium text-link hover:text-link-hover">
                                "Forgot your password?"
                            </A>
                        </div>
                    </div>

                    <InlineErrorMessage error=error.read_only().into() />

                    <div>
                        <button
                            type="submit"
                            disabled={move || pending.get()}
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-action-primary-text bg-action-primary-bg hover:bg-action-primary-bg-hover focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-action-primary-focus disabled:opacity-50"
                        >
                            <span class="absolute left-0 inset-y-0 flex items-center pl-3">
                                <svg
                                    class="h-5 w-5 text-action-primary-text/70 group-hover:text-action-primary-text"
                                    xmlns="http://www.w3.org/2000/svg"
                                    viewBox="0 0 20 20"
                                    fill="currentColor"
                                    aria-hidden="true"
                                >
                                    <path
                                        fill-rule="evenodd"
                                        d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                                        clip-rule="evenodd"
                                    />
                                </svg>
                            </span>
                            {move || if pending.get() { "ログイン中..." } else { "ログイン" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
