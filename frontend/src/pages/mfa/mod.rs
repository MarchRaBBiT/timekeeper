use leptos::*;

mod panel;

pub use panel::MfaRegisterPanel;

#[component]
pub fn MfaRegisterPage() -> impl IntoView {
    view! { <MfaRegisterPanel /> }
}
