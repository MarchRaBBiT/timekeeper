use super::repository;
use crate::api::{ApiClient, ApiError, CreateDataSubjectRequest, DataSubjectRequestResponse};
use crate::pages::mfa::view_model::MfaViewModel;
use leptos::*;

#[derive(Clone)]
pub struct SettingsViewModel {
    pub change_password_action: Action<(String, String), Result<(), ApiError>>,
    pub mfa_view_model: MfaViewModel, // Reuse existing MFA logic
    pub subject_request_view_model: SubjectRequestViewModel,
}

#[derive(Clone)]
pub struct SubjectRequestViewModel {
    pub requests_resource: Resource<u32, Result<Vec<DataSubjectRequestResponse>, ApiError>>,
    pub reload: RwSignal<u32>,
    pub create_action:
        Action<CreateDataSubjectRequest, Result<DataSubjectRequestResponse, ApiError>>,
    pub cancel_action: Action<String, Result<(), ApiError>>,
}

pub fn use_settings_view_model() -> SettingsViewModel {
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let api_for_password = api.clone();
    let change_password_action = create_action(move |(current, new): &(String, String)| {
        let api = api_for_password.clone();
        let current = current.clone();
        let new = new.clone();
        async move { repository::change_password(api, current, new).await }
    });

    let subject_reload = create_rw_signal(0u32);
    let list_api = api.clone();
    let requests_resource = create_resource(
        move || subject_reload.get(),
        move |_| {
            let api = list_api.clone();
            async move { api.list_my_subject_requests().await }
        },
    );

    let create_api = api.clone();
    let create_subject_action = create_action(move |payload: &CreateDataSubjectRequest| {
        let api = create_api.clone();
        let payload = payload.clone();
        async move { api.create_subject_request(payload).await }
    });

    let cancel_api = api.clone();
    let cancel_subject_action = create_action(move |id: &String| {
        let api = cancel_api.clone();
        let id = id.clone();
        async move { api.cancel_subject_request(&id).await }
    });

    let subject_request_view_model = SubjectRequestViewModel {
        requests_resource,
        reload: subject_reload,
        create_action: create_subject_action,
        cancel_action: cancel_subject_action,
    };

    let mfa_view_model = crate::pages::mfa::view_model::use_mfa_view_model();

    SettingsViewModel {
        change_password_action,
        mfa_view_model,
        subject_request_view_model,
    }
}
