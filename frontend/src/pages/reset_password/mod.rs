use leptos::*;

mod panel;
mod repository;
mod view_model;

pub use panel::ResetPasswordPanel;

#[component]
pub fn ResetPasswordPage() -> impl IntoView {
    view! { <ResetPasswordPanel /> }
}
