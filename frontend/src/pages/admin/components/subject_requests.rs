use crate::{
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        utils::SubjectRequestFilterState,
        view_model::SubjectRequestActionPayload,
    },
};
use chrono::DateTime;
use leptos::*;
use serde_json::to_string_pretty;

use crate::api::{
    ApiError, DataSubjectRequestResponse, DataSubjectRequestType, SubjectRequestListResponse,
};

#[component]
pub fn AdminSubjectRequestsSection(
    users: UsersResource,
    filter: SubjectRequestFilterState,
    resource: Resource<
        (
            bool,
            crate::pages::admin::utils::SubjectRequestFilterSnapshot,
            u32,
        ),
        Result<SubjectRequestListResponse, ApiError>,
    >,
    action: Action<SubjectRequestActionPayload, Result<(), ApiError>>,
    action_error: RwSignal<Option<ApiError>>,
    reload: RwSignal<u32>,
) -> impl IntoView {
    let modal_open = create_rw_signal(false);
    let modal_request = create_rw_signal(None::<DataSubjectRequestResponse>);
    let modal_comment = create_rw_signal(String::new());

    let modal_detail = Signal::derive(move || {
        modal_request
            .get()
            .and_then(|request| to_string_pretty(&request).ok())
            .unwrap_or_default()
    });
    let modal_pending = Signal::derive(move || {
        modal_request
            .get()
            .map(|request| request.status == "pending")
            .unwrap_or(false)
    });

    let loading = resource.loading();
    let data = Signal::derive(move || resource.get().and_then(|result| result.ok()));
    let error = Signal::derive(move || resource.get().and_then(|result| result.err()));

    let action_pending = action.pending();

    create_effect(move |_| {
        if let Some(result) = action.value().get() {
            match result {
                Ok(_) => {
                    modal_open.set(false);
                    modal_request.set(None);
                    modal_comment.set(String::new());
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

    let on_type_change = move |value: String| {
        filter.request_type_signal().set(value);
        filter.reset_page();
        trigger_reload();
    };

    let on_search = move |_| {
        filter.reset_page();
        trigger_reload();
    };

    let open_modal = Callback::new(move |request: DataSubjectRequestResponse| {
        modal_request.set(Some(request));
        modal_comment.set(String::new());
        modal_open.set(true);
        action_error.set(None);
    });

    let on_action = move |approve: bool| {
        let comment = modal_comment.get();
        if comment.trim().is_empty() {
            action_error.set(Some(ApiError::validation("コメントを入力してください。")));
            return;
        }
        let Some(request) = modal_request.get() else {
            action_error.set(Some(ApiError::validation(
                "申請情報を取得できませんでした。",
            )));
            return;
        };
        action.dispatch(SubjectRequestActionPayload {
            id: request.id,
            comment,
            approve,
        });
    };

    view! {
        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
            <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100">{"本人対応申請"}</h3>
            <div class="flex flex-col gap-3 lg:flex-row lg:flex-wrap lg:items-end">
                <select
                    class="w-full lg:w-auto border-gray-300 rounded-md px-2 py-1 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    on:change=move |ev| on_status_change(event_target_value(&ev))
                >
                    <option value="">{ "すべて" }</option>
                    <option value="pending">{ "承認待ち" }</option>
                    <option value="approved">{ "承認済み" }</option>
                    <option value="rejected">{ "却下" }</option>
                    <option value="cancelled">{ "取消" }</option>
                </select>
                <select
                    class="w-full lg:w-auto border-gray-300 rounded-md px-2 py-1 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    on:change=move |ev| on_type_change(event_target_value(&ev))
                >
                    <option value="">{ "請求種別" }</option>
                    <option value="access">{ "開示" }</option>
                    <option value="rectify">{ "訂正" }</option>
                    <option value="delete">{ "削除" }</option>
                    <option value="stop">{ "停止" }</option>
                </select>
                <div class="w-full lg:min-w-[220px] lg:flex-1">
                    <AdminUserSelect
                        users=users
                        selected=filter.user_id_signal()
                        label=Some("請求種別".into())
                        placeholder="全ユーザー".into()
                    />
                </div>
                <button
                    class="w-full lg:w-auto px-3 py-1 bg-blue-600 text-white rounded"
                    disabled={move || loading.get()}
                    on:click=on_search
                >
                    <span class="inline-flex items-center gap-2">
                        <Show when=move || loading.get()>
                            <span class="h-4 w-4 animate-spin rounded-full border-2 border-white/70 border-t-transparent"></span>
                        </Show>
                        {move || if loading.get() { "検索中..." } else { "検索" }}
                    </span>
                </button>
            </div>
            <Show when=move || error.get().is_some()>
                <ErrorMessage message={error.get().map(|err| err.to_string()).unwrap_or_default()} />
            </Show>
            <Show when=move || loading.get()>
                <div class="flex items-center gap-2 text-sm text-gray-600">
                    <LoadingSpinner />
                    <span>{"本人対応申請を読み込み中..."}</span>
                </div>
            </Show>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                    <thead class="bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">{"種別"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">{"ユーザー"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">{"ステータス"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">{"申請日"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-200 uppercase tracking-wider">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                        <Show when=move || data.get().is_some()>
                            {move || {
                                data.get()
                                    .map(|payload| {
                                        payload
                                            .items
                                            .into_iter()
                                            .map(|item| {
                                                let status_label = item.status.clone();
                                                let created_label = format_datetime(item.created_at);
                                                let type_label = type_label(&item.request_type);
                                                let id = item.id.clone();
                                                let open = {
                                                    let item = item.clone();
                                                    let open_modal = open_modal.clone();
                                                    move |_| open_modal.call(item.clone())
                                                };
                                                view! {
                                                    <tr>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">{type_label}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">{item.user_id}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap">
                                                            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-100">
                                                                {status_label}
                                                            </span>
                                                        </td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">{created_label}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                                                            <button class="text-blue-600" on:click=open>{"詳細"}</button>
                                                            <span class="sr-only">{id}</span>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default()
                            }}
                        </Show>
                    </tbody>
                </table>
            </div>
            <Show when=move || modal_open.get()>
                <div class="fixed inset-0 bg-black/30 dark:bg-black/80 flex items-center justify-center z-50">
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg w-full max-w-lg p-6">
                        <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">{"本人対応申請の詳細"}</h3>
                        <pre class="text-xs bg-gray-50 dark:bg-gray-900 dark:text-gray-300 p-2 rounded overflow-auto max-h-64 whitespace-pre-wrap">{move || modal_detail.get()}</pre>
                        <div class="mt-3">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">{"コメント"}</label>
                            <textarea
                                class="w-full border rounded px-2 py-1 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                on:input=move |ev| modal_comment.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <Show when=move || action_error.get().is_some()>
                            <ErrorMessage
                                message={action_error
                                    .get()
                                    .map(|err| err.to_string())
                                    .unwrap_or_default()}
                            />
                        </Show>
                        <div class="mt-4 flex justify-end space-x-2">
                            <button class="px-3 py-1 rounded border dark:border-gray-600 dark:text-gray-300" on:click=move |_| modal_open.set(false)>{"閉じる"}</button>
                            <button
                                class="px-3 py-1 rounded bg-red-600 text-white disabled:opacity-50"
                                disabled={move || action_pending.get() || !modal_pending.get()}
                                on:click=move |_| on_action(false)
                            >
                                {"却下"}
                            </button>
                            <button
                                class="px-3 py-1 rounded bg-green-600 text-white disabled:opacity-50"
                                disabled={move || action_pending.get() || !modal_pending.get()}
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

fn type_label(request_type: &DataSubjectRequestType) -> &'static str {
    match request_type {
        DataSubjectRequestType::Access => "開示",
        DataSubjectRequestType::Rectify => "訂正",
        DataSubjectRequestType::Delete => "削除",
        DataSubjectRequestType::Stop => "停止",
    }
}

fn format_datetime(value: DateTime<chrono::Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}
