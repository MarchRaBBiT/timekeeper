use leptos::*;

pub mod components;
pub mod panel;
pub mod repository;
pub mod utils;
pub mod view_model;

pub use panel::MfaRegisterPanel;

#[component]
pub fn MfaRegisterPage() -> impl IntoView {
    view! { <MfaRegisterPanel /> }
}
