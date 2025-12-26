use crate::components::guard::RequireAuth;
use leptos::*;

pub mod components;
pub mod layout;
pub mod panel;
pub mod repository;
pub mod utils;
pub mod view_model;

pub use panel::AdminPanel;

#[component]
pub fn AdminPage() -> impl IntoView {
    view! { <RequireAuth><AdminPanel /></RequireAuth> }
}
