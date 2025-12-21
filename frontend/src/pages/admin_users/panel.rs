use crate::{
    api::{CreateUser, UserResponse},
    components::layout::Layout,
    state::auth::use_auth,
};
use leptos::*;

use super::{
    components::{detail::UserDetailDrawer, invite_form::InviteForm, list::UserList},
    layout::{AdminUsersFrame, UnauthorizedAdminUsersMessage},
    repository::AdminUsersRepository,
    utils::{InviteFormState, MessageState},
};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let repository = AdminUsersRepository::new();
    let (auth, _set_auth) = use_auth();
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
    let users_loading = users_resource.loading();

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

    {
        create_effect(move |_| {
            if let Some(result) = invite_action.value().get() {
                match result {
                    Ok(user) => {
                        invite_messages.update(|state| {
                            state.set_success(format!(
                                "ユーザー '{}' を作成しました。",
                                user.username
                            ));
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
    }

    let select_user = Callback::new({
        move |user: UserResponse| {
            drawer_messages.set(MessageState::default());
            selected_user.set(Some(user));
        }
    });

    view! {
        <Layout>
            <Show
                when=move || is_system_admin.get()
                fallback=move || view! { <UnauthorizedAdminUsersMessage /> }.into_view()
            >
                <AdminUsersFrame>
                    <InviteForm
                        form_state=invite_form
                        messages=invite_messages
                        invite_action=invite_action
                        is_system_admin=is_system_admin
                    />
                    <UserList
                        users_resource=users_resource
                        loading=users_loading
                        on_select=select_user
                    />
                </AdminUsersFrame>
                <UserDetailDrawer
                    selected_user=selected_user
                    messages=drawer_messages
                    reset_mfa_action=reset_mfa_action
                />
            </Show>
        </Layout>
    }
}
