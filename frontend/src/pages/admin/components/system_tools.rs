use crate::{
    components::layout::{ErrorMessage, SuccessMessage},
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
    let error = create_rw_signal(None::<String>);
    let success = create_rw_signal(None::<String>);
    let repo_for_reset = repository.clone();
    let reset_action = create_action(move |target: &String| {
        let repo = repo_for_reset.clone();
        let user_id = target.clone();
        async move {
            if user_id.trim().is_empty() {
                Err("ユーザーIDを入力してください。".into())
            } else {
                repo.reset_mfa(&user_id).await
            }
        }
    });
    let pending = reset_action.pending();
    {
        let error = error.clone();
        let success = success.clone();
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
        let user_id = user_id.clone();
        let error = error.clone();
        let success = success.clone();
        let reset_action = reset_action.clone();
        move |_| {
            if !system_admin_allowed.get_untracked() {
                return;
            }
            let value = user_id.get();
            if value.trim().is_empty() {
                error.set(Some("ユーザーIDを入力してください。".into()));
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
            <div class="bg-white shadow rounded-lg p-6">
                <h3 class="text-lg font-medium text-gray-900 mb-4">{"MFA リセット"}</h3>
                <div class="flex flex-col gap-2">
                    <AdminUserSelect
                        users=users
                        selected=user_id
                        label=Some("対象ユーザー".into())
                        placeholder="ユーザーを選択してください".into()
                    />
                    <button
                        class="px-3 py-1 rounded bg-indigo-600 text-white disabled:opacity-50"
                        disabled={move || pending.get()}
                        on:click=on_reset
                    >
                        <span class="inline-flex items-center gap-2">
                            <Show when=move || pending.get()>
                                <span class="h-4 w-4 animate-spin rounded-full border-2 border-white/70 border-t-transparent"></span>
                            </Show>
                            {move || if pending.get() { "リセット中..." } else { "MFA をリセット" }}
                        </span>
                    </button>
                    <Show when=move || error.get().is_some()>
                        <ErrorMessage message={error.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || success.get().is_some()>
                        <SuccessMessage message={success.get().unwrap_or_default()} />
                    </Show>
                </div>
            </div>
        </Show>
    }
}
