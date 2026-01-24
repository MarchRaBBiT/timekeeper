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
            <h1 class="text-xl font-semibold text-fg">{"MFA 設定"}</h1>

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
                                "MFA は有効です。必要に応じて再登録してください。"
                            } else if info.pending {
                                "認証アプリにシークレットを登録済みです。確認コードを入力して有効化してください。"
                            } else {
                                "まだ MFA は無効です。このボタンから登録できます。"
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
                        "登録中..."
                    } else {
                        "シークレットを発行"
                    }}
                </button>
                <button
                    class="px-4 py-2 border border-border text-fg rounded hover:bg-action-ghost-bg_hover"
                    on:click=move |_| on_refresh()
                >
                    {"ステータスを再取得"}
                </button>
            </div>
        </div>
    }
}
