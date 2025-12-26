use crate::{
    api::{CreateUser, UserResponse},
    components::layout::Layout,
};
use leptos::*;

use super::{
    components::{detail::UserDetailDrawer, invite_form::InviteForm, list::UserList},
    layout::{AdminUsersFrame, UnauthorizedAdminUsersMessage},
};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let vm = super::view_model::use_admin_users_view_model();

    let select_user = Callback::new({
        let selected = vm.selected_user;
        let messages = vm.drawer_messages;
        move |user: UserResponse| {
            messages.clear();
            selected.set(Some(user));
        }
    });

    view! {
        <Layout>
            <Show
                when=move || vm.is_system_admin.get()
                fallback=move || view! { <UnauthorizedAdminUsersMessage /> }.into_view()
            >
                <AdminUsersFrame>
                    <InviteForm
                        form_state=vm.invite_form
                        messages=vm.invite_messages
                        invite_action=vm.invite_action
                        is_system_admin=vm.is_system_admin
                    />
                    <UserList
                        users_resource=vm.users_resource
                        loading=vm.users_resource.loading()
                        on_select=select_user
                    />
                </AdminUsersFrame>
                <UserDetailDrawer
                    selected_user=vm.selected_user
                    messages=vm.drawer_messages
                    reset_mfa_action=vm.reset_mfa_action
                />
            </Show>
        </Layout>
    }
}
