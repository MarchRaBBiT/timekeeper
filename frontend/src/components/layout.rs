use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn Header() -> impl IntoView {
    let (_auth, set_auth) = use_auth();

    fn can_manage_system() -> bool {
        let win = match web_sys::window() {
            Some(w) => w,
            None => return false,
        };
        let storage = match win.local_storage() {
            Ok(Some(s)) => s,
            _ => return false,
        };
        let user_json = match storage.get_item("current_user") {
            Ok(Some(json)) => json,
            _ => return false,
        };
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&user_json) {
            if value
                .get("is_system_admin")
                .and_then(|flag| flag.as_bool())
                .unwrap_or(false)
            {
                return true;
            }
            if value
                .get("role")
                .and_then(|r| r.as_str())
                .map(|r| r == "admin")
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }
    let on_logout = move |_| {
        leptos::spawn_local({
            let set_auth = set_auth.clone();
            async move {
                crate::state::auth::logout(set_auth).await;
                if let Some(win) = web_sys::window() {
                    let _ = win.location().set_href("/login");
                }
            }
        });
    };
    view! {
        <header class="bg-white shadow-sm border-b">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div class="flex justify-between items-center h-16">
                    <div class="flex items-center">
                        <h1 class="text-xl font-semibold text-gray-900">
                            "Timekeeper"
                        </h1>
                    </div>
                    <nav class="flex space-x-4">
                        <a href="/dashboard" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                            "ダッシュボード"
                        </a>
                        <a href="/attendance" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                            "勤怠"
                        </a>
                        <a href="/requests" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                            "申請"
                        </a>
                        <a href="/mfa/register" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                            "MFA設定"
                        </a>
                        <Show when=move || can_manage_system()>
                            <a href="/admin" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                                "管理"
                            </a>
                            <a href="/admin/users" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                                "ユーザー追加"
                            </a>
                            <a href="/admin/export" class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                                "データエクスポート"
                            </a>
                        </Show>
                        <button on:click=on_logout class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
                            "ログアウト"
                        </button>
                    </nav>
                </div>
            </div>
        </header>
    }
}

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            <Header/>
            <main class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                {children()}
            </main>
        </div>
    }
}

#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="flex justify-center items-center p-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
    }
}

#[component]
pub fn ErrorMessage(message: String) -> impl IntoView {
    view! {
        <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
            <div class="flex">
                <div class="flex-shrink-0">
                    <i class="fas fa-exclamation-circle"></i>
                </div>
                <div class="ml-3">
                    <p class="text-sm">{message}</p>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SuccessMessage(message: String) -> impl IntoView {
    view! {
        <div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded mb-4">
            <div class="flex">
                <div class="flex-shrink-0">
                    <i class="fas fa-check-circle"></i>
                </div>
                <div class="ml-3">
                    <p class="text-sm">{message}</p>
                </div>
            </div>
        </div>
    }
}
