use super::{
    repository::AdminUsersRepository,
    utils::{InviteFormState, MessageState},
};
use crate::api::{ApiClient, CreateUser, UserResponse};
use crate::state::auth::use_auth;
use leptos::*;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct AdminUsersViewModel {
    pub invite_form: InviteFormState,
    pub invite_messages: MessageState,
    pub drawer_messages: MessageState,
    pub selected_user: RwSignal<Option<UserResponse>>,
    pub users_resource: Resource<(bool, u32), Result<Vec<UserResponse>, String>>,
    pub invite_action: Action<CreateUser, Result<UserResponse, String>>,
    pub reset_mfa_action: Action<String, Result<(), String>>,
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

    let repo_resource = repo.clone();
    let users_resource = create_resource(
        move || (is_system_admin.get(), 0u32),
        move |(allowed, _reload)| {
            let repo = repo_resource.clone();
            async move {
                if !allowed {
                    Err("システム管理者のみ利用できます。".to_string())
                } else {
                    repo.fetch_users().await
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

    AdminUsersViewModel {
        invite_form,
        invite_messages,
        drawer_messages,
        selected_user,
        users_resource,
        invite_action,
        reset_mfa_action,
        is_system_admin,
    }
}
