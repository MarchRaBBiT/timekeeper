use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension, Router,
};
use serde_json::Value;
use sqlx::PgPool;
use timekeeper_backend::{
    handlers::admin::bulk_import::{import_departments, import_users},
    models::user::{User, UserRole},
    state::AppState,
    types::{DepartmentId, UserId},
    utils::{
        encryption::{encrypt_pii, hash_email},
        password::hash_password,
    },
};
use tower::ServiceExt;

mod support;
use support::{seed_user, test_config, test_pool};

async fn integration_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static GUARD: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    GUARD
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn import_router(pool: PgPool, user: User) -> Router {
    let state = AppState::new(pool, None, None, None, test_config());
    Router::new()
        .route(
            "/api/admin/bulk-import/departments",
            axum::routing::post(import_departments),
        )
        .route(
            "/api/admin/bulk-import/users",
            axum::routing::post(import_users),
        )
        .layer(Extension(user))
        .with_state(state)
}

async fn post_json(app: Router, uri: &str, body: serde_json::Value) -> (StatusCode, Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).expect("response should be json");
    (status, json)
}

// ─── Department import tests ──────────────────────────────────────────────────

#[tokio::test]
async fn import_departments_success() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    let csv = "name,parent_name\nBulkEngineering,\nBulkFrontend,BulkEngineering\nBulkBackend,BulkEngineering\n";
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 3);
    assert_eq!(json["failed"], 0);
    assert!(json["errors"].as_array().unwrap().is_empty());

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM departments WHERE name IN ('BulkEngineering','BulkFrontend','BulkBackend')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 3);

    // Verify parent_id is set correctly
    let parent: (Option<String>,) =
        sqlx::query_as("SELECT parent_id FROM departments WHERE name = 'BulkFrontend'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(parent.0.is_some(), "BulkFrontend should have a parent_id");
}

#[tokio::test]
async fn import_departments_unknown_parent() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    let csv = "name,parent_name\nOrphanDept,NonExistentParent\n";
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    assert!(json["failed"].as_i64().unwrap() > 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(!errors.is_empty());
    assert!(errors[0]["message"]
        .as_str()
        .unwrap()
        .contains("NonExistentParent"));
}

#[tokio::test]
async fn import_departments_ambiguous_parent_name() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let parent1 = DepartmentId::new().to_string();
    let parent2 = DepartmentId::new().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&parent1)
        .bind("SharedParent")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&parent2)
        .bind("SharedParent")
        .execute(&pool)
        .await
        .unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    let csv = "name,parent_name\nChildDept,SharedParent\n";
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors
        .iter()
        .any(|e| e["message"].as_str().unwrap().contains("ambiguous")));
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM departments WHERE name = 'ChildDept'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn import_departments_csv_duplicate() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool, sysadmin);

    let csv = "name,parent_name\nDupDept,\nDupDept,\n";
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    assert!(json["failed"].as_i64().unwrap() > 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors
        .iter()
        .any(|e| e["message"].as_str().unwrap().contains("DupDept")));
}

#[tokio::test]
async fn import_departments_circular_dependency() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool, sysadmin);

    // CircA -> CircB -> CircA forms a cycle
    let csv = "name,parent_name\nCircA,CircB\nCircB,CircA\n";
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    assert!(json["failed"].as_i64().unwrap() > 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("circular")));
}

