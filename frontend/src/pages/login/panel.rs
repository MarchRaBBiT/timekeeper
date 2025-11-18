use crate::components::forms::LoginForm;
use leptos::*;

#[component]
pub fn LoginPanel() -> impl IntoView {
    view! { <LoginForm/> }
}
