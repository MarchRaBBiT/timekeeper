use crate::{
    api::{ArchivedUserResponse, UserResponse},
    components::layout::Layout,
    state::auth::use_auth,
};
use leptos::*;

use super::{
    components::{
        archived_detail::ArchivedUserDetailDrawer, archived_list::ArchivedUserList,
        detail::UserDetailDrawer, invite_form::InviteForm, list::UserList,
    },
    layout::{AdminUsersFrame, UnauthorizedAdminUsersMessage},
    view_model::UserTab,
};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let vm = super::view_model::use_admin_users_view_model();
    let (auth, _) = use_auth();

    let current_user_id = Signal::derive(move || auth.get().user.as_ref().map(|u| u.id.clone()));

    let select_user = Callback::new({
        let selected = vm.selected_user;
        let messages = vm.drawer_messages;
        move |user: UserResponse| {
            messages.clear();
            selected.set(Some(user));
        }
    });

    let select_archived_user = Callback::new({
        let selected = vm.selected_archived_user;
        let messages = vm.drawer_messages;
        move |user: ArchivedUserResponse| {
            messages.clear();
            selected.set(Some(user));
        }
    });

    let set_tab_active = {
        let tab = vm.active_tab;
        move |_| tab.set(UserTab::Active)
    };

    let set_tab_archived = {
        let tab = vm.active_tab;
        move |_| tab.set(UserTab::Archived)
    };

    view! {
        <Layout>
            <Show
                when=move || vm.is_system_admin.get()
                fallback=move || view! { <UnauthorizedAdminUsersMessage /> }.into_view()
            >
                <AdminUsersFrame>
                    // Tab header
                    <div class="flex border-b border-border mb-4">
                        <button
                            class=move || {
                                if vm.active_tab.get() == UserTab::Active {
                                    "px-4 py-2 font-medium text-link border-b-2 border-action-primary-border"
                                } else {
                                    "px-4 py-2 font-medium text-fg-muted hover:text-fg"
                                }
                            }
                            on:click=set_tab_active
                        >
                            {"アクティブユーザー"}
                        </button>
                        <button
                            class=move || {
                                if vm.active_tab.get() == UserTab::Archived {
                                    "px-4 py-2 font-medium text-link border-b-2 border-action-primary-border"
                                } else {
                                    "px-4 py-2 font-medium text-fg-muted hover:text-fg"
                                }
                            }
                            on:click=set_tab_archived
                        >
                            {"退職ユーザー"}
                        </button>
                    </div>

                    // Tab content
                    <Show
                        when=move || vm.active_tab.get() == UserTab::Active
                        fallback=move || {
                            view! {
                                <ArchivedUserList
                                    archived_users_resource=vm.archived_users_resource
                                    loading=vm.archived_users_resource.loading()
                                    on_select=select_archived_user
                                />
                            }
                        }
                    >
                        <Show when=move || vm.pii_masked.get()>
                            <div class="mb-4 rounded-lg border border-status-warning-border bg-status-warning-bg px-3 py-2 text-sm text-status-warning-text">
                                {"この一覧の個人情報はマスキング表示されています。"}
                            </div>
                        </Show>
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
                    </Show>
                </AdminUsersFrame>

                // Active user detail drawer
                <UserDetailDrawer
                    selected_user=vm.selected_user
                    messages=vm.drawer_messages
                    reset_mfa_action=vm.reset_mfa_action
                    unlock_user_action=vm.unlock_user_action
                    delete_user_action=vm.delete_user_action
                    current_user_id=current_user_id
                />

                // Archived user detail drawer
                <ArchivedUserDetailDrawer
                    selected_archived_user=vm.selected_archived_user
                    messages=vm.drawer_messages
                    restore_action=vm.restore_archived_action
                    delete_action=vm.delete_archived_action
                />
            </Show>
        </Layout>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn admin_users_page_renders_for_system_admin() {
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            view! { <AdminUsersPage /> }
        });
        assert!(html.contains("ユーザー管理"));
        assert!(html.contains("アクティブユーザー"));
    }
}
