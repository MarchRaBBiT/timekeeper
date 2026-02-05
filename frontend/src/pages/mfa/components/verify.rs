use crate::api::MfaSetupResponse;
use crate::pages::mfa::utils::svg_to_data_url;
use leptos::{ev::SubmitEvent, Callback, *};
use qrcode::{render::svg, QrCode};
use web_sys::HtmlInputElement;

#[component]
pub fn VerificationSection(
    setup_info: ReadSignal<Option<MfaSetupResponse>>,
    activate_loading: Signal<bool>,
    on_submit: Callback<SubmitEvent>,
    on_input: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <Show when=move || setup_info.get().is_some() fallback=|| ()>
            {move || {
                setup_info
                    .get()
                    .map(|info| {
                        let qr_svg = QrCode::new(info.otpauth_url.as_bytes())
                            .map(|code| code.render::<svg::Color>().build())
                            .unwrap_or_default();
                        let qr_svg_data_url = svg_to_data_url(&qr_svg);

                        view! {
                            <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
                                <h2 class="text-lg font-semibold text-fg">
                                    {"認証アプリで QR をスキャン"}
                                </h2>
                                <div class="border border-border rounded p-4 bg-surface-muted">
                                    <img src=qr_svg_data_url alt="認証アプリのQRコード" />
                                </div>
                                <p class="text-sm text-fg-muted break-all">
                                    {info.otpauth_url.clone()}
                                </p>
                                <form class="space-y-3" on:submit=move |ev| on_submit.call(ev)>
                                    <label class="block text-sm font-medium text-fg-muted">
                                        {"確認コード"}
                                    </label>
                                    <input
                                        type="text"
                                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                        placeholder="6桁コード"
                                        maxlength=6
                                        on:input=move |ev| {
                                            let target = event_target::<HtmlInputElement>(&ev);
                                            on_input.set(target.value());
                                        }
                                    />
                                    <button
                                        class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                                        disabled={move || activate_loading.get()}
                                    >
                                        {move || if activate_loading.get() {
                                            "有効化中..."
                                        } else {
                                            "有効化する"
                                        }}
                                    </button>
                                </form>
                            </div>
                        }
                        .into_view()
                    })
                    .unwrap_or_else(|| view! {}.into_view())
            }}
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::MfaSetupResponse;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn verification_section_renders_qr_info() {
        let html = render_to_string(move || {
            let setup = create_rw_signal(Some(MfaSetupResponse {
                secret: "secret".into(),
                otpauth_url: "otpauth://totp/test".into(),
            }));
            let (loading, _) = create_signal(false);
            let on_submit = Callback::new(|_| {});
            let on_input = create_rw_signal(String::new());
            view! {
                <VerificationSection
                    setup_info=setup.read_only()
                    activate_loading=loading.into()
                    on_submit=on_submit
                    on_input=on_input.write_only()
                />
            }
        });
        assert!(html.contains("認証アプリで QR をスキャン"));
    }
}
