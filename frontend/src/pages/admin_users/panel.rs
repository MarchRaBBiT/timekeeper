use crate::components::layout::Layout;
use leptos::*;

use super::layout::{AdminUsersFrame, UnauthorizedAdminUsersMessage};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    view! {
        <Layout>
            <Show
                when=move || true
                fallback=move || view! { <UnauthorizedAdminUsersMessage /> }.into_view()
            >
                <AdminUsersFrame>
                    <p class="text-sm text-gray-600">{"Admin Users panel placeholder"}</p>
                </AdminUsersFrame>
            </Show>
        </Layout>
    }
}
