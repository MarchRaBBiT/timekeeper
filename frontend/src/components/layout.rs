use crate::state::auth::{self, use_auth};
use leptos::*;

#[component]
pub fn Header() -> impl IntoView {
    let (auth, _set_auth) = use_auth();
    let (menu_open, set_menu_open) = create_signal(false);
    let can_access_admin = move || {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
            .unwrap_or(false)
    };
    let can_manage_users = move || {
        auth.get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin)
            .unwrap_or(false)
    };
    let logout_action = auth::use_logout_action();
    let logout_pending = logout_action.pending();
    {
        create_effect(move |_| {
            if logout_action.value().get().is_some() {
                if let Some(win) = web_sys::window() {
                    let _ = win.location().set_href("/login");
                }
            }
        });
    }
    let on_logout = {
        move |_| {
            if logout_pending.get_untracked() {
                return;
            }
            set_menu_open.set(false);
            logout_action.dispatch(false);
        }
    };
    let toggle_menu = { move |_| set_menu_open.update(|open| *open = !*open) };
    view! {
        <header class="bg-surface-elevated shadow-sm border-b border-border">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div class="flex justify-between items-center h-16">
                    <div class="flex items-center">
                        <h1 class="text-xl font-semibold text-fg">
                            "Timekeeper"
                        </h1>
                    </div>
                    <div class="flex items-center">
                        <nav class="hidden lg:flex space-x-4">
                            <a href="/dashboard" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                "ダッシュボード"
                            </a>
                            <a href="/attendance" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                "勤怠"
                            </a>
                            <a href="/requests" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                "申請"
                            </a>
                            <a href="/settings" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                "設定"
                            </a>
                            <Show when=move || can_access_admin()>
                                <a href="/admin" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                    "管理"
                                </a>
                                <a href="/admin/export" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                    "データエクスポート"
                                </a>
                                <a href="/admin/audit-logs" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                    "監査ログ"
                                </a>
                            </Show>
                            <Show when=move || can_manage_users()>
                                <a href="/admin/users" class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover">
                                    "ユーザー追加"
                                </a>
                            </Show>
                            <button
                                on:click=on_logout
                                class="text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium disabled:opacity-50 hover:bg-action-ghost-bg-hover"
                                disabled={move || logout_pending.get()}
                            >
                                "ログアウト"
                            </button>
                        </nav>
                        <button
                            type="button"
                            class="lg:hidden inline-flex items-center justify-center p-2 rounded-md text-fg-muted hover:text-fg hover:bg-action-ghost-bg-hover"
                            on:click=toggle_menu
                            aria-expanded=move || menu_open.get()
                            aria-controls="mobile-nav"
                        >
                            <span class="sr-only">
                                {move || if menu_open.get() { "メニューを閉じる" } else { "メニューを開く" }}
                            </span>
                            <svg
                                class="h-6 w-6"
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke="currentColor"
                            >
                                <Show
                                    when=move || menu_open.get()
                                    fallback=move || {
                                        view! {
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M4 6h16M4 12h16M4 18h16"
                                            />
                                        }
                                    }
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M6 18L18 6M6 6l12 12"
                                    />
                                </Show>
                            </svg>
                        </button>
                    </div>
                </div>
                <Show when=move || menu_open.get()>
                    <div id="mobile-nav" class="lg:hidden border-t border-border">
                        <nav class="px-4 py-3 space-y-2">
                            <a
                                href="/dashboard"
                                class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                on:click=move |_| set_menu_open.set(false)
                            >
                                "ダッシュボード"
                            </a>
                            <a
                                href="/attendance"
                                class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                on:click=move |_| set_menu_open.set(false)
                            >
                                "勤怠"
                            </a>
                            <a
                                href="/requests"
                                class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                on:click=move |_| set_menu_open.set(false)
                            >
                                "申請"
                            </a>
                            <a
                                href="/settings"
                                class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                on:click=move |_| set_menu_open.set(false)
                            >
                                "設定"
                            </a>
                            <Show when=move || can_access_admin()>
                                <a
                                    href="/admin"
                                    class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                    on:click=move |_| set_menu_open.set(false)
                                >
                                    "管理"
                                </a>
                                <a
                                    href="/admin/export"
                                    class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                    on:click=move |_| set_menu_open.set(false)
                                >
                                    "データエクスポート"
                                </a>
                                <a
                                    href="/admin/audit-logs"
                                    class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                    on:click=move |_| set_menu_open.set(false)
                                >
                                    "監査ログ"
                                </a>
                            </Show>
                            <Show when=move || can_manage_users()>
                                <a
                                    href="/admin/users"
                                    class="block text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium hover:bg-action-ghost-bg-hover"
                                    on:click=move |_| set_menu_open.set(false)
                                >
                                    "ユーザー追加"
                                </a>
                            </Show>
                            <button
                                on:click=on_logout
                                class="w-full text-left text-fg-muted hover:text-fg px-3 py-2 rounded-md text-sm font-medium disabled:opacity-50 hover:bg-action-ghost-bg-hover"
                                disabled={move || logout_pending.get()}
                            >
                                "ログアウト"
                            </button>
                        </nav>
                    </div>
                </Show>
            </div>
        </header>
    }
}

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen bg-surface">
            <Header/>
            <main class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                <TimeZoneWarningBanner/>
                {children()}
            </main>
        </div>
    }
}

