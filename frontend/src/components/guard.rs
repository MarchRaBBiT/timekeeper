use crate::{components::layout::LoadingSpinner, state::auth::use_auth};
use leptos::*;

#[component]
pub fn RequireAuth(children: ChildrenFn) -> impl IntoView {
    let (auth, _) = use_auth();
    let is_authenticated = create_memo(move |_| auth.get().is_authenticated);
    let is_loading = create_memo(move |_| auth.get().loading);
    create_effect(move |_| {
        let state = auth.get();
        if state.loading || state.is_authenticated {
            return;
        }
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href("/login");
        }
    });
    view! {
        <Show
            when=move || should_render_children(is_authenticated.get(), is_loading.get())
            fallback=move || {
                if is_loading.get() {
                    view! { <LoadingSpinner /> }.into_view()
                } else {
                    view! { <></> }.into_view()
                }
            }
        >
            {children()}
        </Show>
    }
}

fn should_render_children(is_authenticated: bool, is_loading: bool) -> bool {
    is_authenticated && !is_loading
}

#[cfg(test)]
mod tests {
    use super::should_render_children;

    #[test]
    fn guard_blocks_until_authenticated() {
        assert!(!should_render_children(false, true));
        assert!(!should_render_children(false, false));
        assert!(!should_render_children(true, true));
        assert!(should_render_children(true, false));
    }
}
