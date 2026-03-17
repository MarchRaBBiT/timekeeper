#[cfg(all(test, not(target_arch = "wasm32")))]
pub mod ssr;

#[cfg(test)]
pub mod helpers {
    use crate::api::UserResponse;
    use crate::config::TimeZoneStatus;
    use crate::state::auth::AuthState;
    use leptos::*;
    use std::sync::MutexGuard;

    pub struct TestLocaleGuard {
        _guard: MutexGuard<'static, ()>,
        previous: String,
    }

    impl Drop for TestLocaleGuard {
        fn drop(&mut self) {
            rust_i18n::set_locale(&self.previous);
        }
    }

    pub fn admin_user(system_admin: bool) -> UserResponse {
        UserResponse {
            id: "u-admin".into(),
            username: "admin".into(),
            full_name: "Admin User".into(),
            role: "admin".into(),
            is_system_admin: system_admin,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        }
    }

    pub fn manager_user() -> UserResponse {
        UserResponse {
            id: "u-manager".into(),
            username: "manager".into(),
            full_name: "Manager User".into(),
            role: "manager".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
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
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
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

    pub fn set_test_locale(locale: &str) -> TestLocaleGuard {
        let guard = crate::config::acquire_test_serial_lock();
        let previous = rust_i18n::locale().to_string();
        rust_i18n::set_locale(locale);
        TestLocaleGuard {
            _guard: guard,
            previous,
        }
    }
}
