use crate::{
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        repository::AdminRepository,
        utils::RequestFilterState,
    },
};
use leptos::*;
use serde_json::{json, Value};

#[derive(Clone)]
struct RequestActionPayload {
    id: String,
    comment: String,
    approve: bool,
}

#[component]
pub fn AdminRequestsSection(
    repository: AdminRepository,
    admin_allowed: Memo<bool>,
    users: UsersResource,
) -> impl IntoView {
    let filter_state = RequestFilterState::new();
    let reload = create_rw_signal(0u32);
    let modal_open = create_rw_signal(false);
    let modal_data = create_rw_signal(Value::Null);
    let modal_comment = create_rw_signal(String::new());
    let action_error = create_rw_signal(None::<String>);

    let filter_state_for_snapshot = filter_state.clone();
    let snapshot = Signal::derive(move || filter_state_for_snapshot.snapshot());
    let repo_for_requests = repository.clone();
    let requests_resource = create_resource(
        move || (admin_allowed.get(), snapshot.get(), reload.get()),
        move |(allowed, snapshot, _)| {
            let repo = repo_for_requests.clone();
            async move {
                if !allowed {
                    Ok(Value::Null)
                } else {
                    repo.list_requests(
                        snapshot.status.clone(),
                        snapshot.user_id.clone(),
                        snapshot.page,
                        snapshot.per_page,
                    )
                    .await
                }
            }
        },
    );
    let requests_loading = requests_resource.loading();
    let requests_data = Signal::derive(move || {
        requests_resource
            .get()
            .and_then(|result| result.ok())
            .unwrap_or(Value::Null)
    });
    let requests_error =
        Signal::derive(move || requests_resource.get().and_then(|result| result.err()));

    let repo_for_action = repository.clone();
    let request_action = create_action(move |payload: &RequestActionPayload| {
        let repo = repo_for_action.clone();
        let payload = payload.clone();
        async move {
            if payload.id.trim().is_empty() {
                Err("リクエストIDを取得できませんでした。".into())
            } else if payload.approve {
                repo.approve_request(&payload.id, &payload.comment).await
            } else {
                repo.reject_request(&payload.id, &payload.comment).await
            }
        }
    });
    let action_pending = request_action.pending();
    {
        let modal_open = modal_open.clone();
        let action_error = action_error.clone();
        let reload = reload.clone();
        create_effect(move |_| {
            if let Some(result) = request_action.value().get() {
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
    }

    let trigger_reload = {
        let reload = reload.clone();
        move || reload.update(|value| *value = value.wrapping_add(1))
    };

    let on_status_change = {
        let filter_state = filter_state.clone();
        let trigger_reload = trigger_reload.clone();
        move |value: String| {
            filter_state.status_signal().set(value);
            filter_state.reset_page();
            trigger_reload();
        }
    };

    let on_search = {
        let filter_state = filter_state.clone();
        let trigger_reload = trigger_reload.clone();
        move |_| {
            filter_state.reset_page();
            trigger_reload();
        }
    };

    let open_modal = {
        let modal_open = modal_open.clone();
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        move |data: Value| {
            modal_data.set(data);
            modal_comment.set(String::new());
            modal_open.set(true);
        }
    };

    let on_action = {
        let modal_data = modal_data.clone();
        let modal_comment = modal_comment.clone();
        let request_action = request_action.clone();
        move |approve: bool| {
            let id = modal_data
                .get()
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let comment = modal_comment.get();
            request_action.dispatch(RequestActionPayload {
                id,
                comment,
                approve,
            });
        }
    };

    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-gray-900">{"申請一覧"}</h3>
            <div class="flex flex-wrap gap-3">
                <select
                    class="border-gray-300 rounded-md px-2 py-1"
                    on:change=move |ev| on_status_change(event_target_value(&ev))
                >
                    <option value="">{ "すべて" }</option>
                    <option value="pending">{ "承認待ち" }</option>
                    <option value="approved">{ "承認済み" }</option>
                    <option value="rejected">{ "却下" }</option>
                    <option value="cancelled">{ "取消" }</option>
                </select>
                <div class="min-w-[220px] flex-1">
                    <AdminUserSelect
                        users=users
                        selected=filter_state.user_id_signal()
                        label=Some("ユーザー".into())
                        placeholder="全ユーザー".into()
                    />
                </div>
                <button
                    class="px-3 py-1 bg-blue-600 text-white rounded disabled:opacity-50"
                    disabled={move || requests_loading.get()}
                    on:click=on_search
                >
                    <span class="inline-flex items-center gap-2">
                        <Show when=move || requests_loading.get()>
                            <span class="h-4 w-4 animate-spin rounded-full border-2 border-white/70 border-t-transparent"></span>
                        </Show>
                        {move || if requests_loading.get() { "検索中..." } else { "検索" }}
                    </span>
                </button>
            </div>
            <Show when=move || requests_error.get().is_some()>
                <ErrorMessage message={requests_error.get().unwrap_or_default()} />
            </Show>
            <Show when=move || requests_loading.get()>
                <div class="flex items-center gap-2 text-sm text-gray-600">
                    <LoadingSpinner />
                    <span>{"申請情報を読み込み中..."}</span>
                </div>
            </Show>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"種別"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"対象"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"ユーザー"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"ステータス"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200">
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
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{kind_label}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{target.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{user.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800">
                                                        {statusv.clone()}
                                                    </span>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                    <button class="text-blue-600" on:click=open>{"詳細"}</button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </> }
                            }
                        </Show>
                    </tbody>
                </table>
            </div>
            <Show when=move || modal_open.get()>
                <div class="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 mb-2">{"申請詳細"}</h3>
                        <pre class="text-xs bg-gray-50 p-2 rounded overflow-auto max-h-64">{format!("{}", modal_data.get())}</pre>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-gray-700">{"コメント（任意）"}</label>
                            <textarea
                                class="w-full border rounded px-2 py-1"
                                on:input=move |ev| modal_comment.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <Show when=move || action_error.get().is_some()>
                            <ErrorMessage message={action_error.get().unwrap_or_default()} />
                        </Show>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button class="px-3 py-1 rounded border" on:click=move |_| modal_open.set(false)>{"閉じる"}</button>
                            <button
                                class="px-3 py-1 rounded bg-red-600 text-white disabled:opacity-50"
                                disabled={move || action_pending.get()}
                                on:click=move |_| on_action(false)
                            >
                                {"却下"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-green-600 text-white disabled:opacity-50"
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
