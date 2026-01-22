use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{header::USER_AGENT, HeaderMap, Method},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use http_body::{Body as HttpBody, Frame};
use http_body_util::BodyExt;
use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use uuid::Uuid;

use crate::{
    middleware::request_id::RequestId,
    models::user::User,
    services::audit_log::{AuditLogEntry, AuditLogServiceTrait},
    state::AppState,
};

const DEFAULT_CLOCK_SOURCE: &str = "api";
const MAX_BUFFERED_BODY_BYTES: usize = 64 * 1024;
const REQUEST_TYPE_UNKNOWN: &str = "unknown";

struct AuditEventDescriptor {
    event_type: &'static str,
    target_type: Option<&'static str>,
    target_id: Option<String>,
}

pub async fn audit_log(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let descriptor = classify_event(&method, &path);

    if descriptor.is_none() {
        return next.run(request).await;
    }

    let headers = request.headers().clone();
    let body_bytes = if let Some(descriptor_ref) = descriptor.as_ref() {
        if needs_body_for_metadata(descriptor_ref.event_type) {
            let (buffered_request, bytes) = buffer_request_body(request).await;
            request = buffered_request;
            bytes
        } else {
            None
        }
    } else {
        None
    };

    let audit_service = request
        .extensions()
        .get::<Arc<dyn AuditLogServiceTrait>>()
        .cloned();
    let actor_before = request.extensions().get::<User>().cloned();
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|id| id.0.clone())
        .unwrap_or_else(|| extract_request_id(&headers));

    let response = next.run(request).await;

    if !state
        .config
        .audit_log_retention_policy()
        .is_recording_enabled()
    {
        return response;
    }

    let Some(descriptor) = descriptor else {
        return response;
    };
    let Some(audit_service) = audit_service else {
        return response;
    };

    let status = response.status();
    let actor = response
        .extensions()
        .get::<User>()
        .cloned()
        .or_else(|| actor_before.clone());
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
        actor_id: actor.as_ref().map(|user| user.id),
        actor_type: actor
            .as_ref()
            .map(|_| "user".to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        event_type: descriptor.event_type.to_string(),
        target_type: descriptor.target_type.map(|value| value.to_string()),
        target_id: descriptor.target_id,
        result: result.to_string(),
        error_code,
        metadata: build_metadata(
            descriptor.event_type,
            &headers,
            &state,
            actor.as_ref(),
            body_bytes.as_ref(),
        ),
        ip: extract_ip(&headers),
        user_agent: extract_user_agent(&headers),
        request_id: Some(request_id),
    };

    let method = method.to_string();
    tokio::spawn(async move {
        if let Err(err) = audit_service.record_event(entry).await {
            tracing::warn!(
                error = ?err,
                method = %method,
                path = %path,
                "Failed to record audit log"
            );
        }
    });

    response
}

struct BufferedBody {
    buffered: VecDeque<Frame<Bytes>>,
    inner: Body,
    pending_error: Option<axum::Error>,
}

impl BufferedBody {
    fn new(
        buffered: VecDeque<Frame<Bytes>>,
        inner: Body,
        pending_error: Option<axum::Error>,
    ) -> Self {
        Self {
            buffered,
            inner,
            pending_error,
        }
    }

    fn buffered_len(&self) -> u64 {
        self.buffered
            .iter()
            .filter_map(|frame| frame.data_ref().map(|data| data.len() as u64))
            .sum()
    }
}

impl HttpBody for BufferedBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if let Some(frame) = this.buffered.pop_front() {
            return Poll::Ready(Some(Ok(frame)));
        }
        if let Some(err) = this.pending_error.take() {
            this.inner = Body::empty();
            return Poll::Ready(Some(Err(err)));
        }
        Pin::new(&mut this.inner).poll_frame(cx)
    }

    fn size_hint(&self) -> http_body::SizeHint {
        let buffered_len = self.buffered_len();
        let mut hint = self.inner.size_hint();
        hint.set_lower(hint.lower().saturating_add(buffered_len));
        if let Some(upper) = hint.upper() {
            hint.set_upper(upper.saturating_add(buffered_len));
        }
        hint
    }

    fn is_end_stream(&self) -> bool {
        if !self.buffered.is_empty() || self.pending_error.is_some() {
            return false;
        }
        self.inner.is_end_stream()
    }
}

