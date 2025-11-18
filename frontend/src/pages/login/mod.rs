use leptos::*;

mod panel;

pub use panel::LoginPanel;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! { <LoginPanel /> }
}
