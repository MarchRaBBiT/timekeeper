use crate::{
    api::LoginRequest,
    pages::login::{components::form::LoginForm, utils},
    state::auth,
};
use leptos::{ev::SubmitEvent, Callback, *};

#[component]
pub fn LoginPanel() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (totp_code, set_totp_code) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);

    let login_action = auth::use_login_action();
    let pending = login_action.pending();

    {
        create_effect(move |_| {
            if let Some(result) = login_action.value().get() {
                match result {
                    Ok(_) => {
                        set_error.set(None);
                        set_totp_code.set(String::new());
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/dashboard");
                        }
                    }
                    Err(err) => set_error.set(Some(err)),
                }
            }
        });
    }

    let handle_submit = {
        Callback::new(move |ev: SubmitEvent| {
            ev.prevent_default();
            if pending.get_untracked() {
                return;
            }
            let uname = username.get_untracked();
            let pword = password.get_untracked();

            if let Err(msg) = utils::validate_credentials(&uname, &pword) {
                set_error.set(Some(msg));
                return;
            }

            let totp_payload = utils::normalize_totp_code(&totp_code.get_untracked());
            set_error.set(None);

            let request = LoginRequest {
                username: uname,
                password: pword,
                totp_code: totp_payload,
                device_label: None,
            };

            login_action.dispatch(request);
        })
    };

    let username_input = Callback::new(move |value: String| set_username.set(value));
    let password_input = Callback::new(move |value: String| set_password.set(value));
    let totp_input = Callback::new(move |value: String| set_totp_code.set(value));

    view! {
        <LoginForm
            username=username
            password=password
            totp_code=totp_code
            error=error
            pending=pending.into()
            on_username_input=username_input
            on_password_input=password_input
            on_totp_input=totp_input
            on_submit=handle_submit
        />
    }
}
