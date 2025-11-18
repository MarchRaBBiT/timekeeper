use crate::components::guard::RequireAuth;
use leptos::*;

pub mod attendance;
pub mod holidays;
pub mod layout;
pub mod panel;
pub mod requests;
pub mod system_tools;
pub mod weekly_holidays;

pub use panel::AdminPanel;

#[component]
pub fn AdminPage() -> impl IntoView {
    view! { <RequireAuth><AdminPanel /></RequireAuth> }
}
