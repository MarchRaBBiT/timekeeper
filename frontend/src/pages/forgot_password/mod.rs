use leptos::*;

mod panel;

pub use panel::ForgotPasswordPanel;

#[component]
pub fn ForgotPasswordPage() -> impl IntoView {
    view! { <ForgotPasswordPanel /> }
}
