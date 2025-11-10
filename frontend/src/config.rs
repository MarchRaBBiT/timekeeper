use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub api_base_url: Option<String>,
}

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

fn snapshot_from_globals() -> RuntimeConfig {
    if let Some(env_url) = get_from_env_js() {
        return RuntimeConfig {
            api_base_url: Some(env_url),
        };
    }
    RuntimeConfig {
        api_base_url: get_from_window_config(),
    }
}

pub fn resolve_api_base_url() -> String {
    snapshot_from_globals()
        .api_base_url
        .unwrap_or_else(|| "http://localhost:3000/api".to_string())
}

pub async fn init() {
    // If env.js or a pre-populated window config is present, nothing to do.
    if snapshot_from_globals().api_base_url.is_some() {
        return;
    }

    // Try to fetch ./config.json and stash into window.__TIMEKEEPER_CONFIG
    let url = "./config.json";
    let resp = match reqwest_wasm::get(url).await {
        Ok(r) => r,
        Err(_) => return, // No config file; keep defaults
    };
    if !resp.status().is_success() {
        return;
    }
    match resp.json::<RuntimeConfig>().await {
        Ok(cfg) => {
            let w = window();
            let obj = js_sys::Object::new();
            if let Some(v) = cfg.api_base_url {
                let _ = js_sys::Reflect::set(
                    &obj,
                    &"api_base_url".into(),
                    &wasm_bindgen::JsValue::from_str(&v),
                );
            }
            let _ = js_sys::Reflect::set(&w, &"__TIMEKEEPER_CONFIG".into(), &obj);
        }
        Err(_) => {}
    }
}
