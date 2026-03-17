use crate::components::guard::RequireAuth;
use leptos::*;

pub mod layout;
pub mod panel;
pub mod view_model;

pub use panel::AdminSettingsPanel;

#[component]
pub fn AdminSettingsPage() -> impl IntoView {
    view! { <RequireAuth><AdminSettingsPanel /></RequireAuth> }
}
