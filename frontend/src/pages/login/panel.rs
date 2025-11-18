use crate::pages::login::components::form::LoginForm;
use leptos::*;

#[component]
pub fn LoginPanel() -> impl IntoView {
    view! { <LoginForm/> }
}