#[tokio::test]
async fn import_departments_non_system_admin_forbidden() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let manager = seed_user(&pool, UserRole::Manager, false).await;
    let app = import_router(pool, manager);

    let csv = "name,parent_name\nSomeTeam,\n";
    let (status, _json) = post_json(
        app,
        "/api/admin/bulk-import/departments",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── User import tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn import_users_success() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let dept_id = DepartmentId::new().to_string();
    sqlx::query("INSERT INTO departments (id, name) VALUES ($1, $2)")
        .bind(&dept_id)
        .bind("BulkImportDept")
        .execute(&pool)
        .await
        .unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    let csv = concat!(
        "username,password,full_name,email,role,is_system_admin,department_name\n",
        "bulkalice,SecurePass1!,Alice Bulk,bulkalice@example.com,employee,false,BulkImportDept\n",
        "bulkbob,SecurePass2!,Bob Bulk,bulkbob@example.com,manager,false,\n",
    );
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/users",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 2);
    assert_eq!(json["failed"], 0);
    assert!(json["errors"].as_array().unwrap().is_empty());

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE username IN ('bulkalice','bulkbob')")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn import_users_invalid_role() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool, sysadmin);

    let csv = concat!(
        "username,password,full_name,email,role,is_system_admin,department_name\n",
        "badroleuser,SecurePass1!,Bad Role,badrole@example.com,superuser,false,\n",
    );
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/users",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    assert!(json["failed"].as_i64().unwrap() > 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors[0]["message"].as_str().unwrap().contains("superuser"));
}

#[tokio::test]
async fn import_users_reuses_create_user_validation() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    let csv = concat!(
        "username,password,full_name,email,role,is_system_admin,department_name\n",
        "bad username,SecurePass1!,Bad Username,bad@@example.com,employee,false,\n",
    );
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/users",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors
        .iter()
        .any(|e| e["message"].as_str().unwrap().contains("username:")));
    assert!(errors
        .iter()
        .any(|e| e["message"].as_str().unwrap().contains("email:")));

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'bad username'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn import_users_existing_username() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let existing_username = "existing_user".to_string();
    let existing_email = "existing_user@example.com".to_string();
    let existing_password_hash = hash_password("SecurePass1!").unwrap();
    let config = test_config();
    let full_name_enc = encrypt_pii("Existing User", &config).unwrap();
    let email_enc = encrypt_pii(&existing_email, &config).unwrap();
    let email_hash = hash_email(&existing_email, &config);
    let existing_id = UserId::new().to_string();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, email_hash, role, is_system_admin, \
         mfa_secret_enc, mfa_enabled_at, password_changed_at, failed_login_attempts, locked_until, lock_reason, \
         lockout_count, department_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, NOW(), 0, NULL, NULL, 0, NULL, NOW(), NOW())",
    )
    .bind(&existing_id)
    .bind(&existing_username)
    .bind(&existing_password_hash)
    .bind(&full_name_enc)
    .bind(&email_enc)
    .bind(&email_hash)
    .bind("employee")
    .bind(false)
    .execute(&pool)
    .await
    .unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool, sysadmin);

    let csv = format!(
        "username,password,full_name,email,role,is_system_admin,department_name\n\
         {},SecurePass1!,Dup User,dupimport@example.com,employee,false,\n",
        existing_username
    );
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/users",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["imported"], 0);
    assert!(json["failed"].as_i64().unwrap() > 0);
    let errors = json["errors"].as_array().unwrap();
    assert!(errors.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("already exists")));
}

#[tokio::test]
async fn import_users_partial_errors_no_commit() {
    let _guard = integration_guard().await;
    let pool = test_pool().await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let sysadmin = seed_user(&pool, UserRole::Employee, true).await;
    let app = import_router(pool.clone(), sysadmin);

    // Row 1: valid; Row 2: invalid role — nothing should be committed
    let csv = concat!(
        "username,password,full_name,email,role,is_system_admin,department_name\n",
        "partialok,SecurePass1!,Partial OK,partialok@example.com,employee,false,\n",
        "partialbd,SecurePass2!,Partial Bad,partialbad@example.com,badrol,false,\n",
    );
    let (status, json) = post_json(
        app,
        "/api/admin/bulk-import/users",
        serde_json::json!({ "csv_data": csv }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json["imported"], 0,
        "nothing should be committed when any row fails"
    );

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE username IN ('partialok','partialbd')")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count.0, 0,
        "neither user should be inserted when validation fails"
    );
}
