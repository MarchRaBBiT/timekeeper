use super::{
    repository::AdminUsersRepository,
    utils::{InviteFormState, MessageState},
};
use crate::{
    api::{ApiClient, CreateUser, UserResponse},
    state::auth::use_auth,
};
use leptos::*;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct AdminUsersViewModel {
    pub is_system_admin: Memo<bool>,
    pub invite_form: RwSignal<InviteFormState>,
    pub invite_messages: RwSignal<MessageState>,
    pub drawer_messages: RwSignal<MessageState>,
    pub selected_user: RwSignal<Option<UserResponse>>,
    pub users_reload: RwSignal<u32>,
    pub users_resource: Resource<(bool, u32), Result<Vec<UserResponse>, String>>,
    pub invite_action: Action<CreateUser, Result<UserResponse, String>>,
    pub reset_mfa_action: Action<String, Result<(), String>>,
}

pub fn use_admin_users_view_model() -> AdminUsersViewModel {
    let (auth, _set_auth) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repository = AdminUsersRepository::new_with_client(Rc::new(api));

    let is_system_admin = create_memo(move |_| {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    });

    let invite_form = create_rw_signal(InviteFormState::default());
    let invite_messages = create_rw_signal(MessageState::default());
    let drawer_messages = create_rw_signal(MessageState::default());
    let selected_user = create_rw_signal(None::<UserResponse>);
    let users_reload = create_rw_signal(0u32);

    let repo_for_resource = repository.clone();
    let users_resource = create_resource(
        move || (is_system_admin.get(), users_reload.get()),
        move |(allowed, _reload)| {
            let repo = repo_for_resource.clone();
            async move {
                if !allowed {
                    Err("システム管理者のみ利用できます。".to_string())
                } else {
                    repo.fetch_users().await
                }
            }
        },
    );

    let repo_for_invite = repository.clone();
    let invite_action = create_action(move |payload: &CreateUser| {
        let repo = repo_for_invite.clone();
        let payload = payload.clone();
        async move { repo.invite_user(payload).await }
    });

    let repo_for_reset = repository.clone();
    let reset_mfa_action = create_action(move |user_id: &String| {
        let repo = repo_for_reset.clone();
        let user_id = user_id.clone();
        async move { repo.reset_user_mfa(user_id).await }
    });

    // Effects
    create_effect(move |_| {
        if let Some(result) = invite_action.value().get() {
            match result {
                Ok(user) => {
                    invite_messages.update(|state| {
                        state.set_success(format!("ユーザー '{}' を作成しました。", user.username));
                    });
                    invite_form.update(|state| state.reset());
                    users_reload.update(|value| *value = value.wrapping_add(1));
                }
                Err(err) => {
                    invite_messages.update(|state| state.set_error(err));
                }
            }
        }
    });

    AdminUsersViewModel {
        is_system_admin,
        invite_form,
        invite_messages,
        drawer_messages,
        selected_user,
        users_reload,
        users_resource,
        invite_action,
        reset_mfa_action,
    }
}
