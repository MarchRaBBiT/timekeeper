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
                <p class="mt-1 text-sm text-gray-600">{"申請の承認/却下、各種の手動登録が行えます。"}</p>
            </div>
            {children()}
        </div>
    }
}
