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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn dashboard_frame_wraps_children() {
        let html = render_to_string(move || {
            view! { <DashboardFrame><div>{"child"}</div></DashboardFrame> }
        });
        assert!(html.contains("child"));
    }

    #[test]
    fn unauthorized_message_renders_copy() {
        let html = render_to_string(move || view! { <UnauthorizedMessage /> });
        assert!(html.contains("権限が必要"));
    }
}
