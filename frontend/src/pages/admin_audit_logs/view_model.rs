use crate::api::{ApiClient, ApiError, AuditLogListResponse, AuditLogQuery};
use leptos::*;
use wasm_bindgen::JsCast;

pub const DEFAULT_AUDIT_LOGS_PER_PAGE: i64 = 20;

pub const AUDIT_EVENT_TYPES: &[(&str, &str)] = &[
    ("auth_login", "pages.admin_audit_logs.events.auth_login"),
    ("auth_logout", "pages.admin_audit_logs.events.auth_logout"),
    ("auth_refresh", "pages.admin_audit_logs.events.auth_refresh"),
    ("mfa_activate", "pages.admin_audit_logs.events.mfa_activate"),
    ("mfa_disable", "pages.admin_audit_logs.events.mfa_disable"),
    ("mfa_register", "pages.admin_audit_logs.events.mfa_register"),
    ("mfa_setup", "pages.admin_audit_logs.events.mfa_setup"),
    (
        "password_change",
        "pages.admin_audit_logs.events.password_change",
    ),
    (
        "attendance_break_end",
        "pages.admin_audit_logs.events.attendance_break_end",
    ),
    (
        "attendance_break_start",
        "pages.admin_audit_logs.events.attendance_break_start",
    ),
    (
        "attendance_clock_in",
        "pages.admin_audit_logs.events.attendance_clock_in",
    ),
    (
        "attendance_clock_out",
        "pages.admin_audit_logs.events.attendance_clock_out",
    ),
    (
        "attendance_export",
        "pages.admin_audit_logs.events.attendance_export",
    ),
    (
        "request_cancel",
        "pages.admin_audit_logs.events.request_cancel",
    ),
    (
        "request_leave_create",
        "pages.admin_audit_logs.events.request_leave_create",
    ),
    (
        "request_overtime_create",
        "pages.admin_audit_logs.events.request_overtime_create",
    ),
    (
        "request_update",
        "pages.admin_audit_logs.events.request_update",
    ),
    (
        "subject_request_cancel",
        "pages.admin_audit_logs.events.subject_request_cancel",
    ),
    (
        "subject_request_create",
        "pages.admin_audit_logs.events.subject_request_create",
    ),
    (
        "consent_record",
        "pages.admin_audit_logs.events.consent_record",
    ),
    (
        "admin_attendance_list",
        "pages.admin_audit_logs.events.admin_attendance_list",
    ),
    (
        "admin_attendance_upsert",
        "pages.admin_audit_logs.events.admin_attendance_upsert",
    ),
    (
        "admin_audit_log_detail",
        "pages.admin_audit_logs.events.admin_audit_log_detail",
    ),
    (
        "admin_audit_log_export",
        "pages.admin_audit_logs.events.admin_audit_log_export",
    ),
    (
        "admin_audit_log_list",
        "pages.admin_audit_logs.events.admin_audit_log_list",
    ),
    (
        "admin_break_force_end",
        "pages.admin_audit_logs.events.admin_break_force_end",
    ),
    ("admin_export", "pages.admin_audit_logs.events.admin_export"),
    (
        "admin_holiday_create",
        "pages.admin_audit_logs.events.admin_holiday_create",
    ),
    (
        "admin_holiday_delete",
        "pages.admin_audit_logs.events.admin_holiday_delete",
    ),
    (
        "admin_holiday_google_fetch",
        "pages.admin_audit_logs.events.admin_holiday_google_fetch",
    ),
    (
        "admin_holiday_list",
        "pages.admin_audit_logs.events.admin_holiday_list",
    ),
    (
        "admin_mfa_reset",
        "pages.admin_audit_logs.events.admin_mfa_reset",
    ),
    (
        "admin_request_approve",
        "pages.admin_audit_logs.events.admin_request_approve",
    ),
    (
        "admin_request_detail",
        "pages.admin_audit_logs.events.admin_request_detail",
    ),
    (
        "admin_request_list",
        "pages.admin_audit_logs.events.admin_request_list",
    ),
    (
        "admin_request_reject",
        "pages.admin_audit_logs.events.admin_request_reject",
    ),
    (
        "admin_subject_request_approve",
        "pages.admin_audit_logs.events.admin_subject_request_approve",
    ),
    (
        "admin_subject_request_list",
        "pages.admin_audit_logs.events.admin_subject_request_list",
    ),
    (
        "admin_subject_request_reject",
        "pages.admin_audit_logs.events.admin_subject_request_reject",
    ),
    (
        "admin_user_create",
        "pages.admin_audit_logs.events.admin_user_create",
    ),
    (
        "admin_user_list",
        "pages.admin_audit_logs.events.admin_user_list",
    ),
    (
        "admin_weekly_holiday_create",
        "pages.admin_audit_logs.events.admin_weekly_holiday_create",
    ),
    (
        "admin_weekly_holiday_list",
        "pages.admin_audit_logs.events.admin_weekly_holiday_list",
    ),
];

