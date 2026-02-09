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
                    ().into_view()
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::RequireAuth;
    use crate::state::auth::AuthState;
    use crate::test_support::helpers::regular_user;
    use crate::test_support::ssr::render_to_string;
    use leptos::*;

    fn provide_auth_state(is_authenticated: bool, loading: bool) {
        let (auth, set_auth) = create_signal(AuthState {
            user: if is_authenticated {
                Some(regular_user())
            } else {
                None
            },
            is_authenticated,
            loading,
        });
        provide_context((auth, set_auth));
    }

    #[test]
    fn require_auth_renders_children_when_authenticated() {
        let html = render_to_string(move || {
            provide_auth_state(true, false);
            view! {
                <RequireAuth>
                    {|| view! { <div>"protected-content"</div> }}
                </RequireAuth>
            }
        });
        assert!(html.contains("protected-content"));
    }

    #[test]
    fn require_auth_hides_children_when_unauthenticated() {
        let html = render_to_string(move || {
            provide_auth_state(false, false);
            view! {
                <RequireAuth>
                    {|| view! { <div>"protected-content"</div> }}
                </RequireAuth>
            }
        });
        assert!(!html.contains("protected-content"));
    }

    #[test]
    fn require_auth_shows_loading_spinner_while_loading() {
        let html = render_to_string(move || {
            provide_auth_state(false, true);
            view! {
                <RequireAuth>
                    {|| view! { <div>"protected-content"</div> }}
                </RequireAuth>
            }
        });
        assert!(html.contains("animate-spin"));
    }
}
