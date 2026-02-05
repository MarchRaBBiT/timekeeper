#[cfg(all(test, not(target_arch = "wasm32")))]
pub mod ssr;

#[cfg(test)]
pub mod helpers {
    use crate::api::UserResponse;
    use crate::config::TimeZoneStatus;
    use crate::state::auth::AuthState;
    use leptos::*;

    pub fn admin_user(system_admin: bool) -> UserResponse {
        UserResponse {
            id: "u-admin".into(),
            username: "admin".into(),
            full_name: "Admin User".into(),
            role: "admin".into(),
            is_system_admin: system_admin,
            mfa_enabled: false,
        }
    }

    pub fn regular_user() -> UserResponse {
        UserResponse {
            id: "u-regular".into(),
            username: "member".into(),
            full_name: "Regular User".into(),
            role: "member".into(),
            is_system_admin: false,
            mfa_enabled: false,
        }
    }

    pub fn provide_auth(
        user: Option<UserResponse>,
    ) -> (ReadSignal<AuthState>, WriteSignal<AuthState>) {
        let (auth, set_auth) = create_signal(AuthState {
            user,
            is_authenticated: true,
            loading: false,
        });
        provide_context((auth, set_auth));
        (auth, set_auth)
    }

    pub fn set_time_zone_ok() {
        crate::config::overwrite_time_zone_status_for_test(TimeZoneStatus {
            time_zone: Some("UTC".into()),
            is_fallback: false,
            last_error: None,
            loading: false,
        });
    }
}
