use crate::{
    api::{ApiClient, ApiError, BulkImportResponse, CreateDepartmentRequest, DepartmentResponse},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
    state::auth::use_auth,
};
use leptos::{ev, *};

fn is_system_admin_user(auth: &crate::state::auth::AuthState) -> bool {
    auth.user
        .as_ref()
        .map(|u| u.is_system_admin)
        .unwrap_or(false)
}

#[component]
pub fn DepartmentsPanel() -> impl IntoView {
    let (auth, _) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);

    let (departments, set_departments) = create_signal(Vec::<DepartmentResponse>::new());
    let (loading, set_loading) = create_signal(false);
    let (load_error, set_load_error) = create_signal(None::<ApiError>);
    let (new_name, set_new_name) = create_signal(String::new());
    let (create_error, set_create_error) = create_signal(None::<ApiError>);
    let (delete_error, set_delete_error) = create_signal(None::<ApiError>);

    let can_write = create_memo(move |_| is_system_admin_user(&auth.get()));

    // Load departments on mount
    let api_load = api.clone();
    let load_action = create_action(move |_: &()| {
        let api = api_load.clone();
        async move {
            set_loading.set(true);
            set_load_error.set(None);
            match api.admin_list_departments().await {
                Ok(depts) => set_departments.set(depts),
                Err(e) => set_load_error.set(Some(e)),
            }
            set_loading.set(false);
        }
    });
    load_action.dispatch(());

    // Create department action
    let api_create = api.clone();
    let dept_create_action = create_action(move |name: &String| {
        let api = api_create.clone();
        let name = name.clone();
        async move {
            let payload = CreateDepartmentRequest {
                name: name.trim().to_string(),
                parent_id: None,
            };
            api.admin_create_department(&payload).await
        }
    });

    let creating = dept_create_action.pending();

    create_effect(move |_| {
        if let Some(result) = dept_create_action.value().get() {
            match result {
                Ok(dept) => {
                    set_departments.update(|d| d.push(dept));
                    set_new_name.set(String::new());
                    set_create_error.set(None);
                }
                Err(e) => set_create_error.set(Some(e)),
            }
        }
    });

    let on_create = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        let name = new_name.get();
        if name.trim().is_empty() {
            set_create_error.set(Some(ApiError::validation("部署名を入力してください。")));
            return;
        }
        set_create_error.set(None);
        dept_create_action.dispatch(name);
    };

    // Delete department action
    let api_delete = api.clone();
    let dept_delete_action = create_action(move |dept_id: &String| {
        let api = api_delete.clone();
        let id = dept_id.clone();
        async move { api.admin_delete_department(&id).await.map(|_| id) }
    });

    create_effect(move |_| {
        if let Some(result) = dept_delete_action.value().get() {
            match result {
                Ok(id) => {
                    set_departments.update(|d| d.retain(|dep| dep.id != id));
                    set_delete_error.set(None);
                }
                Err(e) => set_delete_error.set(Some(e)),
            }
        }
    });

    let api_import = api.clone();
    let on_imported = move |_: ()| load_action.dispatch(());

    view! {
        <div class="space-y-6">
            <h2 class="text-lg font-semibold text-fg-default">{"部署管理"}</h2>

            // Create form (system_admin only)
            <Show when=move || can_write.get()>
                <form on:submit=on_create class="flex gap-2 items-end">
                    <div class="flex-1">
                        <label class="block text-sm font-medium text-fg-muted mb-1">
                            {"新規部署名"}
                        </label>
                        <input
                            type="text"
                            class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-1.5 text-sm"
                            placeholder="例: 営業部"
                            prop:value=new_name
                            on:input=move |ev| set_new_name.set(event_target_value(&ev))
                        />
                    </div>
                    <button
                        type="submit"
                        class="px-4 py-1.5 bg-action-primary-bg text-action-primary-text rounded text-sm disabled:opacity-50"
                        prop:disabled=creating
                    >
                        {move || if creating.get() { "作成中..." } else { "作成" }}
                    </button>
                </form>
                <Show when=move || create_error.get().is_some()>
                    <InlineErrorMessage error=Signal::derive(move || create_error.get()) />
                </Show>
            </Show>

            // Error display
            <Show when=move || load_error.get().is_some()>
                <InlineErrorMessage error=Signal::derive(move || load_error.get()) />
            </Show>
            <Show when=move || delete_error.get().is_some()>
                <InlineErrorMessage error=Signal::derive(move || delete_error.get()) />
            </Show>

            // Loading
            <Show when=move || loading.get()>
                <LoadingSpinner />
            </Show>

            // Department list
            <Show when=move || !loading.get()>
                <div class="overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="border-b border-border-default">
                                <th class="text-left py-2 pr-4 text-fg-muted font-medium">
                                    {"ID"}
                                </th>
                                <th class="text-left py-2 pr-4 text-fg-muted font-medium">
                                    {"部署名"}
                                </th>
                                <th class="text-left py-2 pr-4 text-fg-muted font-medium">
                                    {"親部署"}
                                </th>
                                <Show when=move || can_write.get()>
                                    <th class="text-left py-2 text-fg-muted font-medium">
                                        {"操作"}
                                    </th>
                                </Show>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || departments.get()
                                key=|dept| dept.id.clone()
                                children=move |dept| {
                                    let dept = store_value(dept);
                                    let dept_val = dept.get_value();
                                    let dept_id = dept_val.id.clone();
                                    let dept_id_display =
                                        dept_id[..8.min(dept_id.len())].to_string();
                                    let parent_label = dept_val
                                        .parent_id
                                        .clone()
                                        .map(|p| p[..8.min(p.len())].to_string() + "...")
                                        .unwrap_or_else(|| "—".to_string());
                                    view! {
                                        <tr class="border-b border-border-subtle hover:bg-surface-hover">
                                            <td class="py-2 pr-4 text-fg-muted font-mono text-xs">
                                                {dept_id_display}{"..."}
                                            </td>
                                            <td class="py-2 pr-4 text-fg-default">
                                                {dept_val.name.clone()}
                                            </td>
                                            <td class="py-2 pr-4 text-fg-muted text-xs">
                                                {parent_label}
                                            </td>
                                            <Show when=move || can_write.get()>
                                                <td class="py-2">
                                                    <button
                                                        class="text-xs text-danger-text hover:underline"
                                                        on:click=move |_| {
                                                            dept_delete_action
                                                                .dispatch(dept.get_value().id)
                                                        }
                                                    >
                                                        {"削除"}
                                                    </button>
                                                </td>
                                            </Show>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                    <Show when=move || departments.get().is_empty() && !loading.get()>
                        <p class="text-sm text-fg-muted py-4 text-center">
                            {"部署が登録されていません。"}
                        </p>
                    </Show>
                </div>
            </Show>

            // CSV Bulk Import section (system_admin only)
            <Show when=move || can_write.get()>
                <DepartmentImportSection api=api_import.clone() on_imported=on_imported />
            </Show>
        </div>
    }
}

#[component]
pub fn DepartmentImportSection(
    api: ApiClient,
    #[prop(into)] on_imported: Callback<()>,
) -> impl IntoView {
    let (csv_content, set_csv_content) = create_signal(None::<String>);
    let (import_result, set_import_result) = create_signal(None::<BulkImportResponse>);
    let (import_error, set_import_error) = create_signal(None::<ApiError>);

    let api_import = api.clone();
    let import_action = create_action(move |csv: &String| {
        let api = api_import.clone();
        let csv = csv.clone();
        async move { api.admin_bulk_import_departments(csv).await }
    });

    let importing = import_action.pending();

    create_effect(move |_| {
        if let Some(result) = import_action.value().get() {
            match result {
                Ok(resp) => {
                    set_import_error.set(None);
                    if resp.imported > 0 {
                        on_imported.call(());
                    }
                    set_import_result.set(Some(resp));
                }
                Err(e) => set_import_error.set(Some(e)),
            }
        }
    });

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
            set_import_result.set(None);
            set_import_error.set(None);
            import_action.dispatch(csv);
        }
    };

    view! {
        <div class="border border-border-subtle rounded p-4 space-y-3">
            <h3 class="text-sm font-semibold text-fg-default">{"部署 CSV 一括インポート"}</h3>
            <p class="text-xs text-fg-muted">
                {"CSV ヘッダー: "}
                <code class="font-mono bg-surface-raised px-1 rounded">{"name,parent_name"}</code>
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
                    {move || if importing.get() { "インポート中..." } else { "インポート実行" }}
                </button>
            </form>

            // Import error
            <Show when=move || import_error.get().is_some()>
                <InlineErrorMessage error=Signal::derive(move || import_error.get()) />
            </Show>

            // Import result
            <Show when=move || import_result.get().is_some()>
                {move || {
                    let r = import_result.get().unwrap();
                    if r.errors.is_empty() {
                        view! {
                            <p class="text-sm text-success-text">
                                {format!("{} 件インポートしました。", r.imported)}
                            </p>
                        }
                        .into_view()
                    } else {
                        view! {
                            <div class="space-y-1">
                                <p class="text-sm text-warning-text">
                                    {format!(
                                        "バリデーションエラー {} 件。インポートは実行されませんでした。",
                                        r.failed
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
                                                each=move || r.errors.clone()
                                                key=|e| e.row
                                                children=move |e| {
                                                    view! {
                                                        <tr class="border-b border-border-subtle">
                                                            <td class="px-2 py-1 font-mono">
                                                                {e.row}
                                                            </td>
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
                        .into_view()
                    }
                }}
            </Show>
        </div>
    }
}
