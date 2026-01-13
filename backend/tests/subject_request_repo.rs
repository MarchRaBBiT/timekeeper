use chrono::{Duration, Utc};
use std::sync::OnceLock;
use timekeeper_backend::{
    models::{
        request::RequestStatus,
        subject_request::{DataSubjectRequest, DataSubjectRequestType},
        user::UserRole,
    },
    repositories::subject_request::{self, SubjectRequestFilters},
};
use tokio::sync::Mutex;

#[path = "support/mod.rs"]
mod support;

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().await
}

async fn reset_subject_requests(pool: &sqlx::PgPool) {
    sqlx::query("TRUNCATE subject_requests")
        .execute(pool)
        .await
        .expect("truncate subject_requests");
}

#[tokio::test]
async fn subject_request_lists_for_user_in_descending_order() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let now = Utc::now();

    let mut older = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Access,
        Some("old".to_string()),
        now - Duration::days(2),
    );
    older.created_at = now - Duration::days(2);
    older.updated_at = older.created_at;

    let mut newer = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Delete,
        Some("new".to_string()),
        now - Duration::days(1),
    );
    newer.created_at = now - Duration::days(1);
    newer.updated_at = newer.created_at;

    subject_request::insert_subject_request(&pool, &older)
        .await
        .expect("insert older request");
    subject_request::insert_subject_request(&pool, &newer)
        .await
        .expect("insert newer request");

    let user_id = user.id.to_string();
    let items = subject_request::list_subject_requests_by_user(&pool, &user_id)
        .await
        .expect("list subject requests");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, newer.id);
    assert_eq!(items[1].id, older.id);
}

#[tokio::test]
async fn subject_request_filters_by_status_type_and_user() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user_one = support::seed_user(&pool, UserRole::Employee, false).await;
    let user_two = support::seed_user(&pool, UserRole::Employee, false).await;
    let now = Utc::now();

    let mut req_one = DataSubjectRequest::new(
        user_one.id.to_string(),
        DataSubjectRequestType::Access,
        None,
        now - Duration::hours(2),
    );
    req_one.created_at = now - Duration::hours(2);
    req_one.updated_at = req_one.created_at;

    let mut req_two = DataSubjectRequest::new(
        user_one.id.to_string(),
        DataSubjectRequestType::Delete,
        None,
        now - Duration::hours(1),
    );
    req_two.status = RequestStatus::Approved;
    req_two.created_at = now - Duration::hours(1);
    req_two.updated_at = req_two.created_at;

    let mut req_three = DataSubjectRequest::new(
        user_two.id.to_string(),
        DataSubjectRequestType::Delete,
        None,
        now,
    );
    req_three.status = RequestStatus::Approved;
    req_three.created_at = now;
    req_three.updated_at = req_three.created_at;

    for req in [&req_one, &req_two, &req_three] {
        subject_request::insert_subject_request(&pool, req)
            .await
            .expect("insert subject request");
    }

    let filters = SubjectRequestFilters {
        status: Some(RequestStatus::Approved),
        request_type: Some(DataSubjectRequestType::Delete),
        user_id: Some(user_one.id.to_string()),
        from: None,
        to: None,
    };

    let (items, total) = subject_request::list_subject_requests(&pool, &filters, 20, 0)
        .await
        .expect("list subject requests with filters");

    assert_eq!(total, 1);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, req_two.id);
}

#[tokio::test]
async fn subject_request_approve_and_reject_updates_status() {
    let _guard = integration_guard().await;
    let pool = support::test_pool().await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    reset_subject_requests(&pool).await;

    let user = support::seed_user(&pool, UserRole::Employee, false).await;
    let admin = support::seed_user(&pool, UserRole::Admin, false).await;
    let now = Utc::now();

    let mut approve_request = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Rectify,
        Some("fix".to_string()),
        now,
    );
    approve_request.created_at = now - Duration::minutes(10);
    approve_request.updated_at = approve_request.created_at;
    subject_request::insert_subject_request(&pool, &approve_request)
        .await
        .expect("insert approval request");

    let admin_id = admin.id.to_string();
    let rows =
        subject_request::approve_subject_request(&pool, &approve_request.id, &admin_id, "ok", now)
            .await
            .expect("approve request");
    assert_eq!(rows, 1);

    let approved = subject_request::fetch_subject_request(&pool, &approve_request.id)
        .await
        .expect("fetch request")
        .expect("request exists");
    assert!(matches!(approved.status, RequestStatus::Approved));
    assert_eq!(approved.approved_by.as_deref(), Some(admin_id.as_str()));
    assert_eq!(approved.decision_comment.as_deref(), Some("ok"));

    let mut reject_request = DataSubjectRequest::new(
        user.id.to_string(),
        DataSubjectRequestType::Stop,
        Some("stop".to_string()),
        now,
    );
    reject_request.created_at = now - Duration::minutes(5);
    reject_request.updated_at = reject_request.created_at;
    subject_request::insert_subject_request(&pool, &reject_request)
        .await
        .expect("insert reject request");

    let rows =
        subject_request::reject_subject_request(&pool, &reject_request.id, &admin_id, "no", now)
            .await
            .expect("reject request");
    assert_eq!(rows, 1);

    let rejected = subject_request::fetch_subject_request(&pool, &reject_request.id)
        .await
        .expect("fetch request")
        .expect("request exists");
    assert!(matches!(rejected.status, RequestStatus::Rejected));
    assert_eq!(rejected.rejected_by.as_deref(), Some(admin_id.as_str()));
    assert_eq!(rejected.decision_comment.as_deref(), Some("no"));
}
