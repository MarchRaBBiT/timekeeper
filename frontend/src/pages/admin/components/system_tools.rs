use crate::{
    api::ApiError,
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin::{
        components::user_select::{AdminUserSelect, UsersResource},
        repository::AdminRepository,
    },
};
use leptos::*;

fn validate_mfa_reset_user_id(value: &str) -> Result<String, ApiError> {
    let user_id = value.trim();
    if user_id.is_empty() {
        Err(ApiError::validation("ユーザーIDを入力してください。"))
    } else {
        Ok(user_id.to_string())
    }
}

fn reset_button_label(pending: bool) -> &'static str {
    if pending {
        "リセット中..."
    } else {
        "MFA をリセット"
    }
}

fn prepare_mfa_reset_submission(
    system_admin_allowed: bool,
    user_id_raw: &str,
) -> Result<Option<String>, ApiError> {
    if !system_admin_allowed {
        return Ok(None);
    }
    let user_id = validate_mfa_reset_user_id(user_id_raw)?;
    Ok(Some(user_id))
}

fn mfa_reset_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some("MFA をリセットしました。".into()), None),
        Err(err) => (None, Some(err)),
    }
}

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
            let user_id = validate_mfa_reset_user_id(&user_id)?;
            repo.reset_mfa(&user_id).await
        }
    });
    let pending = reset_action.pending();
    {
        create_effect(move |_| {
            if let Some(result) = reset_action.value().get() {
                let (next_success, next_error) = mfa_reset_feedback(result);
                success.set(next_success);
                error.set(next_error);
            }
        });
    }

    let on_reset = {
        move |_| {
            let prepared = match prepare_mfa_reset_submission(
                system_admin_allowed.get_untracked(),
                &user_id.get(),
            ) {
                Ok(Some(value)) => value,
                Ok(None) => return,
                Err(err) => {
                    error.set(Some(err));
                    success.set(None);
                    return;
                }
            };
            error.set(None);
            success.set(None);
            reset_action.dispatch(prepared);
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
                            {move || reset_button_label(pending.get())}
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

    #[test]
    fn validate_mfa_reset_user_id_handles_blank_and_trimmed_values() {
        assert!(validate_mfa_reset_user_id("").is_err());
        assert!(validate_mfa_reset_user_id("   ").is_err());
        assert_eq!(
            validate_mfa_reset_user_id("  user-1 ").expect("valid"),
            "user-1"
        );
    }

    #[test]
    fn admin_mfa_reset_section_hidden_when_not_allowed() {
        let html = render_to_string(move || {
            let api = ApiClient::new();
            let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));
            let users = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            let allowed = create_memo(|_| false);
            view! { <AdminMfaResetSection repository=repo system_admin_allowed=allowed users=users /> }
        });
        assert!(!html.contains("MFA リセット"));
    }

    #[test]
    fn reset_button_label_reflects_pending_state() {
        assert_eq!(reset_button_label(true), "リセット中...");
        assert_eq!(reset_button_label(false), "MFA をリセット");
    }

    #[test]
    fn prepare_mfa_reset_submission_covers_allowed_and_validation_paths() {
        assert!(prepare_mfa_reset_submission(false, " user-1 ")
            .expect("not allowed should short-circuit")
            .is_none());
        assert!(prepare_mfa_reset_submission(true, "   ").is_err());
        assert_eq!(
            prepare_mfa_reset_submission(true, " user-1 ")
                .expect("valid input")
                .expect("dispatch payload"),
            "user-1"
        );
    }

    #[test]
    fn mfa_reset_feedback_maps_success_and_error() {
        let (ok_msg, ok_err) = mfa_reset_feedback(Ok(()));
        assert_eq!(ok_msg.as_deref(), Some("MFA をリセットしました。"));
        assert!(ok_err.is_none());

        let (err_msg, err) = mfa_reset_feedback(Err(ApiError::unknown("reset failed")));
        assert!(err_msg.is_none());
        assert_eq!(err.expect("error").error, "reset failed");
    }
}
