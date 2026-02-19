use leptos::ev::KeyboardEvent;
use leptos::*;

#[component]
pub fn ConfirmDialog(
    is_open: Signal<bool>,
    #[prop(into)] title: MaybeSignal<String>,
    #[prop(into)] message: MaybeSignal<String>,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional, into)] confirm_label: MaybeSignal<String>,
    #[prop(optional, into)] cancel_label: MaybeSignal<String>,
    #[prop(optional, into)] confirm_disabled: MaybeSignal<bool>,
    #[prop(optional)] destructive: bool,
) -> impl IntoView {
    let confirm_button_class = if destructive {
        "inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-semibold bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50"
    } else {
        "inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-semibold bg-action-primary-bg text-action-primary-text hover:bg-action-primary-bg-hover disabled:opacity-50"
    };

    let confirm_label_text = Signal::derive(move || {
        let text = confirm_label.get();
        if text.trim().is_empty() {
            "はい".to_string()
        } else {
            text
        }
    });
    let title_text = Signal::derive(move || title.get());
    let message_text = Signal::derive(move || message.get());
    let cancel_label_text = Signal::derive(move || {
        let text = cancel_label.get();
        if text.trim().is_empty() {
            "いいえ".to_string()
        } else {
            text
        }
    });

    let cancel_on_backdrop = on_cancel;
    let cancel_on_header_button = on_cancel;
    let cancel_on_esc = on_cancel;
    let cancel_on_footer_button = on_cancel;
    let confirm_on_footer_button = on_confirm;

    view! {
        <Show when=move || is_open.get()>
            <div class="fixed inset-0 z-[70] flex items-center justify-center p-4">
                <button
                    type="button"
                    aria-label="閉じる"
                    class="absolute inset-0 bg-overlay-backdrop"
                    on:click=move |_| cancel_on_backdrop.call(())
                ></button>
                <div
                    class="relative z-[71] w-full max-w-md rounded-lg bg-surface-elevated shadow-xl border border-border p-6 space-y-4"
                    role="dialog"
                    aria-modal="true"
                    tabindex="-1"
                    on:keydown=move |ev: KeyboardEvent| {
                        if ev.key() == "Escape" {
                            ev.prevent_default();
                            cancel_on_esc.call(());
                        }
                    }
                >
                    <div class="flex items-start justify-between gap-3">
                        <h2 class="text-lg font-semibold text-fg">{move || title_text.get()}</h2>
                        <button
                            type="button"
                            aria-label="閉じる"
                            class="text-fg-muted hover:text-fg"
                            on:click=move |_| cancel_on_header_button.call(())
                        >
                            {"✕"}
                        </button>
                    </div>
                    <p class="text-sm text-fg-muted">{move || message_text.get()}</p>
                    <div class="flex justify-end gap-2">
                        <button
                            type="button"
                            class="inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-semibold bg-surface-muted text-fg hover:bg-surface-elevated"
                            on:click=move |_| cancel_on_footer_button.call(())
                        >
                            {move || cancel_label_text.get()}
                        </button>
                        <button
                            type="button"
                            class=confirm_button_class
                            disabled=move || confirm_disabled.get()
                            on:click=move |_| confirm_on_footer_button.call(())
                        >
                            {move || confirm_label_text.get()}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn confirm_dialog_renders_with_default_labels() {
        let html = render_to_string(move || {
            let is_open = Signal::derive(|| true);
            view! {
                <ConfirmDialog
                    is_open=is_open
                    title="確認"
                    message="本当に実行しますか？"
                    on_confirm=Callback::new(|_| {})
                    on_cancel=Callback::new(|_| {})
                />
            }
        });
        assert!(html.contains("role=\"dialog\""));
        assert!(html.contains("aria-modal=\"true\""));
        assert!(html.contains("本当に実行しますか？"));
        assert!(html.contains("はい"));
        assert!(html.contains("いいえ"));
    }
}
