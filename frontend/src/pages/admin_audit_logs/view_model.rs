use crate::api::{ApiClient, ApiError, AuditLogListResponse};
use leptos::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogFilters {
    pub from: String,
    pub to: String,
    pub actor_id: String,
    pub event_type: String,
    pub result: String,
}

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

#[derive(Clone)]
pub struct AuditLogViewModel {
    pub logs_resource: Resource<(i64, AuditLogFilters), Result<AuditLogListResponse, ApiError>>,
    pub page: RwSignal<i64>,
    pub filters: RwSignal<AuditLogFilters>,
    #[allow(dead_code)]
    pub api_client: ApiClient,
    pub export_action: Action<(), Result<(), ApiError>>,
}

pub fn use_audit_log_view_model() -> AuditLogViewModel {
    let api_client = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let page = create_rw_signal(1);
    let filters = create_rw_signal(AuditLogFilters::default());

    let api = api_client.clone();
    let logs_resource = create_resource(
        move || (page.get(), filters.get()),
        move |(p, f)| {
            let api = api.clone();
            async move {
                api.list_audit_logs(
                    p,
                    20,
                    if f.from.is_empty() {
                        None
                    } else {
                        Some(f.from)
                    },
                    if f.to.is_empty() { None } else { Some(f.to) },
                    if f.actor_id.is_empty() {
                        None
                    } else {
                        Some(f.actor_id)
                    },
                    if f.event_type.is_empty() {
                        None
                    } else {
                        Some(f.event_type)
                    },
                    if f.result.is_empty() {
                        None
                    } else {
                        Some(f.result)
                    },
                )
                .await
            }
        },
    );

    let api_export = api_client.clone();
    let export_action = create_action(move |_| {
        let api = api_export.clone();
        let f = filters.get_untracked();
        async move {
            let result_data = api
                .export_audit_logs(
                    if f.from.is_empty() {
                        None
                    } else {
                        Some(f.from)
                    },
                    if f.to.is_empty() { None } else { Some(f.to) },
                    if f.actor_id.is_empty() {
                        None
                    } else {
                        Some(f.actor_id)
                    },
                    if f.event_type.is_empty() {
                        None
                    } else {
                        Some(f.event_type)
                    },
                    if f.result.is_empty() {
                        None
                    } else {
                        Some(f.result)
                    },
                )
                .await;

            match result_data {
                Ok(data) => {
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
        filters,
        api_client,
        export_action,
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
