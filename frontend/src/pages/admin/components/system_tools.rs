use crate::{
    api::ApiError,
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        repository::AdminRepository,
    },
};
use leptos::*;

#[component]
pub fn AdminMfaResetSection(
    repository: AdminRepository,
    system_admin_allowed: Memo<bool>,
    users: UsersResource,
) -> impl IntoView {
    let user_id = create_rw_signal(String::new());
    let error = create_rw_signal(None::<ApiError>);
    let success = create_rw_signal(None::<String>);
    let repo_for_reset = repository.clone();
    let reset_action = create_action(move |target: &String| {
        let repo = repo_for_reset.clone();
        let user_id = target.clone();
        async move {
            if user_id.trim().is_empty() {
                Err(ApiError::validation("ユーザーIDを入力してください。"))
            } else {
                repo.reset_mfa(&user_id).await
            }
        }
    });
    let pending = reset_action.pending();
    {
        create_effect(move |_| {
            if let Some(result) = reset_action.value().get() {
                match result {
                    Ok(_) => {
                        error.set(None);
                        success.set(Some("MFA をリセットしました。".into()));
                    }
                    Err(err) => {
                        success.set(None);
                        error.set(Some(err));
                    }
                }
            }
        });
    }

    let on_reset = {
        move |_| {
            if !system_admin_allowed.get_untracked() {
                return;
            }
            let value = user_id.get();
            if value.trim().is_empty() {
                error.set(Some(ApiError::validation("ユーザーIDを入力してください。")));
                success.set(None);
                return;
            }
            error.set(None);
            success.set(None);
            reset_action.dispatch(value);
        }
    };

    view! {
        <Show when=move || system_admin_allowed.get()>
            <div class="bg-surface-elevated shadow rounded-lg p-6">
                <h3 class="text-lg font-medium text-fg mb-4">{"MFA リセット"}</h3>
                <div class="flex flex-col gap-2">
                    <AdminUserSelect
                        users=users
                        selected=user_id
                        label=Some("対象ユーザー".into())
                        placeholder="ユーザーを選択してください".into()
                    />
                    <button
                        class="px-3 py-1 rounded bg-action-primary-bg text-action-primary-text disabled:opacity-50"
                        disabled={move || pending.get()}
                        on:click=on_reset
                    >
                        <span class="inline-flex items-center gap-2">
                            <Show when=move || pending.get()>
                                <span class="h-4 w-4 animate-spin rounded-full border-2 border-action-primary-text/70 border-t-transparent"></span>
                            </Show>
                            {move || if pending.get() { "リセット中..." } else { "MFA をリセット" }}
                        </span>
                    </button>
                    <Show when=move || error.get().is_some()>
                        <InlineErrorMessage error={error.into()} />
                    </Show>
                    <Show when=move || success.get().is_some()>
                        <SuccessMessage message={success.get().unwrap_or_default()} />
                    </Show>
                </div>
            </div>
        </Show>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::ApiClient;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn admin_mfa_reset_section_renders_when_allowed() {
        let html = render_to_string(move || {
            let api = ApiClient::new();
            let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let allowed = create_memo(|_| true);
            view! { <AdminMfaResetSection repository=repo system_admin_allowed=allowed users=users /> }
        });
        assert!(html.contains("MFA リセット"));
    }
}
