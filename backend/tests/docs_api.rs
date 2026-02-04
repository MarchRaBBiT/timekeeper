use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::Value;
use timekeeper_backend::docs;
use tower::ServiceExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

fn swagger_router() -> Router {
    let openapi = docs::ApiDoc::openapi();
    Router::new().merge(SwaggerUi::new("/api/docs").url("/api-doc/openapi.json", openapi))
}

#[test]
fn openapi_includes_login_path_and_bearer_scheme() {
    let openapi = docs::ApiDoc::openapi();
    let json = serde_json::to_value(&openapi).expect("serialize openapi");

    let paths = json
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("paths object");
    assert!(paths.contains_key("/api/auth/login"));

    let bearer = json
        .pointer("/components/securitySchemes/BearerAuth")
        .expect("BearerAuth scheme");
    assert_eq!(bearer.get("type").and_then(Value::as_str), Some("http"));
    assert_eq!(
        bearer.get("scheme").and_then(Value::as_str),
        Some("bearer")
    );
}

#[tokio::test]
async fn swagger_ui_routes_respond() {
    let app = swagger_router();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docs")
                .body(Body::empty())
                .expect("build docs request"),
        )
        .await
        .expect("call swagger ui");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(location, "/api/docs/");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/swagger-initializer.js")
                .body(Body::empty())
                .expect("build swagger initializer request"),
        )
        .await
        .expect("call swagger initializer");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("javascript"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read initializer body");
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("SwaggerUIBundle"));
}

#[tokio::test]
async fn openapi_json_route_serves_spec() {
    let app = swagger_router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-doc/openapi.json")
                .body(Body::empty())
                .expect("build openapi request"),
        )
        .await
        .expect("call openapi route");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse openapi json");

    let paths = json
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("paths object");
    assert!(paths.contains_key("/api/auth/login"));
}
