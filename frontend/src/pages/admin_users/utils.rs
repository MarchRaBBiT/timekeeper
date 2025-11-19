use crate::api::CreateUser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteFormState {
    pub username: String,
    pub full_name: String,
    pub password: String,
    pub role: String,
    pub is_system_admin: bool,
}

impl Default for InviteFormState {
    fn default() -> Self {
        Self {
            username: String::new(),
            full_name: String::new(),
            password: String::new(),
            role: "employee".to_string(),
            is_system_admin: false,
        }
    }
}

impl InviteFormState {
    pub fn is_valid(&self) -> bool {
        !(self.username.trim().is_empty()
            || self.full_name.trim().is_empty()
            || self.password.trim().is_empty())
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn to_request(&self) -> CreateUser {
        CreateUser {
            username: self.username.clone(),
            password: self.password.clone(),
            full_name: self.full_name.clone(),
            role: self.role.clone(),
            is_system_admin: self.is_system_admin,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageState {
    pub success: Option<String>,
    pub error: Option<String>,
}

impl MessageState {
    pub fn clear(&mut self) {
        self.success = None;
        self.error = None;
    }

    pub fn set_success(&mut self, message: impl Into<String>) {
        self.success = Some(message.into());
        self.error = None;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.success = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn invite_form_state_validation() {
        let mut state = InviteFormState::default();
        assert!(!state.is_valid());

        state.username = "admin".into();
        state.full_name = "System 管理者".into();
        state.password = "Password123!".into();

        assert!(state.is_valid());

        let request = state.to_request();
        assert_eq!(request.username, "admin");
        assert_eq!(request.full_name, "System 管理者");
        assert_eq!(request.role, "employee");
        assert!(!request.is_system_admin);
    }

    #[wasm_bindgen_test]
    fn message_state_resets_flags() {
        let mut state = MessageState::default();
        state.set_error("NG");
        assert!(state.error.is_some());
        assert!(state.success.is_none());

        state.set_success("OK");
        assert!(state.success.is_some());
        assert!(state.error.is_none());

        state.clear();
        assert!(state.success.is_none());
        assert!(state.error.is_none());
    }
}
