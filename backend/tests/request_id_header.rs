use axum::{body::Body, http::Request};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_request_id_header_added_to_response() {
    let app = axum::Router::new()
        .route("/test", axum::routing::get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(
            timekeeper_backend::middleware::request_id::request_id,
        ));

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.headers().contains_key("x-request-id"));
    let id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(Uuid::parse_str(id).is_ok());
}

#[tokio::test]
async fn test_request_id_header_persists_client_id() {
    let app = axum::Router::new()
        .route("/test", axum::routing::get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(
            timekeeper_backend::middleware::request_id::request_id,
        ));

    let client_id = "client-req-123";
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-request-id", client_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.headers().get("x-request-id").unwrap(), client_id);
}

#[tokio::test]
async fn test_request_id_header_persists_correlation_id() {
    let app = axum::Router::new()
        .route("/test", axum::routing::get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(
            timekeeper_backend::middleware::request_id::request_id,
        ));

    let correlation_id = "corr-req-456";
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("x-correlation-id", correlation_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        correlation_id
    );
}
