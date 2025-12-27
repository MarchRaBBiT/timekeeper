use crate::api::CreateUser;
use leptos::*;

#[derive(Clone, Copy)]
pub struct InviteFormState {
    pub username: RwSignal<String>,
    pub full_name: RwSignal<String>,
    pub password: RwSignal<String>,
    pub role: RwSignal<String>,
    pub is_system_admin: RwSignal<bool>,
}

impl Default for InviteFormState {
    fn default() -> Self {
        Self {
            username: create_rw_signal(String::new()),
            full_name: create_rw_signal(String::new()),
            password: create_rw_signal(String::new()),
            role: create_rw_signal("employee".to_string()),
            is_system_admin: create_rw_signal(false),
        }
    }
}

impl InviteFormState {
    pub fn is_valid(&self) -> bool {
        !(self.username.get().trim().is_empty()
            || self.full_name.get().trim().is_empty()
            || self.password.get().trim().is_empty())
    }

    pub fn reset(&self) {
        self.username.set(String::new());
        self.full_name.set(String::new());
        self.password.set(String::new());
        self.role.set("employee".to_string());
        self.is_system_admin.set(false);
    }

    pub fn to_request(self) -> CreateUser {
        CreateUser {
            username: self.username.get(),
            password: self.password.get(),
            full_name: self.full_name.get(),
            role: self.role.get(),
            is_system_admin: self.is_system_admin.get(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MessageState {
    pub success: RwSignal<Option<String>>,
    pub error: RwSignal<Option<String>>,
}

impl Default for MessageState {
    fn default() -> Self {
        Self {
            success: create_rw_signal(None),
            error: create_rw_signal(None),
        }
    }
}

impl MessageState {
    pub fn clear(&self) {
        self.success.set(None);
        self.error.set(None);
    }

    pub fn set_success(&self, message: impl Into<String>) {
        self.success.set(Some(message.into()));
        self.error.set(None);
    }

    pub fn set_error(&self, message: impl Into<String>) {
        self.error.set(Some(message.into()));
        self.success.set(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::create_runtime;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[wasm_bindgen_test]
    fn invite_form_state_validation() {
        with_runtime(|| {
            let state = InviteFormState::default();
            assert!(!state.is_valid());

            state.username.set("admin".into());
            state.full_name.set("System 管理者".into());
            state.password.set("Password123!".into());

            assert!(state.is_valid());

            let request = state.to_request();
            assert_eq!(request.username, "admin");
            assert_eq!(request.full_name, "System 管理者");
            assert_eq!(request.role, "employee");
            assert!(!request.is_system_admin);
        });
    }

    #[wasm_bindgen_test]
    fn message_state_resets_flags() {
        with_runtime(|| {
            let state = MessageState::default();
            state.set_error("NG");
            assert!(state.error.get().is_some());
            assert!(state.success.get().is_none());

            state.set_success("OK");
            assert!(state.success.get().is_some());
            assert!(state.error.get().is_none());

            state.clear();
            assert!(state.success.get().is_none());
            assert!(state.error.get().is_none());
        });
    }
}
