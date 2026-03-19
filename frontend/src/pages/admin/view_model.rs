use super::{
    repository::AdminRepository,
    utils::{validate_action_id, RequestFilterState},
};
use crate::api::{ApiClient, ApiError, UserResponse};
use crate::state::auth::use_auth;
use leptos::*;

fn is_admin_user(user: Option<&UserResponse>) -> bool {
    user.map(|user| {
        user.is_system_admin
            || user.role.eq_ignore_ascii_case("manager")
            || user.role.eq_ignore_ascii_case("admin")
    })
    .unwrap_or(false)
}

#[derive(Clone)]
pub struct AdminViewModel {
    pub users_resource: Resource<bool, Result<Vec<UserResponse>, ApiError>>,
    pub requests_filter: RequestFilterState,
    pub requests_resource: Resource<
        (bool, super::utils::RequestFilterSnapshot, u32),
        Result<serde_json::Value, ApiError>,
    >,
    pub request_action: Action<RequestActionPayload, Result<(), ApiError>>,
    pub requests_action_error: RwSignal<Option<ApiError>>,
    pub reload_requests: RwSignal<u32>,
}

pub fn use_admin_view_model() -> AdminViewModel {
    let (auth, _) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));

    let admin_allowed = create_memo(move |_| is_admin_user(auth.get().user.as_ref()));

    let repo_users = repo.clone();
    let users_resource = create_resource(
        move || admin_allowed.get(),
        move |allowed| {
            let repo = repo_users.clone();
            async move {
                if allowed {
                    repo.fetch_users().await
                } else {
                    Ok(Vec::new())
                }
            }
        },
    );

    let requests_filter = RequestFilterState::new();
    let filter_state_for_snapshot = requests_filter;
    let snapshot = create_memo(move |_| filter_state_for_snapshot.snapshot());

    let repo_for_requests = repo.clone();
    let reload_requests = create_rw_signal(0u32);
    let requests_resource = create_resource(
        move || (admin_allowed.get(), snapshot.get(), reload_requests.get()),
        move |(allowed, snapshot, _)| {
            let repo = repo_for_requests.clone();
            async move {
                if !allowed {
                    Ok(serde_json::Value::Null)
                } else {
                    repo.list_requests(
                        snapshot.status.clone(),
                        snapshot.user_id.clone(),
                        snapshot.page,
                        snapshot.per_page,
                    )
                    .await
                }
            }
        },
    );

    let repo_action = repo.clone();
    let requests_action_error = create_rw_signal(None::<ApiError>);
    let request_action = create_action(move |payload: &RequestActionPayload| {
        let repo = repo_action.clone();
        let payload = payload.clone();
        async move {
            if let Err(err) = validate_action_id(&payload.id) {
                Err(err)
            } else if payload.approve {
                repo.approve_request(&payload.id, &payload.comment).await
            } else {
                repo.reject_request(&payload.id, &payload.comment).await
            }
        }
    });

    AdminViewModel {
        users_resource,
        requests_filter,
        requests_resource,
        request_action,
        requests_action_error,
        reload_requests,
    }
}

#[derive(Clone)]
pub struct RequestActionPayload {
    pub id: String,
    pub comment: String,
    pub approve: bool,
}

