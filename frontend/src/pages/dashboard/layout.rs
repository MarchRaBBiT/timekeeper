use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn DashboardFrame(children: Children) -> impl IntoView {
    view! { <Layout>{children()}</Layout> }
}

#[component]
pub fn UnauthorizedMessage() -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 text-center">
            <p class="text-sm text-fg">{"このページにアクセスするには権限が必要です。管理者にお問い合わせください。"}</p>
        </div>
    }
}
