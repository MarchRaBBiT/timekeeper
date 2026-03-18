use crate::{
    api::{ApiError, BulkImportResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin_users::utils::MessageState,
};
use leptos::{ev, *};

#[component]
pub fn UserImportSection(
    import_action: Action<String, Result<BulkImportResponse, ApiError>>,
    messages: MessageState,
) -> impl IntoView {
    let (csv_content, set_csv_content) = create_signal(None::<String>);

    let importing = import_action.pending();

    let on_file_change = move |_ev: ev::Event| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;

            let input = _ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
            if let Some(input) = input {
                if let Some(files) = input.files() {
                    if let Some(file) = files.item(0) {
                        let set_csv = set_csv_content;
                        spawn_local(async move {
                            if let Ok(text) = JsFuture::from(file.text()).await {
                                let s = text.as_string().unwrap_or_default();
                                set_csv.set(Some(s));
                            }
                        });
                    }
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = set_csv_content;
        let _ = _ev;
    };

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(csv) = csv_content.get() {
            messages.clear();
            import_action.dispatch(csv);
        }
    };

    view! {
        <div class="border border-border-subtle rounded p-4 space-y-3">
            <h3 class="text-sm font-semibold text-fg-default">
                {"ユーザー CSV 一括インポート"}
            </h3>
            <p class="text-xs text-fg-muted">
                {"CSV ヘッダー: "}
                <code class="font-mono bg-surface-raised px-1 rounded">
                    {"username,password,full_name,email,role,is_system_admin,department_name"}
                </code>
            </p>
            <form on:submit=on_submit class="space-y-2">
                <input
                    type="file"
                    accept=".csv"
                    class="block text-sm text-fg-default"
                    on:change=on_file_change
                />
                // Preview first 5 lines
                <Show when=move || csv_content.get().is_some()>
                    {move || {
                        let preview: Vec<String> = csv_content
                            .get()
                            .unwrap_or_default()
                            .lines()
                            .take(5)
                            .map(|s| s.to_string())
                            .collect();
                        view! {
                            <div class="overflow-x-auto">
                                <table class="text-xs w-full border border-border-subtle">
                                    <For
                                        each=move || preview.clone().into_iter().enumerate()
                                        key=|(i, _)| *i
                                        children=move |(_, line)| {
                                            view! {
                                                <tr class="border-b border-border-subtle">
                                                    <td class="px-2 py-1 font-mono">{line}</td>
                                                </tr>
                                            }
                                        }
                                    />
                                </table>
                            </div>
                        }
                    }}
                </Show>
                <button
                    type="submit"
                    class="px-4 py-1.5 bg-action-primary-bg text-action-primary-text rounded text-sm disabled:opacity-50"
                    prop:disabled=move || importing.get() || csv_content.get().is_none()
                >
                    {move || {
                        if importing.get() {
                            "インポート中..."
                        } else {
                            "インポート実行"
                        }
                    }}
                </button>
            </form>

            // Global API error
            <Show when=move || messages.error.get().is_some()>
                <InlineErrorMessage error=Signal::derive(move || messages.error.get()) />
            </Show>
            <Show when=move || messages.success.get().is_some()>
                <SuccessMessage message={messages.success.get().unwrap_or_default()} />
            </Show>

            // Row-level validation errors from the last import attempt
            <Show when=move || {
                import_action
                    .value()
                    .get()
                    .and_then(|r| r.ok())
                    .map(|r| !r.errors.is_empty())
                    .unwrap_or(false)
            }>
                {move || {
                    let result = import_action
                        .value()
                        .get()
                        .and_then(|r| r.ok())
                        .unwrap_or(BulkImportResponse {
                            imported: 0,
                            failed: 0,
                            errors: Vec::new(),
                        });
                    view! {
                        <div class="space-y-1">
                            <p class="text-sm text-warning-text">
                                {format!(
                                    "バリデーションエラー {} 件。インポートは実行されませんでした。",
                                    result.failed
                                )}
                            </p>
                            <div class="overflow-x-auto">
                                <table class="text-xs w-full border border-border-subtle">
                                    <thead>
                                        <tr class="border-b border-border-default">
                                            <th class="text-left px-2 py-1 text-fg-muted">
                                                {"行"}
                                            </th>
                                            <th class="text-left px-2 py-1 text-fg-muted">
                                                {"エラー内容"}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || result.errors.clone()
                                            key=|e| e.row
                                            children=move |e| {
                                                view! {
                                                    <tr class="border-b border-border-subtle">
                                                        <td class="px-2 py-1 font-mono">{e.row}</td>
                                                        <td class="px-2 py-1 text-danger-text">
                                                            {e.message.clone()}
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        />
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}
