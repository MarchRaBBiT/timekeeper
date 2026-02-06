use super::{
    repository::AdminRepository,
    utils::{
        next_allowed_weekly_start, RequestFilterState, SubjectRequestFilterState,
        WeeklyHolidayFormState,
    },
};
use crate::api::{
    ApiClient, ApiError, CreateWeeklyHolidayRequest, SubjectRequestListResponse, UserResponse,
    WeeklyHolidayResponse,
};
use crate::state::auth::use_auth;
use crate::utils::time::today_in_app_tz;
use leptos::*;

fn is_admin_user(user: Option<&UserResponse>) -> bool {
    user.map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
        .unwrap_or(false)
}

fn is_system_admin_user(user: Option<&UserResponse>) -> bool {
    user.map(|user| user.is_system_admin).unwrap_or(false)
}

fn validate_action_id(id: &str) -> Result<(), ApiError> {
    if id.trim().is_empty() {
        Err(ApiError::validation("リクエストIDを取得できませんでした。"))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub struct AdminViewModel {
    pub users_resource: Resource<bool, Result<Vec<UserResponse>, ApiError>>,
    pub weekly_holiday_state: WeeklyHolidayFormState,
    pub reload_weekly: RwSignal<u32>,
    pub weekly_holidays_resource:
        Resource<(bool, u32), Result<Vec<WeeklyHolidayResponse>, ApiError>>,
    pub weekly_action_message: RwSignal<Option<String>>,
    pub weekly_action_error: RwSignal<Option<ApiError>>,
    pub create_weekly_action:
        Action<CreateWeeklyHolidayRequest, Result<WeeklyHolidayResponse, ApiError>>,
    pub delete_weekly_action: Action<String, Result<(), ApiError>>,
    pub requests_filter: RequestFilterState,
    pub requests_resource: Resource<
        (bool, super::utils::RequestFilterSnapshot, u32),
        Result<serde_json::Value, ApiError>,
    >,
    pub request_action: Action<RequestActionPayload, Result<(), ApiError>>,
    pub requests_action_error: RwSignal<Option<ApiError>>,
    pub reload_requests: RwSignal<u32>,
    pub subject_request_filter: SubjectRequestFilterState,
    pub subject_requests_resource: Resource<
        (bool, super::utils::SubjectRequestFilterSnapshot, u32),
        Result<SubjectRequestListResponse, ApiError>,
    >,
    pub subject_request_action: Action<SubjectRequestActionPayload, Result<(), ApiError>>,
    pub subject_request_action_error: RwSignal<Option<ApiError>>,
    pub reload_subject_requests: RwSignal<u32>,
    pub repository: AdminRepository,
    // Add other section states here as needed
}

pub fn use_admin_view_model() -> AdminViewModel {
    let (auth, _) = use_auth();
    // ApiClient is typically provided by AuthProvider/router context.
    // We use a fallback to provide robustness if AuthProvider is not mounted.
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));

    // Admin Access Check
    let admin_allowed = create_memo(move |_| is_admin_user(auth.get().user.as_ref()));

    let system_admin_allowed = create_memo(move |_| is_system_admin_user(auth.get().user.as_ref()));

    // Resources
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

    // Weekly Holidays Section State
    let default_start =
        next_allowed_weekly_start(today_in_app_tz(), system_admin_allowed.get_untracked())
            .format("%Y-%m-%d")
            .to_string();
    let weekly_holiday_state = WeeklyHolidayFormState::new("0", default_start);

    let reload_weekly = create_rw_signal(0u32);
    let repo_weekly = repo.clone();
    let weekly_holidays_resource = create_resource(
        move || (admin_allowed.get(), reload_weekly.get()),
        move |(allowed, _)| {
            let repo = repo_weekly.clone();
            async move {
                if !allowed {
                    Ok(Vec::new())
                } else {
                    repo.list_weekly_holidays().await
                }
            }
        },
    );

    let weekly_action_message = create_rw_signal(None);
    let weekly_action_error = create_rw_signal(None);

    let repo_create = repo.clone();
    let create_weekly_action = create_action(move |payload: &CreateWeeklyHolidayRequest| {
        let repo = repo_create.clone();
        let payload = payload.clone();
        async move { repo.create_weekly_holiday(payload).await }
    });

    let repo_delete = repo.clone();
    let delete_weekly_action = create_action(move |id: &String| {
        let repo = repo_delete.clone();
        let id = id.clone();
        async move { repo.delete_weekly_holiday(&id).await }
    });

    // Requests Section State
    let requests_filter = RequestFilterState::new();
    let filter_state_for_snapshot = requests_filter.clone();
    let snapshot = Signal::derive(move || filter_state_for_snapshot.snapshot());

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

    // Subject Requests Section State
    let subject_request_filter = SubjectRequestFilterState::new();
    let subject_filter_state = subject_request_filter.clone();
    let subject_snapshot = Signal::derive(move || subject_filter_state.snapshot());

    let repo_for_subject_requests = repo.clone();
    let reload_subject_requests = create_rw_signal(0u32);
    let subject_requests_resource = create_resource(
        move || {
            (
                admin_allowed.get(),
                subject_snapshot.get(),
                reload_subject_requests.get(),
            )
        },
        move |(allowed, snapshot, _reload)| {
            let repo = repo_for_subject_requests.clone();
            async move {
                if !allowed {
                    Ok(SubjectRequestListResponse {
                        page: snapshot.page as i64,
                        per_page: snapshot.per_page as i64,
                        total: 0,
                        items: Vec::new(),
                    })
                } else {
                    repo.list_subject_requests(
                        snapshot.status.clone(),
                        snapshot.request_type.clone(),
                        snapshot.user_id.clone(),
                        snapshot.page as i64,
                        snapshot.per_page as i64,
                    )
                    .await
                }
            }
        },
    );

    let repo_subject_action = repo.clone();
    let subject_request_action_error = create_rw_signal(None::<ApiError>);
    let subject_request_action = create_action(move |payload: &SubjectRequestActionPayload| {
        let repo = repo_subject_action.clone();
        let payload = payload.clone();
        async move {
            if let Err(err) = validate_action_id(&payload.id) {
                Err(err)
            } else if payload.approve {
                repo.approve_subject_request(&payload.id, &payload.comment)
                    .await
            } else {
                repo.reject_subject_request(&payload.id, &payload.comment)
                    .await
            }
        }
    });

    AdminViewModel {
        users_resource,
        weekly_holiday_state,
        reload_weekly,
        weekly_holidays_resource,
        weekly_action_message,
        weekly_action_error,
        create_weekly_action,
        delete_weekly_action,
        requests_filter,
        requests_resource,
        request_action,
        requests_action_error,
        reload_requests,
        subject_request_filter,
        subject_requests_resource,
        subject_request_action,
        subject_request_action_error,
        reload_subject_requests,
        repository: repo,
    }
}