async fn buffer_request_body(request: Request) -> (Request, Option<Bytes>) {
    let (parts, mut body) = request.into_parts();
    let mut buffered_frames = VecDeque::new();
    let mut buffered_bytes = Vec::new();
    let mut overflowed = false;
    let mut pending_error = None;

    while let Some(frame_result) = body.frame().await {
        match frame_result {
            Ok(frame) => {
                if let Some(data) = frame.data_ref() {
                    if !overflowed {
                        let new_len = buffered_bytes.len() + data.len();
                        if new_len > MAX_BUFFERED_BODY_BYTES {
                            overflowed = true;
                        } else {
                            buffered_bytes.extend_from_slice(data);
                        }
                    }
                }
                buffered_frames.push_back(frame);
                if overflowed {
                    break;
                }
            }
            Err(err) => {
                pending_error = Some(err);
                break;
            }
        }
    }

    let body_bytes = if overflowed || pending_error.is_some() {
        None
    } else {
        Some(Bytes::from(buffered_bytes))
    };
    let replay_body = BufferedBody::new(buffered_frames, body, pending_error);
    let request = Request::from_parts(parts, Body::new(replay_body));
    (request, body_bytes)
}

fn needs_body_for_metadata(event_type: &str) -> bool {
    matches!(
        event_type,
        "request_leave_create"
            | "request_overtime_create"
            | "request_update"
            | "request_cancel"
            | "consent_record"
            | "subject_request_create"
            | "admin_subject_request_approve"
            | "admin_subject_request_reject"
    )
}

fn build_metadata(
    event_type: &str,
    headers: &HeaderMap,
    state: &AppState,
    actor: Option<&User>,
    body_bytes: Option<&Bytes>,
) -> Option<Value> {
    match event_type {
        "attendance_clock_in"
        | "attendance_clock_out"
        | "attendance_break_start"
        | "attendance_break_end" => Some(build_attendance_metadata(event_type, headers, state)),
        "request_leave_create"
        | "request_overtime_create"
        | "request_update"
        | "request_cancel" => {
            let payload = parse_json_body(body_bytes);
            Some(build_request_metadata(event_type, payload.as_ref()))
        }
        "consent_record" => {
            let payload = parse_json_body(body_bytes);
            Some(build_consent_metadata(payload.as_ref()))
        }
        "subject_request_create"
        | "subject_request_cancel"
        | "admin_subject_request_approve"
        | "admin_subject_request_reject" => {
            let payload = parse_json_body(body_bytes);
            Some(build_subject_request_metadata(event_type, payload.as_ref()))
        }
        "admin_request_approve" | "admin_request_reject" => {
            Some(build_approval_metadata(event_type))
        }
        "password_change" => Some(build_password_change_metadata(actor)),
        _ => None,
    }
}

fn build_attendance_metadata(event_type: &str, headers: &HeaderMap, state: &AppState) -> Value {
    let clock_type = match event_type {
        "attendance_clock_in" => "clock_in",
        "attendance_clock_out" => "clock_out",
        "attendance_break_start" => "break_start",
        "attendance_break_end" => "break_end",
        _ => "unknown",
    };
    let source = extract_source(headers);
    json!({
        "clock_type": clock_type,
        "timezone": state.config.time_zone.to_string(),
        "source": source,
    })
}

fn build_request_metadata(event_type: &str, payload: Option<&Value>) -> Value {
    let request_type = match event_type {
        "request_leave_create" => "leave",
        "request_overtime_create" => "overtime",
        _ => infer_request_type(payload).unwrap_or(REQUEST_TYPE_UNKNOWN),
    };
    let payload_summary = build_request_payload_summary(request_type, payload);
    json!({
        "request_type": request_type,
        "payload_summary": payload_summary,
    })
}

