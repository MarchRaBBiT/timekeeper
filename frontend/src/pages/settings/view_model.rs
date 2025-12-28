use super::repository;
use crate::api::ApiClient;
use crate::pages::mfa::view_model::MfaViewModel;
use leptos::*;

#[derive(Clone)]
pub struct SettingsViewModel {
    pub change_password_action: Action<(String, String), Result<(), String>>,
    pub mfa_view_model: MfaViewModel, // Reuse existing MFA logic
}

pub fn use_settings_view_model() -> SettingsViewModel {
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let change_password_action = create_action(move |(current, new): &(String, String)| {
        let api = api.clone();
        let current = current.clone();
        let new = new.clone();
        async move { repository::change_password(api, current, new).await }
    });

    let mfa_view_model = crate::pages::mfa::view_model::use_mfa_view_model();

    SettingsViewModel {
        change_password_action,
        mfa_view_model,
    }
}