pub fn audit_event_label(event_type: &str) -> String {
    AUDIT_EVENT_TYPES
        .iter()
        .find(|(code, _)| *code == event_type)
        .map(|(_, key)| rust_i18n::t!(*key).into_owned())
        .unwrap_or_else(|| event_type.to_string())
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogFilters {
    pub from: String,
    pub to: String,
    pub actor_id: String,
    pub event_type: String,
    pub result: String,
}

type AuditLogQueryParams = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl Default for AuditLogFilters {
    fn default() -> Self {
        Self {
            from: "".to_string(),
            to: "".to_string(),
            actor_id: "".to_string(),
            event_type: "".to_string(),
            result: "".to_string(),
        }
    }
}

impl AuditLogFilters {
    fn into_query_params(self) -> AuditLogQueryParams {
        (
            optional_param(self.from),
            optional_param(self.to),
            optional_param(self.actor_id),
            optional_param(self.event_type),
            optional_param(self.result),
        )
    }
}

#[derive(Clone)]
pub struct AuditLogViewModel {
    pub logs_resource:
        Resource<(i64, i64, AuditLogFilters), Result<AuditLogListResponse, ApiError>>,
    pub page: RwSignal<i64>,
    pub per_page: RwSignal<i64>,
    pub filters: RwSignal<AuditLogFilters>,
    pub pii_masked: RwSignal<bool>,
    pub export_action: Action<(), Result<(), ApiError>>,
}

pub fn use_audit_log_view_model() -> AuditLogViewModel {
    let api_client = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let page = create_rw_signal(1);
    let per_page = create_rw_signal(DEFAULT_AUDIT_LOGS_PER_PAGE);
    let filters = create_rw_signal(AuditLogFilters::default());
    let pii_masked = create_rw_signal(false);

    let api = api_client.clone();
    let logs_resource = create_resource(
        move || (page.get(), per_page.get(), filters.get()),
        move |(p, per_page, f)| {
            let api = api.clone();
            let pii_masked = pii_masked;
            async move {
                let (from, to, actor_id, event_type, result) = f.into_query_params();
                let response = api
                    .list_audit_logs_with_policy(AuditLogQuery {
                        page: Some(p),
                        per_page: Some(per_page),
                        from,
                        to,
                        actor_id,
                        event_type,
                        result,
                    })
                    .await?;
                pii_masked.set(response.pii_masked);
                Ok(response.data)
            }
        },
    );

    let api_export = api_client.clone();
    let export_action = create_action(move |_| {
        let api = api_export.clone();
        let f = filters.get_untracked();
        async move {
            let (from, to, actor_id, event_type, result) = f.into_query_params();
            let result_data = api
                .export_audit_logs_with_policy(AuditLogQuery {
                    page: None,
                    per_page: None,
                    from,
                    to,
                    actor_id,
                    event_type,
                    result,
                })
                .await;

            match result_data {
                Ok(response) => {
                    pii_masked.set(response.pii_masked);
                    let data = response.data;
                    // In Rust WASM, to trigger download:
                    // 1. Serialize data to JSON string
                    // 2. Create Blob
                    // 3. Create object URL
                    // 4. Create anchor, click, revoke
                    // We will use a helper or do it here.
                    // Since we cannot access web_sys easily without boilerplate, and result is Vec<AuditLog>.
                    // We will convert to JsValue and use web primitives if possible, or just print success?
                    // The requirement is "download".
                    // We'll wrap this logic in a function `trigger_download` if possible.
                    // For now, let's just return Ok and handle UI side?
                    // No, create_action should probably do the side effect if possible or we return data.

                    // Helper using web-sys
                    if let Ok(json_str) = serde_json::to_string(&data) {
                        trigger_download("audit_logs.json", &json_str);
                        Ok(())
                    } else {
                        Err(ApiError::unknown("Failed to serialize export data"))
                    }
                }
                Err(e) => Err(e),
            }
        }
    });

    AuditLogViewModel {
        logs_resource,
        page,
        per_page,
        filters,
        pii_masked,
        export_action,
    }
}

fn optional_param(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn trigger_download(filename: &str, content: &str) {
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let props = BlobPropertyBag::new();
            props.set_type("application/json");

            let blob_parts = js_sys::Array::of1(&content.into());
            if let Ok(blob) = Blob::new_with_str_sequence_and_options(&blob_parts, &props) {
                if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                    if let Ok(element) = document.create_element("a") {
                        if let Ok(anchor) = element.dyn_into::<HtmlAnchorElement>() {
                            anchor.set_href(&url);
                            anchor.set_download(filename);
                            anchor.click();
                            let _ = Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_default_is_empty() {
        let filters = AuditLogFilters::default();
        assert_eq!(filters.from, "");
        assert_eq!(filters.to, "");
        assert_eq!(filters.actor_id, "");
        assert_eq!(filters.event_type, "");
        assert_eq!(filters.result, "");
    }

    #[test]
    fn filters_convert_empty_values_to_none() {
        let params = AuditLogFilters::default().into_query_params();
        assert_eq!(params, (None, None, None, None, None));
    }

    #[test]
    fn filters_convert_non_empty_values_to_some() {
        let params = AuditLogFilters {
            from: "2026-01-01".to_string(),
            to: "2026-01-31".to_string(),
            actor_id: "12".to_string(),
            event_type: "admin_user_create".to_string(),
            result: "success".to_string(),
        }
        .into_query_params();

        assert_eq!(
            params,
            (
                Some("2026-01-01".to_string()),
                Some("2026-01-31".to_string()),
                Some("12".to_string()),
                Some("admin_user_create".to_string()),
                Some("success".to_string()),
            )
        );
    }

    #[test]
    fn optional_param_keeps_whitespace_input_as_some() {
        assert_eq!(optional_param(" ".to_string()), Some(" ".to_string()));
    }

    #[test]
    fn default_per_page_matches_current_ui_default() {
        assert_eq!(DEFAULT_AUDIT_LOGS_PER_PAGE, 20);
    }

    #[test]
    fn audit_event_types_keys_are_unique_and_labels_not_empty() {
        use std::collections::HashSet;

        let mut keys = HashSet::new();
        for (key, label) in AUDIT_EVENT_TYPES {
            assert!(
                keys.insert(*key),
                "duplicate audit event type key found: {key}"
            );
            assert!(
                !label.trim().is_empty(),
                "audit event type label must not be empty for key: {key}"
            );
        }
    }

    #[test]
    fn audit_event_types_include_core_admin_and_auth_events() {
        let keys: Vec<&str> = AUDIT_EVENT_TYPES.iter().map(|(key, _)| *key).collect();
        assert!(keys.contains(&"auth_login"));
        assert!(keys.contains(&"auth_logout"));
        assert!(keys.contains(&"admin_audit_log_list"));
        assert!(keys.contains(&"admin_request_approve"));
    }
}