fn build_request_payload_summary(request_type: &str, payload: Option<&Value>) -> Value {
    let mut summary = Map::new();
    if let Some(payload) = payload {
        match request_type {
            "leave" => {
                insert_string_if_present(&mut summary, payload, "leave_type");
                insert_string_if_present(&mut summary, payload, "start_date");
                insert_string_if_present(&mut summary, payload, "end_date");
            }
            "overtime" => {
                insert_string_if_present(&mut summary, payload, "date");
                insert_number_if_present(&mut summary, payload, "planned_hours");
            }
            _ => {
                insert_string_if_present(&mut summary, payload, "leave_type");
                insert_string_if_present(&mut summary, payload, "start_date");
                insert_string_if_present(&mut summary, payload, "end_date");
                insert_string_if_present(&mut summary, payload, "date");
                insert_number_if_present(&mut summary, payload, "planned_hours");
            }
        }
    }
    Value::Object(summary)
}

fn build_consent_metadata(payload: Option<&Value>) -> Value {
    let mut summary = Map::new();
    if let Some(payload) = payload {
        insert_string_if_present(&mut summary, payload, "purpose");
        insert_string_if_present(&mut summary, payload, "policy_version");
    }
    Value::Object(summary)
}

fn build_subject_request_metadata(event_type: &str, payload: Option<&Value>) -> Value {
    let mut summary = Map::new();
    match event_type {
        "subject_request_create" => {
            let request_type = payload
                .and_then(|value| value.get("request_type"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            summary.insert(
                "request_type".to_string(),
                Value::String(request_type.to_string()),
            );
            let details_len = payload
                .and_then(|value| value.get("details"))
                .and_then(Value::as_str)
                .map(|value| value.chars().count() as u64);
            summary.insert(
                "details_present".to_string(),
                Value::Bool(details_len.unwrap_or(0) > 0),
            );
            if let Some(len) = details_len {
                summary.insert(
                    "details_length".to_string(),
                    Value::Number(serde_json::Number::from(len)),
                );
            }
        }
        "admin_subject_request_approve" | "admin_subject_request_reject" => {
            let decision = if event_type == "admin_subject_request_approve" {
                "approve"
            } else {
                "reject"
            };
            summary.insert("decision".to_string(), Value::String(decision.to_string()));
            let comment_len = payload
                .and_then(|value| value.get("comment"))
                .and_then(Value::as_str)
                .map(|value| value.chars().count() as u64);
            summary.insert(
                "comment_present".to_string(),
                Value::Bool(comment_len.unwrap_or(0) > 0),
            );
            if let Some(len) = comment_len {
                summary.insert(
                    "comment_length".to_string(),
                    Value::Number(serde_json::Number::from(len)),
                );
            }
        }
        "subject_request_cancel" => {
            summary.insert("status".to_string(), Value::String("cancelled".to_string()));
        }
        _ => {}
    }
    Value::Object(summary)
}

fn insert_string_if_present(summary: &mut Map<String, Value>, payload: &Value, key: &str) {
    if let Some(value) = payload.get(key).and_then(Value::as_str) {
        summary.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn insert_number_if_present(summary: &mut Map<String, Value>, payload: &Value, key: &str) {
    if let Some(value) = payload.get(key).and_then(Value::as_f64) {
        summary.insert(key.to_string(), json!(value));
    }
}

fn infer_request_type(payload: Option<&Value>) -> Option<&'static str> {
    let payload = payload?;
    if payload.get("leave_type").is_some()
        || payload.get("start_date").is_some()
        || payload.get("end_date").is_some()
    {
        return Some("leave");
    }
    if payload.get("planned_hours").is_some() || payload.get("date").is_some() {
        return Some("overtime");
    }
    None
}

fn build_approval_metadata(event_type: &str) -> Value {
    let decision = if event_type == "admin_request_approve" {
        "approve"
    } else {
        "reject"
    };
    json!({
        "approval_step": "single",
        "decision": decision,
    })
}

fn build_password_change_metadata(actor: Option<&User>) -> Value {
    let mfa_value = actor
        .map(|user| Value::Bool(user.is_mfa_enabled()))
        .unwrap_or(Value::Null);
    json!({
        "method": "password",
        "mfa_enabled": mfa_value,
    })
}

fn parse_json_body(body_bytes: Option<&Bytes>) -> Option<Value> {
    let bytes = body_bytes?;
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(bytes).ok()
}

fn extract_source(headers: &HeaderMap) -> String {
    headers
        .get("x-client-source")
        .or_else(|| headers.get("x-source"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_CLOCK_SOURCE.to_string())
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
        (&Method::POST, ["api", "consents"]) => Some(event("consent_record", "consent_log", None)),
        (&Method::POST, ["api", "subject-requests"]) => {
            Some(event("subject_request_create", "subject_request", None))
        }
        (&Method::DELETE, ["api", "subject-requests", request_id]) => Some(event(
            "subject_request_cancel",
            "subject_request",
            Some((*request_id).to_string()),
        )),
        (&Method::GET, ["api", "admin", "subject-requests"]) => {
            Some(event("admin_subject_request_list", "subject_request", None))
        }
        (&Method::PUT, ["api", "admin", "subject-requests", request_id, "approve"]) => Some(event(
            "admin_subject_request_approve",
            "subject_request",
            Some((*request_id).to_string()),
        )),
        (&Method::PUT, ["api", "admin", "subject-requests", request_id, "reject"]) => Some(event(
            "admin_subject_request_reject",
            "subject_request",
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
        (&Method::GET, ["api", "admin", "audit-logs"]) => {
            Some(event("admin_audit_log_list", "audit_log", None))
        }
        (&Method::GET, ["api", "admin", "audit-logs", "export"]) => {
            Some(event("admin_audit_log_export", "audit_log", None))
        }
        (&Method::GET, ["api", "admin", "audit-logs", audit_log_id]) => Some(event(
            "admin_audit_log_detail",
            "audit_log",
            Some((*audit_log_id).to_string()),
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
    use axum::body::to_bytes;
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
    fn classify_event_matches_subject_request_paths() {
        let create_event =
            classify_event(&Method::POST, "/api/subject-requests").expect("create maps");
        assert_eq!(create_event.event_type, "subject_request_create");
        assert_eq!(create_event.target_type, Some("subject_request"));
        assert!(create_event.target_id.is_none());

        let cancel_event =
            classify_event(&Method::DELETE, "/api/subject-requests/req-1").expect("cancel maps");
        assert_eq!(cancel_event.event_type, "subject_request_cancel");
        assert_eq!(cancel_event.target_type, Some("subject_request"));
        assert_eq!(cancel_event.target_id.as_deref(), Some("req-1"));

        let admin_list =
            classify_event(&Method::GET, "/api/admin/subject-requests").expect("admin list maps");
        assert_eq!(admin_list.event_type, "admin_subject_request_list");
        assert_eq!(admin_list.target_type, Some("subject_request"));
        assert!(admin_list.target_id.is_none());

        let admin_approve =
            classify_event(&Method::PUT, "/api/admin/subject-requests/req-2/approve")
                .expect("admin approve maps");
        assert_eq!(admin_approve.event_type, "admin_subject_request_approve");
        assert_eq!(admin_approve.target_type, Some("subject_request"));
        assert_eq!(admin_approve.target_id.as_deref(), Some("req-2"));
    }

    #[test]
    fn classify_event_matches_audit_log_paths() {
        let list_event = classify_event(&Method::GET, "/api/admin/audit-logs")
            .expect("audit log list should map");
        assert_eq!(list_event.event_type, "admin_audit_log_list");
        assert_eq!(list_event.target_type, Some("audit_log"));
        assert!(list_event.target_id.is_none());

        let detail_event = classify_event(&Method::GET, "/api/admin/audit-logs/log-123")
            .expect("audit log detail should map");
        assert_eq!(detail_event.event_type, "admin_audit_log_detail");
        assert_eq!(detail_event.target_type, Some("audit_log"));
        assert_eq!(detail_event.target_id.as_deref(), Some("log-123"));

        let export_event = classify_event(&Method::GET, "/api/admin/audit-logs/export")
            .expect("audit log export should map");
        assert_eq!(export_event.event_type, "admin_audit_log_export");
        assert_eq!(export_event.target_type, Some("audit_log"));
        assert!(export_event.target_id.is_none());
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

    #[tokio::test]
    async fn buffer_request_body_preserves_large_body() {
        let body = vec![b'a'; MAX_BUFFERED_BODY_BYTES + 1];
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/requests/leave")
            .body(Body::from(body.clone()))
            .unwrap();

        let (buffered_request, body_bytes) = buffer_request_body(request).await;
        let bytes = to_bytes(buffered_request.into_body(), body.len() + 1)
            .await
            .expect("body should remain readable");

        assert_eq!(bytes.as_ref(), body.as_slice());
        assert!(body_bytes.is_none());
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
