use axum::{
    extract::{Request, State},
    http::{header::USER_AGENT, HeaderMap, Method},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    config::Config,
    models::user::User,
    services::audit_log::{AuditLogEntry, AuditLogService},
};

struct AuditEventDescriptor {
    event_type: &'static str,
    target_type: Option<&'static str>,
    target_id: Option<String>,
}

pub async fn audit_log(
    State((_, config)): State<(crate::db::connection::DbPool, Config)>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let descriptor = classify_event(&method, &path);

    if descriptor.is_none() {
        return next.run(request).await;
    }

    let audit_service = request.extensions().get::<Arc<AuditLogService>>().cloned();
    let actor = request.extensions().get::<User>().cloned();
    let headers = request.headers().clone();
    let request_id = extract_request_id(&headers);

    let response = next.run(request).await;

    if config.audit_log_retention_days == 0 {
        return response;
    }

    let Some(descriptor) = descriptor else {
        return response;
    };
    let Some(audit_service) = audit_service else {
        return response;
    };

    let status = response.status();
    let result = if status.is_client_error() || status.is_server_error() {
        "failure"
    } else {
        "success"
    };
    let error_code = if result == "failure" {
        Some(format!("http_{}", status.as_u16()))
    } else {
        None
    };

    let entry = AuditLogEntry {
        occurred_at: Utc::now(),
        actor_id: actor.as_ref().map(|user| user.id.clone()),
        actor_type: actor
            .as_ref()
            .map(|_| "user".to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        event_type: descriptor.event_type.to_string(),
        target_type: descriptor.target_type.map(|value| value.to_string()),
        target_id: descriptor.target_id,
        result: result.to_string(),
        error_code,
        metadata: None,
        ip: extract_ip(&headers),
        user_agent: extract_user_agent(&headers),
        request_id: Some(request_id),
    };

    if let Err(err) = audit_service.record_event(entry).await {
        tracing::warn!(error = ?err, method = %method, path = %path, "Failed to record audit log");
    }

    response
}

fn classify_event(method: &Method, path: &str) -> Option<AuditEventDescriptor> {
    let normalized = path.trim_end_matches('/');
    if !normalized.starts_with("/api/") {
        return None;
    }
    if is_excluded(method, normalized) {
        return None;
    }

    let segments: Vec<&str> = normalized.trim_start_matches('/').split('/').collect();

    match (method, segments.as_slice()) {
        (&Method::POST, ["api", "auth", "login"]) => Some(event("auth_login", "user", None)),
        (&Method::POST, ["api", "auth", "refresh"]) => Some(event("auth_refresh", "user", None)),
        (&Method::POST, ["api", "auth", "logout"]) => Some(event("auth_logout", "user", None)),
        (&Method::POST, ["api", "auth", "mfa", "register"]) => {
            Some(event("mfa_register", "user", None))
        }
        (&Method::POST, ["api", "auth", "mfa", "setup"]) => Some(event("mfa_setup", "user", None)),
        (&Method::POST, ["api", "auth", "mfa", "activate"]) => {
            Some(event("mfa_activate", "user", None))
        }
        (&Method::DELETE, ["api", "auth", "mfa"]) => Some(event("mfa_disable", "user", None)),
        (&Method::PUT, ["api", "auth", "change-password"]) => {
            Some(event("password_change", "user", None))
        }
        (&Method::POST, ["api", "attendance", "clock-in"]) => {
            Some(event("attendance_clock_in", "attendance", None))
        }
        (&Method::POST, ["api", "attendance", "clock-out"]) => {
            Some(event("attendance_clock_out", "attendance", None))
        }
        (&Method::POST, ["api", "attendance", "break-start"]) => {
            Some(event("attendance_break_start", "break_record", None))
        }
        (&Method::POST, ["api", "attendance", "break-end"]) => {
            Some(event("attendance_break_end", "break_record", None))
        }
        (&Method::GET, ["api", "attendance", "export"]) => {
            Some(event("attendance_export", "export", None))
        }
        (&Method::POST, ["api", "requests", "leave"]) => {
            Some(event("request_leave_create", "request", None))
        }
        (&Method::POST, ["api", "requests", "overtime"]) => {
            Some(event("request_overtime_create", "request", None))
        }
        (&Method::PUT, ["api", "requests", request_id]) => Some(event(
            "request_update",
            "request",
            Some((*request_id).to_string()),
        )),
        (&Method::DELETE, ["api", "requests", request_id]) => Some(event(
            "request_cancel",
            "request",
            Some((*request_id).to_string()),
        )),
        (&Method::GET, ["api", "admin", "requests"]) => {
            Some(event("admin_request_list", "system", None))
        }
        (&Method::GET, ["api", "admin", "requests", request_id]) => Some(event(
            "admin_request_detail",
            "request",
            Some((*request_id).to_string()),
        )),
        (&Method::PUT, ["api", "admin", "requests", request_id, "approve"]) => Some(event(
            "admin_request_approve",
            "request",
            Some((*request_id).to_string()),
        )),
        (&Method::PUT, ["api", "admin", "requests", request_id, "reject"]) => Some(event(
            "admin_request_reject",
            "request",
            Some((*request_id).to_string()),
        )),
        (&Method::GET, ["api", "admin", "holidays"]) => {
            Some(event("admin_holiday_list", "system", None))
        }
        (&Method::POST, ["api", "admin", "holidays"]) => {
            Some(event("admin_holiday_create", "holiday", None))
        }
        (&Method::DELETE, ["api", "admin", "holidays", holiday_id]) => Some(event(
            "admin_holiday_delete",
            "holiday",
            Some((*holiday_id).to_string()),
        )),
        (&Method::GET, ["api", "admin", "holidays", "weekly"]) => {
            Some(event("admin_weekly_holiday_list", "system", None))
        }
        (&Method::POST, ["api", "admin", "holidays", "weekly"]) => {
            Some(event("admin_weekly_holiday_create", "weekly_holiday", None))
        }
        (&Method::GET, ["api", "admin", "holidays", "google"]) => {
            Some(event("admin_holiday_google_fetch", "system", None))
        }
        (&Method::GET, ["api", "admin", "users", user_id, "holiday-exceptions"]) => Some(event(
            "admin_holiday_exception_list",
            "user",
            Some((*user_id).to_string()),
        )),
        (&Method::POST, ["api", "admin", "users", _, "holiday-exceptions"]) => Some(event(
            "admin_holiday_exception_create",
            "holiday_exception",
            None,
        )),
        (&Method::DELETE, ["api", "admin", "users", _, "holiday-exceptions", exception_id]) => {
            Some(event(
                "admin_holiday_exception_delete",
                "holiday_exception",
                Some((*exception_id).to_string()),
            ))
        }
        (&Method::GET, ["api", "admin", "export"]) => Some(event("admin_export", "export", None)),
        (&Method::GET, ["api", "admin", "users"]) => Some(event("admin_user_list", "system", None)),
        (&Method::POST, ["api", "admin", "users"]) => {
            Some(event("admin_user_create", "user", None))
        }
        (&Method::GET, ["api", "admin", "attendance"]) => {
            Some(event("admin_attendance_list", "system", None))
        }
        (&Method::PUT, ["api", "admin", "attendance"]) => {
            Some(event("admin_attendance_upsert", "attendance", None))
        }
        (&Method::PUT, ["api", "admin", "breaks", break_id, "force-end"]) => Some(event(
            "admin_break_force_end",
            "break_record",
            Some((*break_id).to_string()),
        )),
        (&Method::POST, ["api", "admin", "mfa", "reset"]) => {
            Some(event("admin_mfa_reset", "user", None))
        }
        _ => None,
    }
}

fn event(
    event_type: &'static str,
    target_type: &'static str,
    target_id: Option<String>,
) -> AuditEventDescriptor {
    AuditEventDescriptor {
        event_type,
        target_type: Some(target_type),
        target_id,
    }
}

fn is_excluded(method: &Method, path: &str) -> bool {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match (method, segments.as_slice()) {
        (&Method::GET, ["api", "config", "timezone"]) => true,
        (&Method::GET, ["api", "auth", "me"]) => true,
        (&Method::GET, ["api", "auth", "mfa"]) => true,
        (&Method::GET, ["api", "holidays"]) => true,
        (&Method::GET, ["api", "holidays", "check"]) => true,
        (&Method::GET, ["api", "holidays", "month"]) => true,
        (&Method::GET, ["api", "attendance", "status"]) => true,
        (&Method::GET, ["api", "attendance", "me"]) => true,
        (&Method::GET, ["api", "attendance", "me", "summary"]) => true,
        (&Method::GET, ["api", "attendance", _, "breaks"]) => true,
        (&Method::GET, ["api", "requests", "me"]) => true,
        _ => path.starts_with("/api/docs") || path.starts_with("/api-doc/"),
    }
}

fn extract_request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .or_else(|| headers.get("x-correlation-id"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn classify_event_matches_dynamic_paths() {
        let event = classify_event(&Method::PUT, "/api/admin/requests/req-123/approve")
            .expect("event should map");
        assert_eq!(event.event_type, "admin_request_approve");
        assert_eq!(event.target_type, Some("request"));
        assert_eq!(event.target_id.as_deref(), Some("req-123"));
    }

    #[test]
    fn classify_event_skips_excluded_paths() {
        let event = classify_event(&Method::GET, "/api/attendance/status");
        assert!(event.is_none());
    }

    #[test]
    fn classify_event_returns_none_for_unknown_paths() {
        let event = classify_event(&Method::GET, "/api/unknown");
        assert!(event.is_none());
    }

    #[test]
    fn extract_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1, 10.0.0.1".parse().unwrap());
        headers.insert("x-real-ip", "203.0.113.2".parse().unwrap());
        assert_eq!(extract_ip(&headers).as_deref(), Some("203.0.113.1"));
    }

    #[test]
    fn extract_request_id_generates_when_missing() {
        let headers = HeaderMap::new();
        let value = extract_request_id(&headers);
        assert!(!value.is_empty());
    }

    #[test]
    fn extract_request_id_uses_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "req-001".parse().unwrap());
        assert_eq!(extract_request_id(&headers), "req-001");
    }

    #[test]
    fn extract_user_agent_reads_header() {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "test-agent".parse().unwrap());
        assert_eq!(extract_user_agent(&headers).as_deref(), Some("test-agent"));
    }

    #[test]
    fn result_error_code_uses_http_status() {
        let status = StatusCode::BAD_REQUEST;
        let result = if status.is_client_error() || status.is_server_error() {
            "failure"
        } else {
            "success"
        };
        let error_code = if result == "failure" {
            Some(format!("http_{}", status.as_u16()))
        } else {
            None
        };
        assert_eq!(result, "failure");
        assert_eq!(error_code.as_deref(), Some("http_400"));
    }
}
