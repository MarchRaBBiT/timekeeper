use leptos::*;

mod panel;

pub use panel::ResetPasswordPanel;

#[component]
pub fn ResetPasswordPage() -> impl IntoView {
    view! { <ResetPasswordPanel /> }
}
