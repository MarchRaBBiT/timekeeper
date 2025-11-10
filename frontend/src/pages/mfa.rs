use crate::{
    api::{MfaSetupResponse, MfaStatusResponse},
    components::layout::{ErrorMessage, Layout, LoadingSpinner, SuccessMessage},
    state::auth::{self, use_auth},
};
use leptos::ev::SubmitEvent;
use leptos::*;
use web_sys::HtmlInputElement;

#[component]
pub fn MfaRegisterPage() -> impl IntoView {
    let (_auth_state, set_auth_state) = use_auth();

    let (status, set_status) = create_signal::<Option<MfaStatusResponse>>(None);
    let (status_loading, set_status_loading) = create_signal(true);
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(None::<String>);
    let (setup_info, set_setup_info) = create_signal::<Option<MfaSetupResponse>>(None);
    let (totp_code, set_totp_code) = create_signal(String::new());
    let (register_loading, set_register_loading) = create_signal(false);
    let (activate_loading, set_activate_loading) = create_signal(false);

    let fetch_status = {
        let set_status = set_status.clone();
        let set_status_loading = set_status_loading.clone();
        let set_error = set_error.clone();
        create_action(move |_| {
            let set_status = set_status.clone();
            let set_status_loading = set_status_loading.clone();
            let set_error = set_error.clone();
            async move {
                set_error.set(None);
                set_status_loading.set(true);
                match auth::fetch_mfa_status().await {
                    Ok(resp) => set_status.set(Some(resp)),
                    Err(err) => set_error.set(Some(err)),
                }
                set_status_loading.set(false);
            }
        })
    };
    fetch_status.dispatch(());

    let start_registration = {
        let fetch_status_action = fetch_status.clone();
        let set_error = set_error.clone();
        let set_success = set_success.clone();
        let set_setup_info = set_setup_info.clone();
        let set_register_loading = set_register_loading.clone();
        move |_| {
            if register_loading.get() {
                return;
            }
            set_error.set(None);
            set_success.set(None);
            set_setup_info.set(None);
            set_register_loading.set(true);
            let fetch_status_action = fetch_status_action.clone();
            let set_setup_info = set_setup_info.clone();
            let set_success = set_success.clone();
            let set_error = set_error.clone();
            let set_register_loading = set_register_loading.clone();
            spawn_local(async move {
                match auth::register_mfa().await {
                    Ok(info) => {
                        set_setup_info.set(Some(info));
                        fetch_status_action.dispatch(());
                        set_success.set(Some(
                            "認証アプリにシークレットを登録し、確認コードを入力してください。"
                                .into(),
                        ));
                    }
                    Err(err) => set_error.set(Some(err)),
                }
                set_register_loading.set(false);
            });
        }
    };

    let handle_activate = {
        let fetch_status_action = fetch_status.clone();
        let totp_code_signal = totp_code.clone();
        let set_totp_code = set_totp_code.clone();
        let set_setup_info = set_setup_info.clone();
        let set_error = set_error.clone();
        let set_success = set_success.clone();
        let set_activate_loading = set_activate_loading.clone();
        let set_auth_state = set_auth_state.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if activate_loading.get() {
                return;
            }
            let code_value = totp_code_signal.get();
            let trimmed = code_value.trim().to_string();
            if trimmed.len() < 6 {
                set_error.set(Some("6桁のワンタイムコードを入力してください".into()));
                return;
            }
            set_activate_loading.set(true);
            set_error.set(None);
            set_success.set(None);

            let fetch_status_action = fetch_status_action.clone();
            let set_setup_info = set_setup_info.clone();
            let set_totp_code = set_totp_code.clone();
            let set_success = set_success.clone();
            let set_error = set_error.clone();
            let set_activate_loading = set_activate_loading.clone();
            let set_auth_state = set_auth_state.clone();
            spawn_local(async move {
                match auth::activate_mfa(trimmed, Some(set_auth_state)).await {
                    Ok(_) => {
                        set_setup_info.set(None);
                        set_totp_code.set(String::new());
                        fetch_status_action.dispatch(());
                        set_success.set(Some("MFA を有効化しました。次回以降のログインにワンタイムコードが必要です。".into()));
                    }
                    Err(err) => set_error.set(Some(err)),
                }
                set_activate_loading.set(false);
            });
        }
    };

    view! {
        <Layout>
            <div class="max-w-3xl mx-auto bg-white shadow rounded-lg p-6 space-y-6">
                <div>
                    <h2 class="text-2xl font-semibold text-gray-900">
                        "多要素認証 (MFA) 設定"
                    </h2>
                    <p class="mt-2 text-sm text-gray-600">
                        "Timekeeper へのログインに認証アプリのワンタイムコードを追加します。"
                    </p>
                </div>

                {move || {
                    success.get().map(|msg| view! { <SuccessMessage message=msg/> }.into_view())
                        .unwrap_or_else(|| view! {}.into_view())
                }}

                {move || {
                    error.get().map(|msg| view! { <ErrorMessage message=msg/> }.into_view())
                        .unwrap_or_else(|| view! {}.into_view())
                }}

                <Show when=move || status_loading.get() fallback=|| ()>
                    <LoadingSpinner/>
                </Show>

                <Show when=move || status.get().is_some() fallback=|| ()>
                    {move || {
                        status.get().map(|current| {
                            if current.enabled {
                                view! {
                                    <div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded">
                                        "MFA は有効です。Authenticator アプリを紛失した場合は再登録してください。"
                                    </div>
                                }.into_view()
                            } else if current.pending {
                                view! {
                                    <div class="bg-yellow-50 border border-yellow-200 text-yellow-800 px-4 py-3 rounded">
                                        "セットアップが保留中です。認証アプリに登録し、ワンタイムコードで有効化してください。"
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="bg-blue-50 border border-blue-200 text-blue-800 px-4 py-3 rounded">
                                        "まだ MFA は無効です。下のボタンから登録を開始できます。"
                                    </div>
                                }.into_view()
                            }
                        }).unwrap_or_else(|| view! {}.into_view())
                    }}
                </Show>

                <div class="flex items-center space-x-4">
                    <button
                        on:click=start_registration
                        disabled=move || {
                            register_loading.get()
                                || status
                                    .get()
                                    .map(|s| s.enabled)
                                    .unwrap_or(true)
                        }
                        class="px-4 py-2 rounded-md text-white bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50"
                    >
                        {move || if register_loading.get() { "シークレット発行中..." } else { "MFA 登録を開始" }}
                    </button>
                    <p class="text-sm text-gray-500">
                        "登録をやり直すと新しいシークレットが発行され、以前のコードは無効になります。"
                    </p>
                </div>

                <Show when=move || setup_info.get().is_some() fallback=|| ()>
                    {move || {
                        setup_info.get().map(|info| {
                            view! {
                                <div class="border border-dashed border-indigo-300 rounded-lg p-4 space-y-2 bg-indigo-50">
                                    <p class="text-sm text-gray-600">
                                        "認証アプリで下記シークレットまたは otpauth URL を登録してください。"
                                    </p>
                                    <div class="font-mono text-lg tracking-widest text-gray-900 break-all">
                                        {info.secret.clone()}
                                    </div>
                                    <div class="text-xs text-gray-500 break-all">
                                        {info.otpauth_url.clone()}
                                    </div>
                                </div>
                            }.into_view()
                        }).unwrap_or_else(|| view! {}.into_view())
                    }}
                </Show>

                <Show
                    when=move || {
                        setup_info.get().is_some()
                            || status
                                .get()
                                .map(|s| s.pending && !s.enabled)
                                .unwrap_or(false)
                    }
                    fallback=|| ()
                >
                    <div class="border rounded-lg p-4 space-y-4">
                        <h3 class="text-lg font-semibold text-gray-900">
                            "確認コードを入力"
                        </h3>
                        <form class="space-y-4" on:submit=handle_activate>
                            <div>
                                <label for="totp-code" class="block text-sm font-medium text-gray-700">
                                    "6桁のコード"
                                </label>
                                <input
                                    id="totp-code"
                                    name="totp-code"
                                    type="text"
                                    inputmode="numeric"
                                    pattern="[0-9]*"
                                    maxlength="6"
                                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 font-mono text-lg tracking-widest"
                                    placeholder="123456"
                                    prop:value=totp_code
                                    on:input=move |ev| {
                                        let target = event_target::<HtmlInputElement>(&ev);
                                        set_totp_code.set(target.value());
                                    }
                                />
                            </div>
                            <div>
                                <button
                                    type="submit"
                                    disabled=move || activate_loading.get() || totp_code.get().trim().len() < 6
                                    class="px-4 py-2 rounded-md text-white bg-green-600 hover:bg-green-700 disabled:opacity-50"
                                >
                                    {move || if activate_loading.get() { "検証中..." } else { "MFA を有効化" }}
                                </button>
                            </div>
                        </form>
                    </div>
                </Show>
            </div>
        </Layout>
    }
}
