use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn RequireAuth(children: Children) -> impl IntoView {
    let (auth, _) = use_auth();
    create_effect(move |_| {
        let state = auth.get();
        if state.loading || state.is_authenticated {
            return;
        }
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href("/login");
        }
    });
    view! { <>{children()}</> }
}
