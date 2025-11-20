use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn UnauthorizedMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <p class="text-sm text-gray-700">
                    {"このページは管理者以上の権限が必要です。"}
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn AdminDashboardFrame(children: Children) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">{"管理者ツール"}</h1>
                <p class="mt-1 text-sm text-gray-600">
                    {"週次休日や申請、勤怠、MFA、祝日管理をまとめて実行できます。"}
                </p>
            </div>
            {children()}
        </div>
    }
}

#[component]
pub fn AdminDashboardScaffold(admin_allowed: Memo<bool>, children: Children) -> impl IntoView {
    let content = store_value(children());
    view! {
        <Layout>
            <Show
                when=move || admin_allowed.get()
                fallback=move || view! { <UnauthorizedMessage /> }.into_view()
            >
                <AdminDashboardFrame>
                    {content.get_value().clone()}
                </AdminDashboardFrame>
            </Show>
        </Layout>
    }
}
