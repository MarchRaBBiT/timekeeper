use super::{
    repository::AdminUsersRepository,
    utils::{InviteFormState, MessageState},
};
use crate::api::{
    ApiClient, ApiError, ArchivedUserResponse, CreateUser, PiiProtectedResponse, UserResponse,
};
use crate::state::auth::use_auth;
use leptos::*;
use std::rc::Rc;

/// Tab selection for user management page
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UserTab {
    #[default]
    Active,
    Archived,
}

#[derive(Clone, Copy)]
pub struct AdminUsersViewModel {
    pub invite_form: InviteFormState,
    pub invite_messages: MessageState,
    pub drawer_messages: MessageState,
    pub selected_user: RwSignal<Option<UserResponse>>,
    pub selected_archived_user: RwSignal<Option<ArchivedUserResponse>>,
    pub pii_masked: RwSignal<bool>,
    pub active_tab: RwSignal<UserTab>,
    pub users_resource:
        Resource<(bool, u32), Result<PiiProtectedResponse<Vec<UserResponse>>, ApiError>>,
    pub archived_users_resource: Resource<(bool, u32), Result<Vec<ArchivedUserResponse>, ApiError>>,
    pub invite_action: Action<CreateUser, Result<UserResponse, ApiError>>,
    pub reset_mfa_action: Action<String, Result<(), ApiError>>,
    pub unlock_user_action: Action<String, Result<(), ApiError>>,
    /// Delete a user: (user_id, hard_delete)
    pub delete_user_action: Action<(String, bool), Result<(), ApiError>>,
    /// Restore an archived user
    pub restore_archived_action: Action<String, Result<(), ApiError>>,
    /// Delete an archived user permanently
    pub delete_archived_action: Action<String, Result<(), ApiError>>,
    pub is_system_admin: Memo<bool>,
}

