use leptos::*;

#[component]
pub fn UnauthorizedAdminUsersMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <p class="text-sm text-gray-700">
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
                <h1 class="text-2xl font-bold text-gray-900">{"ユーザー管理"}</h1>
                <p class="mt-1 text-sm text-gray-600">
                    {"ユーザー招待とアクセス権管理、MFA リセットをまとめて操作できます。"}
                </p>
            </div>
            {children()}
        </div>
    }
}
