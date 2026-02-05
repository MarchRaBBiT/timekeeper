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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::{ApiClient, DataSubjectRequestType};
    use crate::test_support::ssr::with_local_runtime_async;
    use serde_json::json;

    fn subject_request_json(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "user_id": "user-1",
            "request_type": "access",
            "status": "pending",
            "details": null,
            "approved_by": null,
            "approved_at": null,
            "rejected_by": null,
            "rejected_at": null,
            "cancelled_at": null,
            "decision_comment": null,
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z"
        })
    }

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/mfa");
            then.status(200).json_body(json!({
                "enabled": false,
                "pending": false
            }));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/api/auth/change-password");
            then.status(200).json_body(json!({}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/subject-requests/me");
            then.status(200).json_body(json!([subject_request_json("sr-1")]));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/subject-requests");
            then.status(200).json_body(subject_request_json("sr-2"));
        });
        server.mock(|when, then| {
            when.method(DELETE).path("/api/subject-requests/sr-1");
            then.status(200).json_body(json!({}));
        });
        server
    }

    #[test]
    fn settings_view_model_dispatches_actions() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));

            let vm = use_settings_view_model();

            vm.change_password_action
                .dispatch(("current".into(), "newpass".into()));
            for _ in 0..10 {
                if vm.change_password_action.value().get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let _ = vm.change_password_action.value().get();

            vm.subject_request_view_model
                .create_action
                .dispatch(CreateDataSubjectRequest {
                    request_type: DataSubjectRequestType::Access,
                    details: None,
                });
            for _ in 0..10 {
                if vm
                    .subject_request_view_model
                    .create_action
                    .value()
                    .get()
                    .is_some()
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let _ = vm
                .subject_request_view_model
                .create_action
                .value()
                .get();

            vm.subject_request_view_model
                .cancel_action
                .dispatch("sr-1".into());
            for _ in 0..10 {
                if vm
                    .subject_request_view_model
                    .cancel_action
                    .value()
                    .get()
                    .is_some()
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let _ = vm
                .subject_request_view_model
                .cancel_action
                .value()
                .get();

            for _ in 0..10 {
                if vm.subject_request_view_model.requests_resource.get().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            let _ = vm.subject_request_view_model.requests_resource.get();

            runtime.dispose();
        });
    }
}
