use crate::api::{
    ApiClient, ApiError, CreateWeeklyHolidayRequest, SubjectRequestListResponse, UserResponse,
    WeeklyHolidayResponse,
};
use crate::pages::admin::{
    repository::AdminRepository,
    utils::{
        next_allowed_weekly_start, SubjectRequestFilterSnapshot, SubjectRequestFilterState,
        WeeklyHolidayFormState,
    },
    view_model::SubjectRequestActionPayload,
};
use crate::state::auth::use_auth;
use crate::utils::time::today_in_app_tz;
use leptos::*;

fn is_strict_admin(user: Option<&UserResponse>) -> bool {
    user.map(|u| u.is_system_admin || u.role.eq_ignore_ascii_case("admin"))
        .unwrap_or(false)
}

fn is_system_admin(user: Option<&UserResponse>) -> bool {
    user.map(|u| u.is_system_admin).unwrap_or(false)
}

fn validate_action_id(id: &str) -> Result<(), ApiError> {
    if id.trim().is_empty() {
        Err(ApiError::validation(rust_i18n::t!(
            "pages.admin.validation.request_id_missing"
        )))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub struct AdminSettingsViewModel {
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
    pub subject_request_filter: SubjectRequestFilterState,
    pub subject_requests_resource: Resource<
        (bool, SubjectRequestFilterSnapshot, u32),
        Result<SubjectRequestListResponse, ApiError>,
    >,
    pub subject_request_action: Action<SubjectRequestActionPayload, Result<(), ApiError>>,
    pub subject_request_action_error: RwSignal<Option<ApiError>>,
    pub reload_subject_requests: RwSignal<u32>,
    pub repository: AdminRepository,
}

pub fn use_admin_settings_view_model() -> AdminSettingsViewModel {
    let (auth, _) = use_auth();
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = AdminRepository::new_with_client(std::rc::Rc::new(api));

    let strict_admin_allowed = create_memo(move |_| is_strict_admin(auth.get().user.as_ref()));
    let system_admin_allowed = create_memo(move |_| is_system_admin(auth.get().user.as_ref()));

    // Users
    let repo_users = repo.clone();
    let users_resource = create_resource(
        move || strict_admin_allowed.get(),
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

    // Weekly Holidays
    let default_start =
        next_allowed_weekly_start(today_in_app_tz(), system_admin_allowed.get_untracked())
            .format("%Y-%m-%d")
            .to_string();
    let weekly_holiday_state = WeeklyHolidayFormState::new("0", default_start);

    let reload_weekly = create_rw_signal(0u32);
    let repo_weekly = repo.clone();
    let weekly_holidays_resource = create_resource(
        move || (strict_admin_allowed.get(), reload_weekly.get()),
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

    // Subject Requests
    let subject_request_filter = SubjectRequestFilterState::new();
    let subject_filter_state = subject_request_filter;
    let subject_snapshot = Signal::derive(move || subject_filter_state.snapshot());

    let repo_for_subject = repo.clone();
    let reload_subject_requests = create_rw_signal(0u32);
    let subject_requests_resource = create_resource(
        move || {
            (
                system_admin_allowed.get(),
                subject_snapshot.get(),
                reload_subject_requests.get(),
            )
        },
        move |(allowed, snapshot, _)| {
            let repo = repo_for_subject.clone();
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

    AdminSettingsViewModel {
        users_resource,
        weekly_holiday_state,
        reload_weekly,
        weekly_holidays_resource,
        weekly_action_message,
        weekly_action_error,
        create_weekly_action,
        delete_weekly_action,
        subject_request_filter,
        subject_requests_resource,
        subject_request_action,
        subject_request_action_error,
        reload_subject_requests,
        repository: repo,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth, regular_user};
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn helper_is_strict_admin_covers_paths() {
        let system_admin = UserResponse {
            id: "u1".into(),
            username: "sys".into(),
            full_name: "Sys".into(),
            role: "user".into(),
            is_system_admin: true,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        let role_admin = UserResponse {
            id: "u2".into(),
            username: "admin".into(),
            full_name: "Admin".into(),
            role: "admin".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        let manager = UserResponse {
            id: "u3".into(),
            username: "mgr".into(),
            full_name: "Mgr".into(),
            role: "manager".into(),
            is_system_admin: false,
            mfa_enabled: false,
            is_locked: false,
            locked_until: None,
            failed_login_attempts: 0,
            password_expiry_warning_days: None,
            department_id: None,
        };
        assert!(is_strict_admin(Some(&system_admin)));
        assert!(is_strict_admin(Some(&role_admin)));
        assert!(!is_strict_admin(Some(&manager)));
        assert!(!is_strict_admin(None));
    }

    #[test]
    fn settings_view_model_returns_empty_for_non_admin() {
        let html = render_to_string(move || {
            provide_auth(Some(regular_user()));
            let vm = use_admin_settings_view_model();
            assert_eq!(vm.subject_request_filter.snapshot().page, 1);
            view! { <div>{vm.subject_request_filter.snapshot().per_page}</div> }
        });
        assert!(html.contains("20"));
    }

    #[test]
    fn settings_view_model_initializes_with_admin_context() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/users");
            then.status(200).json_body(serde_json::json!([]));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/admin/holidays/weekly");
            then.status(200).json_body(serde_json::json!([]));
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

        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let vm = use_admin_settings_view_model();
            assert_eq!(vm.subject_request_filter.snapshot().page, 1);
            assert_eq!(
                vm.weekly_holiday_state.weekday_signal().get_untracked(),
                "0"
            );
            view! { <div>{"ok"}</div> }
        });
        assert!(html.contains("ok"));
    }
}