// Type alias kept for backward compatibility: subject_requests component imports this type.
pub type SubjectRequestActionPayload = RequestActionPayload;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth, regular_user, set_test_locale};
    use crate::test_support::ssr::{render_to_string, with_local_runtime_async};

    async fn wait_until(mut condition: impl FnMut() -> bool) {
        for _ in 0..30 {
            if condition() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    #[test]
    fn helper_is_admin_user_and_validation_cover_paths() {
        let system_admin = UserResponse {
            id: "u1".into(),
            username: "system".into(),
            full_name: "System Admin".into(),
            role: "user".into(),
            is_system_admin: true,
            mfa_enabled: true,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        let role_admin = UserResponse {
            id: "u2".into(),
            username: "admin".into(),
            full_name: "Role Admin".into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        let role_admin_upper = UserResponse {
            role: "ADMIN".into(),
            ..role_admin.clone()
        };
        let normal_user = UserResponse {
            id: "u3".into(),
            username: "member".into(),
            full_name: "Member".into(),
            role: "member".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        let manager = UserResponse {
            id: "u4".into(),
            username: "mgr".into(),
            full_name: "Manager".into(),
            role: "manager".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };

        assert!(is_admin_user(Some(&system_admin)));
        assert!(is_admin_user(Some(&role_admin)));
        assert!(is_admin_user(Some(&role_admin_upper)));
        assert!(is_admin_user(Some(&manager)));
        assert!(!is_admin_user(Some(&normal_user)));
        assert!(!is_admin_user(None));

        assert!(validate_action_id("req-1").is_ok());
        assert!(validate_action_id("  req-2 ").is_ok());
        assert!(validate_action_id("   ").is_err());
    }

    #[test]
    fn admin_view_model_initializes_with_admin_context() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([{
                "id": "u1",
                "username": "alice",
                "full_name": "Alice Example",
                "role": "admin",
                "is_system_admin": true,
                "mfa_enabled": false
            }]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/requests");
            then.status(200)
                .json_body(serde_json::json!({ "items": [] }));
        });

        let server = server.clone();
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_admin_view_model();
            assert_eq!(vm.requests_filter.snapshot().page, 1);
            view! { <div>{vm.requests_filter.snapshot().per_page}</div> }
        });
        assert!(html.contains("20"));
    }

    #[test]
    fn admin_view_model_resources_and_actions_cover_runtime_paths() {
        with_local_runtime_async(|| async {
            let _locale = set_test_locale("ja");
            let runtime = leptos::create_runtime();
            let server = MockServer::start();
            server.mock(|when, then| {
                when.method(GET).path("/api/admin/users");
                then.status(200).json_body(serde_json::json!([{
                    "id": "u1",
                    "username": "alice",
                    "full_name": "Alice Example",
                    "role": "admin",
                    "is_system_admin": true,
                    "mfa_enabled": true
                }]));
            });
            server.mock(|when, then| {
                when.method(GET).path("/api/admin/requests");
                then.status(200)
                    .json_body(serde_json::json!({ "items": [{ "id": "req-1" }] }));
            });
            server.mock(|when, then| {
                when.method(PUT).path("/api/admin/requests/req-1/approve");
                then.status(200)
                    .json_body(serde_json::json!({ "status": "approved" }));
            });
            server.mock(|when, then| {
                when.method(PUT).path("/api/admin/requests/req-1/reject");
                then.status(400).json_body(serde_json::json!({
                    "error": "reject failed",
                    "code": "UNKNOWN"
                }));
            });

            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_admin_view_model();

            wait_until(|| {
                vm.users_resource.get().is_some() && vm.requests_resource.get().is_some()
            })
            .await;

            match vm.users_resource.get() {
                Some(Ok(rows)) => assert_eq!(rows.len(), 1),
                other => panic!("users_resource not ready: {:?}", other),
            }
            match vm.requests_resource.get() {
                Some(Ok(rows)) => assert!(rows.get("items").is_some()),
                other => panic!("requests_resource not ready: {:?}", other),
            }

            vm.request_action.dispatch(RequestActionPayload {
                id: "req-1".to_string(),
                comment: "ok".to_string(),
                approve: true,
            });
            wait_until(|| vm.request_action.pending().get_untracked()).await;
            wait_until(|| !vm.request_action.pending().get_untracked()).await;
            assert!(matches!(vm.request_action.value().get(), Some(Ok(()))));

            vm.request_action.dispatch(RequestActionPayload {
                id: "req-1".to_string(),
                comment: "deny".to_string(),
                approve: false,
            });
            wait_until(|| vm.request_action.pending().get_untracked()).await;
            wait_until(|| !vm.request_action.pending().get_untracked()).await;
            match vm.request_action.value().get() {
                Some(Err(err)) => assert_eq!(err.error, "reject failed"),
                other => panic!("expected reject_request error, got {:?}", other),
            }

            vm.request_action.dispatch(RequestActionPayload {
                id: "   ".to_string(),
                comment: "".to_string(),
                approve: true,
            });
            wait_until(|| vm.request_action.pending().get_untracked()).await;
            wait_until(|| !vm.request_action.pending().get_untracked()).await;
            match vm.request_action.value().get() {
                Some(Err(err)) => {
                    assert_eq!(err.error, "リクエストIDを取得できませんでした。");
                    assert_eq!(err.code, "VALIDATION_ERROR");
                }
                other => panic!("expected request validation error, got {:?}", other),
            }

            runtime.dispose();
        });
    }

    #[test]
    fn admin_view_model_uses_default_payloads_for_non_admin() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            provide_auth(Some(regular_user()));
            let vm = use_admin_view_model();

            wait_until(|| {
                vm.users_resource.get().is_some() && vm.requests_resource.get().is_some()
            })
            .await;

            match vm.users_resource.get() {
                Some(Ok(rows)) => assert!(rows.is_empty()),
                other => panic!("users_resource should be empty for non-admin: {:?}", other),
            }
            match vm.requests_resource.get() {
                Some(Ok(value)) => assert!(value.is_null()),
                other => panic!(
                    "requests_resource should return null for non-admin: {:?}",
                    other
                ),
            }

            runtime.dispose();
        });
    }
}