#[component]
pub fn TimeZoneWarningBanner() -> impl IntoView {
    let (config_read, config_write) = crate::state::config::use_config();
    let status = Signal::derive(move || config_read.get().time_zone_status);

    create_effect(move |_| {
        spawn_local(async move {
            let _ = crate::config::await_time_zone().await;
            let current = crate::config::time_zone_status();
            config_write.update(|s| s.time_zone_status = current);
        });
    });

    let on_retry = move |_| {
        if status.get_untracked().loading {
            return;
        }
        spawn_local(async move {
            crate::state::config::refresh_time_zone(config_write).await;
        });
    };

    let should_show = move || {
        let state = status.get();
        state.is_fallback || state.last_error.is_some()
    };

    let warning_message = move || {
        let state = status.get();
        if state.loading {
            "バックエンドのタイムゾーン情報を再取得しています...".to_string()
        } else if let Some(err) = state.last_error.clone() {
            format!(
                "サーバーのタイムゾーン情報を取得できませんでした ({})。現在 {} として動作しています。",
                err,
                state.time_zone.clone().unwrap_or_else(|| "UTC".into())
            )
        } else {
            format!(
                "サーバーのタイムゾーン情報を取得できず、現在 {} として動作しています。",
                state.time_zone.clone().unwrap_or_else(|| "UTC".into())
            )
        }
    };

    let refreshing = Signal::derive(move || status.get().loading);

    view! {
        <Show when=should_show>
            <div class="mb-4">
                <div class="bg-status-warning-bg border border-status-warning-border text-status-warning-text px-4 py-3 rounded">
                    <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                        <div>
                            <p class="font-semibold">{"タイムゾーン情報に関する警告"}</p>
                            <p class="text-sm mt-1">{warning_message}</p>
                        </div>
                        <button
                            class="inline-flex items-center justify-center px-4 py-2 border border-status-warning-border text-sm font-medium rounded text-status-warning-text hover:bg-status-warning-bg disabled:opacity-60"
                            on:click=on_retry
                            disabled=move || refreshing.get()
                        >
                            {move || if refreshing.get() { "再取得中..." } else { "再取得" }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="flex justify-center items-center p-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-action-primary-bg"></div>
        </div>
    }
}

#[component]
pub fn ErrorMessage(message: String) -> impl IntoView {
    view! {
        <div class="bg-status-error-bg border border-status-error-border text-status-error-text px-4 py-3 rounded mb-4">
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
        <div class="bg-status-success-bg border border-status-success-border text-status-success-text px-4 py-3 rounded mb-4">
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
