use crate::components::forms::LoginForm;
use leptos::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <LoginForm/>
    }
}