#[derive(Clone)]
pub struct RequestActionPayload {
    pub id: String,
    pub comment: String,
    pub approve: bool,
}

#[derive(Clone)]
pub struct SubjectRequestActionPayload {
    pub id: String,
    pub comment: String,
    pub approve: bool,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::render_to_string;

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
            when.method(GET).path("/api/admin/holidays/weekly");
            then.status(200).json_body(serde_json::json!([]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/requests");
            then.status(200)
                .json_body(serde_json::json!({ "items": [] }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/subject-requests");
            then.status(200).json_body(serde_json::json!({
                "page": 1,
                "per_page": 20,
                "total": 0,
                "items": []
            }));
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
    fn helper_auth_and_action_validation_cover_paths() {
        let system_admin = UserResponse {
            id: "u1".into(),
            username: "system".into(),
            full_name: "System Admin".into(),
            role: "user".into(),
            is_system_admin: true,
            mfa_enabled: true,
        };
        let role_admin = UserResponse {
            id: "u2".into(),
            username: "admin".into(),
            full_name: "Role Admin".into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: false,
        };
        let normal_user = UserResponse {
            id: "u3".into(),
            username: "member".into(),
            full_name: "Member".into(),
            role: "member".into(),
            is_system_admin: false,
            mfa_enabled: false,
        };

        assert!(is_admin_user(Some(&system_admin)));
        assert!(is_system_admin_user(Some(&system_admin)));
        assert!(is_admin_user(Some(&role_admin)));
        assert!(!is_system_admin_user(Some(&role_admin)));
        assert!(!is_admin_user(Some(&normal_user)));
        assert!(!is_admin_user(None));
        assert!(!is_system_admin_user(None));

        assert!(validate_action_id("req-1").is_ok());
        assert!(validate_action_id("   ").is_err());
    }
}
