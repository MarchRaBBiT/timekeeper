use crate::api::ArchivedUserResponse;
use leptos::*;

#[component]
pub fn ArchivedUserList(
    archived_users_resource: Resource<(bool, u32), Result<Vec<ArchivedUserResponse>, String>>,
    loading: Signal<bool>,
    on_select: Callback<ArchivedUserResponse>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <Show
                when=move || loading.get()
                fallback=move || {
                    view! {
                        <Suspense fallback=move || view! { <p class="text-gray-500">{"読み込み中..."}</p> }>
                            {move || {
                                archived_users_resource
                                    .get()
                                    .map(|result| match result {
                                        Ok(users) if users.is_empty() => {
                                            view! {
                                                <p class="text-gray-500 text-center py-8">
                                                    {"退職ユーザーはいません。"}
                                                </p>
                                            }
                                                .into_view()
                                        }
                                        Ok(users) => {
                                            view! {
                                                <ul class="divide-y divide-gray-200 border rounded-lg">
                                                    <For
                                                        each=move || users.clone()
                                                        key=|user| user.id.clone()
                                                        children=move |user| {
                                                            let on_click = {
                                                                let user = user.clone();
                                                                move |_| on_select.call(user.clone())
                                                            };
                                                            view! {
                                                                <li
                                                                    class="px-4 py-3 hover:bg-gray-50 cursor-pointer flex items-center justify-between"
                                                                    on:click=on_click
                                                                >
                                                                    <div>
                                                                        <p class="font-medium text-gray-900">
                                                                            {user.full_name.clone()}
                                                                        </p>
                                                                        <p class="text-sm text-gray-500">
                                                                            {format!("@{}", user.username)}
                                                                        </p>
                                                                    </div>
                                                                    <div class="text-right">
                                                                        <p class="text-xs text-gray-400">{"退職日"}</p>
                                                                        <p class="text-sm text-gray-600">
                                                                            {user.archived_at.split('T').next().unwrap_or(&user.archived_at).to_string()}
                                                                        </p>
                                                                    </div>
                                                                </li>
                                                            }
                                                        }
                                                    />
                                                </ul>
                                            }
                                                .into_view()
                                        }
                                        Err(err) => {
                                            view! {
                                                <p class="text-red-500">{format!("エラー: {}", err)}</p>
                                            }
                                                .into_view()
                                        }
                                    })
                            }}
                        </Suspense>
                    }
                }
            >
                <p class="text-gray-500">{"読み込み中..."}</p>
            </Show>
        </div>
    }
}
