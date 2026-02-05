use crate::{
    api::{ApiError, ArchivedUserResponse},
    components::error::InlineErrorMessage,
};
use leptos::*;

#[component]
pub fn ArchivedUserList(
    archived_users_resource: Resource<(bool, u32), Result<Vec<ArchivedUserResponse>, ApiError>>,
    loading: Signal<bool>,
    on_select: Callback<ArchivedUserResponse>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <Show
                when=move || loading.get()
                fallback=move || {
                    view! {
                        <Suspense fallback=move || view! { <p class="text-fg-muted">{"読み込み中..."}</p> }>
                            {move || {
                                archived_users_resource
                                    .get()
                                    .map(|result| match result {
                                        Ok(users) if users.is_empty() => {
                                            view! {
                                                <p class="text-fg-muted text-center py-8">
                                                    {"退職ユーザーはいません。"}
                                                </p>
                                            }
                                                .into_view()
                                        }
                                        Ok(users) => {
                                            view! {
                                                <ul class="divide-y divide-border border border-border rounded-lg">
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
                                                                class="px-4 py-3 hover:bg-surface-muted cursor-pointer flex items-center justify-between"
                                                                on:click=on_click
                                                            >
                                                                <div>
                                                                    <p class="font-medium text-fg">
                                                                        {user.full_name.clone()}
                                                                    </p>
                                                                    <p class="text-sm text-fg-muted">
                                                                        {format!("@{}", user.username)}
                                                                    </p>
                                                                </div>
                                                                <div class="text-right">
                                                                    <p class="text-xs text-fg-muted">{"退職日"}</p>
                                                                    <p class="text-sm text-fg-muted">
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
                                            let error_signal = create_rw_signal(Some(err));
                                            view! {
                                                <InlineErrorMessage error={error_signal.into()} />
                                            }
                                                .into_view()
                                        }
                                    })
                            }}
                        </Suspense>
                    }
                }
            >
                <p class="text-fg-muted">{"読み込み中..."}</p>
            </Show>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::ArchivedUserResponse;
    use crate::test_support::ssr::render_to_string;

    fn sample_user() -> ArchivedUserResponse {
        ArchivedUserResponse {
            id: "arch-1".into(),
            username: "archived".into(),
            full_name: "Archived User".into(),
            role: "member".into(),
            is_system_admin: false,
            archived_at: "2025-01-10T00:00:00Z".into(),
            archived_by: Some("admin".into()),
        }
    }

    #[test]
    fn archived_list_renders_empty_state() {
        let html = render_to_string(move || {
            let resource = Resource::new(|| (true, 0u32), |_| async move { Ok(Vec::new()) });
            resource.set(Ok(Vec::new()));
            let (loading, _) = create_signal(false);
            let on_select = Callback::new(|_: ArchivedUserResponse| {});
            view! {
                <ArchivedUserList
                    archived_users_resource=resource
                    loading=loading.into()
                    on_select=on_select
                />
            }
        });
        assert!(html.contains("退職ユーザーはいません。"));
    }

    #[test]
    fn archived_list_renders_rows() {
        let html = render_to_string(move || {
            let resource = Resource::new(|| (true, 0u32), |_| async move { Ok(Vec::new()) });
            resource.set(Ok(vec![sample_user()]));
            let (loading, _) = create_signal(false);
            let on_select = Callback::new(|_: ArchivedUserResponse| {});
            view! {
                <ArchivedUserList
                    archived_users_resource=resource
                    loading=loading.into()
                    on_select=on_select
                />
            }
        });
        assert!(html.contains("Archived User"));
        assert!(html.contains("退職日"));
    }

    #[test]
    fn archived_list_renders_error() {
        let html = render_to_string(move || {
            let resource = Resource::new(|| (true, 0u32), |_| async move { Ok(Vec::new()) });
            resource.set(Err(ApiError::unknown("boom")));
            let (loading, _) = create_signal(false);
            let on_select = Callback::new(|_: ArchivedUserResponse| {});
            view! {
                <ArchivedUserList
                    archived_users_resource=resource
                    loading=loading.into()
                    on_select=on_select
                />
            }
        });
        assert!(html.contains("boom"));
    }
}
