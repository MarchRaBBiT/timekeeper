use crate::{
    api::{ApiError, UserResponse},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
};
use leptos::{ev, *};

pub type UsersResource = Resource<bool, Result<Vec<UserResponse>, ApiError>>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum UserSelectValue {
    #[default]
    Id,
    Username,
}

#[component]
pub fn AdminUserSelect(
    users: UsersResource,
    selected: RwSignal<String>,
    #[prop(optional_no_strip)] label: Option<String>,
    #[prop(default = "ユーザーを選択".to_string())] placeholder: String,
    #[prop(default = UserSelectValue::Id)] value_kind: UserSelectValue,
    #[prop(into, default = MaybeSignal::Static(false))] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let fetch_error = create_rw_signal(None::<ApiError>);
    let loading = users.loading();

    {
        create_effect(move |_| {
            if let Some(result) = users.get() {
                match result {
                    Ok(_) => fetch_error.set(None),
                    Err(err) => fetch_error.set(Some(err)),
                }
            }
        });
    }

    let on_change = {
        move |ev: ev::Event| {
            selected.set(event_target_value(&ev));
        }
    };

    let has_label = label.as_ref().map(|l| !l.is_empty()).unwrap_or(false);
    let label_value = label.unwrap_or_default();

    let on_retry = {
        move |_| {
            fetch_error.set(None);
            users.refetch();
        }
    };

    let options_view = move || {
        if loading.get() {
            return view! { <option value="" disabled>{"ユーザーを読み込み中..."}</option> }
                .into_view();
        }
        match users.get() {
            None => {
                view! { <option value="" disabled>{"ユーザーを読み込み中..."}</option> }.into_view()
            }
            Some(Err(_)) => {
                view! { <option value="" disabled>{"ユーザーの取得に失敗しました"}</option> }
                    .into_view()
            }
            Some(Ok(list)) if list.is_empty() => {
                view! { <option value="" disabled>{"ユーザーが0件です"}</option> }.into_view()
            }
            Some(Ok(list)) => {
                let mut sorted = list.clone();
                sorted.sort_by(|left, right| {
                    let name_cmp = left.full_name.cmp(&right.full_name);
                    if name_cmp == std::cmp::Ordering::Equal {
                        left.username.cmp(&right.username)
                    } else {
                        name_cmp
                    }
                });
                view! {
                    <For
                        each=move || sorted.clone()
                        key=|user| user.id.clone()
                        children=move |user| {
                            let label = format!("{} ({})", user.full_name, user.username);
                            let value = match value_kind {
                                UserSelectValue::Id => user.id.clone(),
                                UserSelectValue::Username => user.username.clone(),
                        };
                        view! { <option value=value>{label}</option> }
                    }
                    />
                }
                .into_view()
            }
        }
    };

    view! {
        <div class="space-y-1">
            <Show when=move || has_label>
                <label class="block text-sm font-medium text-fg-muted">
                    {label_value.clone()}
                </label>
            </Show>
            <select
                class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 disabled:opacity-50"
                on:change=on_change
                prop:value=selected
                disabled=disabled
            >
                <option value="">{placeholder.clone()}</option>
                {move || options_view()}
            </select>
            <Show when=move || fetch_error.get().is_some()>
                <div class="flex items-center gap-2">
                    <InlineErrorMessage error={fetch_error.into()} />
                    <button
                        class="text-sm text-link hover:text-link-hover hover:underline disabled:opacity-50"
                        on:click=on_retry
                        disabled=move || loading.get()
                    >
                        {"再試行"}
                    </button>
                    <Show when=move || loading.get()>
                        <LoadingSpinner />
                    </Show>
                </div>
            </Show>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::UserResponse;
    use crate::test_support::ssr::render_to_string;

    fn user(id: &str, name: &str, username: &str) -> UserResponse {
        UserResponse {
            id: id.into(),
            username: username.into(),
            full_name: name.into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
        }
    }

    #[test]
    fn user_select_renders_empty_state() {
        let html = render_to_string(move || {
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            users.set(Ok(Vec::new()));
            let selected = create_rw_signal(String::new());
            view! { <AdminUserSelect users=users selected=selected /> }
        });
        assert!(html.contains("ユーザーが0件です"));
    }

    #[test]
    fn user_select_renders_error_state() {
        let html = render_to_string(move || {
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            users.set(Err(ApiError::validation("fetch error")));
            let selected = create_rw_signal(String::new());
            view! { <AdminUserSelect users=users selected=selected /> }
        });
        assert!(html.contains("ユーザーの取得に失敗しました"));
    }

    #[test]
    fn user_select_renders_options() {
        let html = render_to_string(move || {
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            users.set(Ok(vec![
                user("u1", "Alice", "alice"),
                user("u2", "Bob", "bob"),
            ]));
            let selected = create_rw_signal(String::new());
            view! { <AdminUserSelect users=users selected=selected /> }
        });
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
    }
}
