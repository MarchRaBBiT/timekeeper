use crate::{api::UserResponse, components::layout::LoadingSpinner, state::auth::use_auth};
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

#[component]
pub fn RequireAdmin(children: ChildrenFn) -> impl IntoView {
    let (auth, _) = use_auth();
    let is_authenticated = create_memo(move |_| auth.get().is_authenticated);
    let is_loading = create_memo(move |_| auth.get().loading);
    let is_admin = create_memo(move |_| is_admin_user(auth.get().user.as_ref()));
    create_effect(move |_| {
        let state = auth.get();
        if state.loading {
            return;
        }
        let target = if !state.is_authenticated {
            "/login"
        } else if !is_admin_user(state.user.as_ref()) {
            "/dashboard"
        } else {
            return;
        };
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href(target);
        }
    });
    view! {
        <Show
            when=move || {
                should_render_admin_children(is_authenticated.get(), is_loading.get(), is_admin.get())
            }
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

fn is_admin_user(user: Option<&UserResponse>) -> bool {
    user.map(|u| u.is_system_admin || u.role == "admin")
        .unwrap_or(false)
}

fn should_render_admin_children(is_authenticated: bool, is_loading: bool, is_admin: bool) -> bool {
    is_authenticated && is_admin && !is_loading
}

#[cfg(test)]
mod tests {
    use super::{is_admin_user, should_render_admin_children, should_render_children};
    use crate::api::UserResponse;

    #[test]
    fn guard_blocks_until_authenticated() {
        assert!(!should_render_children(false, true));
        assert!(!should_render_children(false, false));
        assert!(!should_render_children(true, true));
        assert!(should_render_children(true, false));
    }

    #[test]
    fn admin_guard_requires_admin_role_or_system_admin() {
        let regular = UserResponse {
            id: "u1".into(),
            username: "employee".into(),
            full_name: "Employee".into(),
            role: "employee".into(),
            is_system_admin: false,
            mfa_enabled: true,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
        };
        let admin = UserResponse {
            role: "admin".into(),
            ..regular.clone()
        };
        let system_admin = UserResponse {
            role: "employee".into(),
            is_system_admin: true,
            ..regular.clone()
        };
        assert!(!is_admin_user(None));
        assert!(!is_admin_user(Some(&regular)));
        assert!(is_admin_user(Some(&admin)));
        assert!(is_admin_user(Some(&system_admin)));
    }

    #[test]
    fn admin_guard_blocks_non_admins() {
        assert!(!should_render_admin_children(false, true, false));
        assert!(!should_render_admin_children(false, false, true));
        assert!(!should_render_admin_children(true, true, true));
        assert!(!should_render_admin_children(true, false, false));
        assert!(should_render_admin_children(true, false, true));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::{RequireAdmin, RequireAuth};
    use crate::state::auth::AuthState;
    use crate::test_support::helpers::{admin_user, regular_user};
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

    #[test]
    fn require_admin_renders_children_for_admin_user() {
        let html = render_to_string(move || {
            let (auth, set_auth) = create_signal(AuthState {
                user: Some(admin_user(true)),
                is_authenticated: true,
                loading: false,
            });
            provide_context((auth, set_auth));
            view! {
                <RequireAdmin>
                    {|| view! { <div>"admin-protected"</div> }}
                </RequireAdmin>
            }
        });
        assert!(html.contains("admin-protected"));
    }

    #[test]
    fn require_admin_hides_children_for_regular_user() {
        let html = render_to_string(move || {
            let (auth, set_auth) = create_signal(AuthState {
                user: Some(regular_user()),
                is_authenticated: true,
                loading: false,
            });
            provide_context((auth, set_auth));
            view! {
                <RequireAdmin>
                    {|| view! { <div>"admin-protected"</div> }}
                </RequireAdmin>
            }
        });
        assert!(!html.contains("admin-protected"));
    }
}
