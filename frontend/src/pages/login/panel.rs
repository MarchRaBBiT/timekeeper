use crate::pages::login::components::form::LoginForm;
use leptos::*;
#[component]
pub fn LoginPanel() -> impl IntoView {
    let vm = crate::pages::login::view_model::use_login_view_model();

    view! {
        <LoginForm
            form=vm.form
            error=vm.error
            login_action=vm.login_action
        />
    }
}
