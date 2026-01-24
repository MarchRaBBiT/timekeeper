use leptos::*;

#[component]
pub fn UnauthorizedAdminUsersMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-surface-elevated shadow rounded-lg p-6">
                <p class="text-sm text-fg">
                    {"このページはシステム管理者のみ利用できます。"}
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn AdminUsersFrame(children: Children) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-fg">{"ユーザー管理"}</h1>
                <p class="mt-1 text-sm text-fg-muted">
                    {"ユーザー招待とアクセス権管理、MFA リセットをまとめて操作できます。"}
                </p>
            </div>
            {children()}
        </div>
    }
}
