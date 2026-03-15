use axum::{
    extract::{Request, State},
    http::{header, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

/// CSRF protection middleware for cookie-authenticated mutation endpoints.
///
/// Skips:
/// - Safe methods (GET, HEAD, OPTIONS, TRACE)
/// - Requests with `Authorization: Bearer …` (those are programmatic clients, not cookies)
///
/// All other state-changing requests must have a valid `Origin` or `Referer` header
/// matching the configured `CORS_ALLOW_ORIGINS`.
pub async fn csrf_check(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method();
    if matches!(
        method,
        &Method::GET | &Method::HEAD | &Method::OPTIONS | &Method::TRACE
    ) {
        return next.run(request).await;
    }

    // Bearer-token requests originate from programmatic clients, not browsers with cookies.
    // CSRF via cookie-injection cannot occur in those paths.
    let has_bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.len() >= 7 && v[..7].eq_ignore_ascii_case("bearer "))
        .unwrap_or(false);

    if has_bearer {
        return next.run(request).await;
    }

    match crate::utils::security::verify_request_origin(request.headers(), &state.config) {
        Ok(()) => next.run(request).await,
        Err(err) => err.into_response(),
    }
}
