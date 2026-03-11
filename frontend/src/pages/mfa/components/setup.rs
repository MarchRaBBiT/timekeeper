use crate::{api::MfaStatusResponse, components::layout::LoadingSpinner};
use leptos::*;

#[component]
pub fn SetupSection<F, R>(
    status: ReadSignal<Option<MfaStatusResponse>>,
    status_loading: ReadSignal<bool>,
    register_loading: Signal<bool>,
    on_register: F,
    on_refresh: R,
) -> impl IntoView
where
    F: Fn() + 'static,
    R: Fn() + 'static,
{
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-3">
            <h1 class="text-xl font-semibold text-fg">{rust_i18n::t!("pages.mfa.title")}</h1>

            <Show when=move || status_loading.get() fallback=|| ()>
                <div class="py-4">
                    <LoadingSpinner />
                </div>
            </Show>

            <Show when=move || status.get().is_some() fallback=|| ()>
                {move || {
                    status
                        .get()
                        .map(|info| {
                            let message = if info.enabled {
                                rust_i18n::t!("pages.mfa.status.enabled")
                            } else if info.pending {
                                rust_i18n::t!("pages.mfa.status.pending")
                            } else {
                                rust_i18n::t!("pages.mfa.status.disabled")
                            };
                            view! {
                                <div class="bg-surface-muted border border-border text-fg px-4 py-3 rounded">
                                    {message}
                                </div>
                            }
                            .into_view()
                        })
                        .unwrap_or_else(|| view! {}.into_view())
                }}
            </Show>

            <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <button
                    class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                    on:click=move |_| on_register()
                    disabled={move || register_loading.get()}
                >
                    {move || if register_loading.get() {
                        rust_i18n::t!("pages.mfa.actions.registering")
                    } else {
                        rust_i18n::t!("pages.mfa.actions.issue_secret")
                    }}
                </button>
                <button
                    class="px-4 py-2 border border-border text-fg rounded hover:bg-action-ghost-bg-hover"
                    on:click=move |_| on_refresh()
                >
                    {rust_i18n::t!("pages.mfa.actions.refresh_status")}
                </button>
            </div>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::MfaStatusResponse;
    use crate::test_support::helpers::set_test_locale;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn setup_section_renders_enabled_message() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            let status = create_rw_signal(Some(MfaStatusResponse {
                enabled: true,
                pending: false,
            }));
            let status_loading = create_rw_signal(false);
            let (register_loading, _) = create_signal(false);
            view! {
                <SetupSection
                    status=status.read_only()
                    status_loading=status_loading.read_only()
                    register_loading=register_loading.into()
                    on_register=move || {}
                    on_refresh=move || {}
                />
            }
        });
        assert!(html.contains("bg-surface-muted"));
    }

    #[test]
    fn setup_section_renders_pending_message() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            let status = create_rw_signal(Some(MfaStatusResponse {
                enabled: false,
                pending: true,
            }));
            let status_loading = create_rw_signal(false);
            let (register_loading, _) = create_signal(false);
            view! {
                <SetupSection
                    status=status.read_only()
                    status_loading=status_loading.read_only()
                    register_loading=register_loading.into()
                    on_register=move || {}
                    on_refresh=move || {}
                />
            }
        });
        assert!(html.contains("border-border"));
    }
}
