use leptos::*;

pub mod components;
pub mod repository;
pub mod utils;
pub mod view_model;

mod panel;

pub use panel::LoginPanel;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! { <LoginPanel /> }
}
