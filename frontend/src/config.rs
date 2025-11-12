use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub api_base_url: Option<String>,
}

static API_BASE_URL: OnceLock<String> = OnceLock::new();

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn get_from_env_js() -> Option<String> {
    // Expect optional global object: window.__TIMEKEEPER_ENV = { API_BASE_URL: "..." }
    let w = window();
    let any = js_sys::Reflect::get(&w, &"__TIMEKEEPER_ENV".into()).ok()?;
    if any.is_undefined() || any.is_null() {
        return None;
    }
    let obj = js_sys::Object::from(any);
    // Try upper and lower case keys
    let val = js_sys::Reflect::get(&obj, &"API_BASE_URL".into())
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .or_else(|| js_sys::Reflect::get(&obj, &"api_base_url".into()).ok());
    val.and_then(|v| v.as_string())
}

fn get_from_window_config() -> Option<String> {
    // Expect optional global object: window.__TIMEKEEPER_CONFIG = { api_base_url: "..." }
    let w = window();
    let any = js_sys::Reflect::get(&w, &"__TIMEKEEPER_CONFIG".into()).ok()?;
    if any.is_undefined() || any.is_null() {
        return None;
    }
    let obj = js_sys::Object::from(any);
    let val = js_sys::Reflect::get(&obj, &"api_base_url".into())
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .or_else(|| js_sys::Reflect::get(&obj, &"API_BASE_URL".into()).ok());
    val.and_then(|v| v.as_string())
}

fn snapshot_from_globals() -> Option<String> {
    if let Some(env_url) = get_from_env_js() {
        return Some(env_url);
    }
    get_from_window_config()
}

fn cache_base_url(value: &str) -> String {
    let value = value.to_string();
    let _ = API_BASE_URL.set(value.clone());
    value
}

fn write_window_config(cfg: &RuntimeConfig) {
    if cfg.api_base_url.is_none() {
        return;
    }
    let w = match web_sys::window() {
        Some(win) => win,
        None => return,
    };
    let obj = js_sys::Object::new();
    if let Some(url) = &cfg.api_base_url {
        let _ = js_sys::Reflect::set(
            &obj,
            &"api_base_url".into(),
            &wasm_bindgen::JsValue::from_str(url),
        );
    }
    let _ = js_sys::Reflect::set(&w, &"__TIMEKEEPER_CONFIG".into(), &obj);
}

async fn fetch_runtime_config() -> Option<RuntimeConfig> {
    let resp = reqwest_wasm::get("./config.json").await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<RuntimeConfig>().await.ok()
}

pub async fn await_api_base_url() -> String {
    if let Some(cached) = API_BASE_URL.get() {
        return cached.clone();
    }
    if let Some(existing) = snapshot_from_globals() {
        return cache_base_url(&existing);
    }
    if let Some(cfg) = fetch_runtime_config().await {
        write_window_config(&cfg);
        if let Some(url) = cfg.api_base_url {
            return cache_base_url(&url);
        }
    }
    cache_base_url("http://localhost:3000/api")
}

pub async fn init() {
    let _ = await_api_base_url().await;
}
