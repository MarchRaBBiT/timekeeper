use crate::utils::storage as storage_utils;
use leptos::*;

fn has_access_token() -> bool {
    if let Ok(storage) = storage_utils::local_storage() {
        if let Ok(Some(tok)) = storage.get_item("access_token") {
            return !tok.is_empty();
        }
    }
    false
}

#[component]
pub fn RequireAuth(children: Children) -> impl IntoView {
    // On mount (async task), check token and redirect if missing
    leptos::spawn_local(async move {
        if !has_access_token() {
            if let Some(win) = web_sys::window() {
                let _ = win.location().set_href("/login");
            }
        }
    });
    // Render wrapped children properly
    view! { <>{children()}</> }
}
