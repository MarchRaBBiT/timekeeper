use leptos::*;

pub mod components;
pub mod repository;
pub mod utils;

mod panel;

pub use panel::MfaRegisterPanel;

#[component]
pub fn MfaRegisterPage() -> impl IntoView {
    view! { <MfaRegisterPanel /> }
}