pub fn use_admin_users_view_model() -> AdminUsersViewModel {
    let (auth, _) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = AdminUsersRepository::new_with_client(Rc::new(api));

    let is_system_admin = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

    let invite_form = InviteFormState::default();
    let invite_messages = MessageState::default();
    let drawer_messages = MessageState::default();
    let selected_user = create_rw_signal(None::<UserResponse>);
    let selected_archived_user = create_rw_signal(None::<ArchivedUserResponse>);
    let pii_masked = create_rw_signal(false);
    let active_tab = create_rw_signal(UserTab::Active);

    // Active users resource
    let repo_resource = repo.clone();
    let users_resource = create_resource(
        move || (is_system_admin.get(), 0u32),
        move |(allowed, _reload)| {
            let repo = repo_resource.clone();
            let pii_masked = pii_masked;
            async move {
                if !allowed {
                    Err(ApiError::validation("システム管理者のみ利用できます。"))
                } else {
                    let response = repo.fetch_users().await?;
                    pii_masked.set(response.pii_masked);
                    Ok(response)
                }
            }
        },
    );

    // Archived users resource
    let repo_archived = repo.clone();
    let archived_users_resource = create_resource(
        move || (is_system_admin.get(), 0u32),
        move |(allowed, _reload)| {
            let repo = repo_archived.clone();
            async move {
                if !allowed {
                    Err(ApiError::validation("システム管理者のみ利用できます。"))
                } else {
                    repo.fetch_archived_users().await
                }
            }
        },
    );

    let repo_invite = repo.clone();
    let invite_action = create_action(move |payload: &CreateUser| {
        let repo = repo_invite.clone();
        let payload = payload.clone();
        async move { repo.invite_user(payload).await }
    });

    let repo_reset = repo.clone();
    let reset_mfa_action = create_action(move |user_id: &String| {
        let repo = repo_reset.clone();
        let user_id = user_id.clone();
        async move { repo.reset_user_mfa(user_id).await }
    });

    let repo_delete = repo.clone();
    let delete_user_action = create_action(move |args: &(String, bool)| {
        let repo = repo_delete.clone();
        let (user_id, hard) = args.clone();
        async move { repo.delete_user(user_id, hard).await }
    });

    let repo_unlock = repo.clone();
    let unlock_user_action = create_action(move |user_id: &String| {
        let repo = repo_unlock.clone();
        let user_id = user_id.clone();
        async move { repo.unlock_user(user_id).await }
    });

    let repo_restore = repo.clone();
    let restore_archived_action = create_action(move |user_id: &String| {
        let repo = repo_restore.clone();
        let user_id = user_id.clone();
        async move { repo.restore_archived_user(user_id).await }
    });

    let repo_delete_archived = repo.clone();
    let delete_archived_action = create_action(move |user_id: &String| {
        let repo = repo_delete_archived.clone();
        let user_id = user_id.clone();
        async move { repo.delete_archived_user(user_id).await }
    });

    // Effects for action success
    create_effect(move |_| {
        if let Some(result) = invite_action.value().get() {
            match result {
                Ok(user) => {
                    invite_messages
                        .set_success(format!("ユーザー '{}' を作成しました。", user.username));
                    invite_form.reset();
                    users_resource.refetch();
                }
                Err(err) => {
                    invite_messages.set_error(err);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = reset_mfa_action.value().get() {
            match result {
                Ok(_) => {
                    drawer_messages.set_success("MFA をリセットしました。");
                }
                Err(err) => {
                    drawer_messages.set_error(err);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = delete_user_action.value().get() {
            match result {
                Ok(_) => {
                    drawer_messages.set_success("ユーザーを削除しました。");
                    selected_user.set(None);
                    users_resource.refetch();
                    archived_users_resource.refetch();
                }
                Err(err) => {
                    drawer_messages.set_error(err);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = unlock_user_action.value().get() {
            match result {
                Ok(_) => {
                    drawer_messages.set_success("ユーザーのロックを解除しました。");
                    users_resource.refetch();
                }
                Err(err) => {
                    drawer_messages.set_error(err);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = restore_archived_action.value().get() {
            match result {
                Ok(_) => {
                    drawer_messages.set_success("ユーザーを復職させました。");
                    selected_archived_user.set(None);
                    users_resource.refetch();
                    archived_users_resource.refetch();
                }
                Err(err) => {
                    drawer_messages.set_error(err);
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(result) = delete_archived_action.value().get() {
            match result {
                Ok(_) => {
                    drawer_messages.set_success("退職ユーザーを完全削除しました。");
                    selected_archived_user.set(None);
                    archived_users_resource.refetch();
                }
                Err(err) => {
                    drawer_messages.set_error(err);
                }
            }
        }
    });

    AdminUsersViewModel {
        invite_form,
        invite_messages,
        drawer_messages,
        selected_user,
        selected_archived_user,
        pii_masked,
        active_tab,
        users_resource,
        archived_users_resource,
        invite_action,
        reset_mfa_action,
        unlock_user_action,
        delete_user_action,
        restore_archived_action,
        delete_archived_action,
        is_system_admin,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::{render_to_string, with_local_runtime_async};

    #[test]
    fn admin_users_view_model_initializes() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([{
                "id": "u1",
                "username": "alice",
                "full_name": "Alice Example",
                "role": "admin",
                "is_system_admin": true,
                "mfa_enabled": false
            }]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/archived-users");
            then.status(200).json_body(serde_json::json!([]));
        });

        let server = server.clone();
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_admin_users_view_model();
            assert!(matches!(vm.active_tab.get_untracked(), UserTab::Active));
            view! { <div>{vm.invite_form.role.get()}</div> }
        });
        assert!(html.contains("employee"));
    }

    fn mock_server_for_actions() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([{
                "id": "u1",
                "username": "alice",
                "full_name": "Alice Example",
                "role": "member",
                "is_system_admin": false,
                "mfa_enabled": false
            }]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/archived-users");
            then.status(200).json_body(serde_json::json!([{
                "id": "a1",
                "username": "retired",
                "full_name": "Retired User",
                "role": "member",
                "is_system_admin": false,
                "archived_at": "2026-01-01T00:00:00Z",
                "archived_by": "u-admin"
            }]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!({
                "id": "u2",
                "username": "new-user",
                "full_name": "New User",
                "role": "member",
                "is_system_admin": false,
                "mfa_enabled": false
            }));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/admin/mfa/reset");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/users/u1");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/admin/archived-users/a1/restore");
            then.status(200).json_body(serde_json::json!({}));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/admin/archived-users/a1");
            then.status(200).json_body(serde_json::json!({}));
        });
        server
    }

    #[test]
    fn admin_users_actions_update_messages() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server_for_actions();
            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_admin_users_view_model();

            vm.invite_action.dispatch(CreateUser {
                username: "new-user".into(),
                password: "password".into(),
                full_name: "New User".into(),
                email: "new@example.com".into(),
                role: "member".into(),
                is_system_admin: false,
            });
            for _ in 0..10 {
                if vm.invite_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let invite_result = vm.invite_action.value().get();
            match invite_result {
                Some(Ok(user)) => assert_eq!(user.username, "new-user"),
                other => panic!("invite action did not succeed: {:?}", other),
            }

            vm.reset_mfa_action.dispatch("u1".into());
            for _ in 0..10 {
                if vm.reset_mfa_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let reset_result = vm.reset_mfa_action.value().get();
            assert!(matches!(reset_result, Some(Ok(()))));

            vm.delete_user_action.dispatch(("u1".into(), false));
            for _ in 0..10 {
                if vm.delete_user_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let delete_result = vm.delete_user_action.value().get();
            assert!(matches!(delete_result, Some(Ok(()))));

            vm.restore_archived_action.dispatch("a1".into());
            for _ in 0..10 {
                if vm.restore_archived_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let restore_result = vm.restore_archived_action.value().get();
            assert!(matches!(restore_result, Some(Ok(()))));

            vm.delete_archived_action.dispatch("a1".into());
            for _ in 0..10 {
                if vm.delete_archived_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let delete_archived_result = vm.delete_archived_action.value().get();
            assert!(matches!(delete_archived_result, Some(Ok(()))));

            runtime.dispose();
        });
    }
}
