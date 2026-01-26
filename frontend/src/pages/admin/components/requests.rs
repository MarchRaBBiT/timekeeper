use crate::{
    api::ApiError,
    components::{empty_state::EmptyState, error::InlineErrorMessage, layout::LoadingSpinner},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        utils::RequestFilterState,
        view_model::RequestActionPayload,
    },
};
use leptos::*;
use serde_json::{json, Value};

#[component]
pub fn AdminRequestsSection(
    users: UsersResource,
    filter: RequestFilterState,
    resource: Resource<
        (bool, crate::pages::admin::utils::RequestFilterSnapshot, u32),
        Result<serde_json::Value, ApiError>,
    >,
    action: Action<RequestActionPayload, Result<(), ApiError>>,
    action_error: RwSignal<Option<ApiError>>,
    reload: RwSignal<u32>,
) -> impl IntoView {
    let modal_open = create_rw_signal(false);
    let modal_data = create_rw_signal(Value::Null);
    let modal_comment = create_rw_signal(String::new());

    let requests_loading = resource.loading();
    let requests_data = Signal::derive(move || {
        resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or(Value::Null)
    });
    let requests_error = Signal::derive(move || resource.get().and_then(|result| result.err()));

    let action_pending = action.pending();

    // Effects
    create_effect(move |_| {
        if let Some(result) = action.value().get() {
            match result {
                Ok(_) => {
                    modal_open.set(false);
                    action_error.set(None);
                    reload.update(|value| *value = value.wrapping_add(1));
                }
                Err(err) => action_error.set(Some(err)),
            }
        }
    });

    let trigger_reload = move || reload.update(|value| *value = value.wrapping_add(1));

    let on_status_change = move |value: String| {
        filter.status_signal().set(value);
        filter.reset_page();
        trigger_reload();
    };

    let on_search = move |_| {
        filter.reset_page();
        trigger_reload();
    };

    let open_modal = move |data: Value| {
        modal_data.set(data);
        modal_comment.set(String::new());
        modal_open.set(true);
    };

    let on_action = move |approve: bool| {
        let id = modal_data
            .get()
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let comment = modal_comment.get();
        action.dispatch(RequestActionPayload {
            id,
            comment,
            approve,
        });
    };

    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-fg">{"申請一覧"}</h3>
            <div class="flex flex-col gap-3 lg:flex-row lg:flex-wrap lg:items-end">
                <select
                    class="w-full lg:w-auto border border-form-control-border bg-form-control-bg text-form-control-text rounded-md px-2 py-1"
                    on:change=move |ev| on_status_change(event_target_value(&ev))
                >
                    <option value="">{ "すべて" }</option>
                    <option value="pending">{ "承認待ち" }</option>
                    <option value="approved">{ "承認済み" }</option>
                    <option value="rejected">{ "却下" }</option>
                    <option value="cancelled">{ "取消" }</option>
                </select>
                <div class="w-full lg:min-w-[220px] lg:flex-1">
                    <AdminUserSelect
                        users=users
                        selected=filter.user_id_signal()
                        label=Some("ユーザー".into())
                        placeholder="全ユーザー".into()
                    />
                </div>
                <button
                    class="w-full lg:w-auto px-3 py-1 bg-action-primary-bg text-action-primary-text rounded"
                    disabled={move || requests_loading.get()}
                    on:click=on_search
                >
                    <span class="inline-flex items-center gap-2">
                        <Show when=move || requests_loading.get()>
                            <span class="h-4 w-4 animate-spin rounded-full border-2 border-action-primary-text/70 border-t-transparent"></span>
                        </Show>
                        {move || if requests_loading.get() { "検索中..." } else { "検索" }}
                    </span>
                </button>
            </div>
            <Show when=move || requests_error.get().is_some()>
                <InlineErrorMessage error={requests_error} />
            </Show>
            <Show when=move || requests_loading.get()>
                <div class="flex items-center gap-2 text-sm text-fg-muted">
                    <LoadingSpinner />
                    <span>{"申請情報を読み込み中..."}</span>
                </div>
            </Show>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-border">
                    <thead class="bg-surface-muted">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"種別"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"対象"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"ユーザー"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"ステータス"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-surface-elevated divide-y divide-border">
                        <Show when=move || requests_data.get().is_object()>
                            {let data = requests_data.get();
                                let leaves = data.get("leave_requests").cloned().unwrap_or(json!([]));
                                let ots = data.get("overtime_requests").cloned().unwrap_or(json!([]));
                                let mut rows: Vec<Value> = vec![];
                                if let Some(arr) = leaves.as_array() {
                                    for r in arr {
                                        rows.push(json!({"kind":"leave","data": r}));
                                    }
                                }
                                if let Some(arr) = ots.as_array() {
                                    for r in arr {
                                        rows.push(json!({"kind":"overtime","data": r}));
                                    }
                                }
                                if rows.is_empty() {
                                    view! {
                                        <tr>
                                            <td colspan="5" class="p-4 bg-surface-muted">
                                                <EmptyState
                                                    title="申請がありません"
                                                    description="表示できる申請データが見つかりませんでした。"
                                                />
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else {
                                    view! { <>
                                        {rows.into_iter().map(|row| {
                                        let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let data = row.get("data").cloned().unwrap_or(json!({}));
                                        let statusv = data.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let user = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let target = if kind == "leave" {
                                            format!(
                                                "{} - {}",
                                                data.get("start_date").and_then(|v| v.as_str()).unwrap_or(""),
                                                data.get("end_date").and_then(|v| v.as_str()).unwrap_or("")
                                            )
                                        } else {
                                            data.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string()
                                        };
                                        let open = {
                                            let data = data.clone();
                                            move |_| open_modal(data.clone())
                                        };
                                        let kind_label = if kind == "leave" { "休暇" } else { "残業" };
                                        view! {
                                            <tr>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{kind_label}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{target.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-fg">{user.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-status-neutral-bg text-status-neutral-text">
                                                        {statusv.clone()}
                                                    </span>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                    <button class="text-link hover:text-link-hover" on:click=open>{"詳細"}</button>
                                                </td>
                                            </tr>
                                        }
                                        }).collect::<Vec<_>>()}
                                    </> }.into_view()
                                }
                            }
                        </Show>
                    </tbody>
                </table>
            </div>
            <Show when=move || modal_open.get()>
                <div class="fixed inset-0 bg-overlay-backdrop flex items-center justify-center z-50">
                    <div class="bg-surface-elevated rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-fg mb-2">{"申請詳細"}</h3>
                        <pre class="text-xs bg-surface-muted text-fg p-2 rounded overflow-auto max-h-64">{format!("{}", modal_data.get())}</pre>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-fg-muted">{"コメント（任意）"}</label>
                            <textarea
                                class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                                on:input=move |ev| modal_comment.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <Show when=move || action_error.get().is_some()>
                            <InlineErrorMessage error={action_error.into()} />
                        </Show>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button class="px-3 py-1 rounded border border-border text-fg hover:bg-action-ghost-bg-hover" on:click=move |_| modal_open.set(false)>{"閉じる"}</button>
                            <button
                                class="px-3 py-1 rounded bg-action-danger-bg text-action-danger-text disabled:opacity-50"
                                disabled={move || action_pending.get()}
                                on:click=move |_| on_action(false)
                            >
                                {"却下"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                                disabled={move || action_pending.get()}
                                on:click=move |_| on_action(true)
                            >
                                {"承認"}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
