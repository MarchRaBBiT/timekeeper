use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn UnauthorizedMessage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="bg-surface-elevated shadow rounded-lg p-6">
                <p class="text-sm text-fg">
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
                <h1 class="text-2xl font-bold text-fg">{"管理者ツール"}</h1>
                <p class="mt-1 text-sm text-fg-muted">
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn unauthorized_message_renders_copy() {
        let html = render_to_string(move || view! { <UnauthorizedMessage /> });
        assert!(html.contains("このページは管理者以上の権限が必要です。"));
    }

    #[test]
    fn admin_dashboard_frame_renders_header() {
        let html = render_to_string(move || {
            view! {
                <AdminDashboardFrame>
                    <div>{"child"}</div>
                </AdminDashboardFrame>
            }
        });
        assert!(html.contains("管理者ツール"));
        assert!(html.contains("child"));
    }

    #[test]
    fn admin_dashboard_scaffold_switches_content() {
        let allowed_html = render_to_string(move || {
            let allowed = create_memo(|_| true);
            view! {
                <AdminDashboardScaffold admin_allowed=allowed>
                    <div>{"allowed"}</div>
                </AdminDashboardScaffold>
            }
        });
        assert!(allowed_html.contains("管理者ツール"));
        assert!(allowed_html.contains("allowed"));

        let denied_html = render_to_string(move || {
            let allowed = create_memo(|_| false);
            view! {
                <AdminDashboardScaffold admin_allowed=allowed>
                    <div>{"denied"}</div>
                </AdminDashboardScaffold>
            }
        });
        assert!(denied_html.contains("このページは管理者以上の権限が必要です。"));
    }
}
