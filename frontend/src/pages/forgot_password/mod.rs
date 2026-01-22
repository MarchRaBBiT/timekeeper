use leptos::*;

mod panel;
mod repository;
mod view_model;

pub use panel::ForgotPasswordPanel;

#[component]
pub fn ForgotPasswordPage() -> impl IntoView {
    view! { <ForgotPasswordPanel /> }
}
