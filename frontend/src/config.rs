use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub api_base_url: Option<String>,
    pub time_zone: Option<String>,
}

static API_BASE_URL: OnceLock<String> = OnceLock::new();
static TIME_ZONE: OnceLock<String> = OnceLock::new();

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn get_window_object(name: &str) -> Option<js_sys::Object> {
    let any = js_sys::Reflect::get(&window(), &name.into()).ok()?;
    if any.is_undefined() || any.is_null() {
        return None;
    }
    Some(js_sys::Object::from(any))
}

fn read_property(obj: &js_sys::Object, key: &str) -> Option<String> {
    js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .and_then(|v| v.as_string())
}

fn read_property_aliases(obj: &js_sys::Object, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = read_property(obj, key) {
            return Some(value);
        }
    }
    None
}

fn get_env_value(keys: &[&str]) -> Option<String> {
    get_window_object("__TIMEKEEPER_ENV").and_then(|obj| read_property_aliases(&obj, keys))
}

fn get_window_config_value(keys: &[&str]) -> Option<String> {
    get_window_object("__TIMEKEEPER_CONFIG").and_then(|obj| read_property_aliases(&obj, keys))
}

fn snapshot_base_url_from_globals() -> Option<String> {
    if let Some(env_url) = get_env_value(&["API_BASE_URL", "api_base_url"]) {
        return Some(env_url);
    }
    get_window_config_value(&["api_base_url", "API_BASE_URL"])
}

fn snapshot_time_zone_from_globals() -> Option<String> {
    if let Some(env_tz) = get_env_value(&["TIME_ZONE", "time_zone"]) {
        return Some(env_tz);
    }
    get_window_config_value(&["time_zone", "TIME_ZONE"])
}

fn cache_base_url(value: &str) -> String {
    let value = value.to_string();
    let _ = API_BASE_URL.set(value.clone());
    value
}

fn cache_time_zone(value: &str) -> String {
    let value = value.to_string();
    let _ = TIME_ZONE.set(value.clone());
    value
}

fn write_window_config(cfg: &RuntimeConfig) {
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
    if let Some(tz) = &cfg.time_zone {
        let _ = js_sys::Reflect::set(
            &obj,
            &"time_zone".into(),
            &wasm_bindgen::JsValue::from_str(tz),
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
    if TIME_ZONE.get().is_none() {
        if let Some(tz) = snapshot_time_zone_from_globals() {
            cache_time_zone(&tz);
        }
    }
    if let Some(existing) = snapshot_base_url_from_globals() {
        return cache_base_url(&existing);
    }
    if let Some(cfg) = fetch_runtime_config().await {
        write_window_config(&cfg);
        if let Some(tz) = cfg.time_zone.as_deref() {
            if TIME_ZONE.get().is_none() {
                cache_time_zone(tz);
            }
        }
        if let Some(url) = cfg.api_base_url {
            return cache_base_url(&url);
        }
    }
    cache_base_url("http://localhost:3000/api")
}

fn parse_time_zone(value: &str) -> Tz {
    value.parse::<Tz>().unwrap_or(chrono_tz::UTC)
}

pub fn current_time_zone() -> Tz {
    TIME_ZONE
        .get()
        .map(|tz| parse_time_zone(tz))
        .unwrap_or(chrono_tz::UTC)
}

pub async fn await_time_zone() -> Tz {
    if let Some(cached) = TIME_ZONE.get() {
        return parse_time_zone(cached);
    }
    if let Some(existing) = snapshot_time_zone_from_globals() {
        return parse_time_zone(&cache_time_zone(&existing));
    }
    if let Some(cfg) = fetch_runtime_config().await {
        write_window_config(&cfg);
        if let Some(url) = cfg.api_base_url {
            if API_BASE_URL.get().is_none() {
                cache_base_url(&url);
            }
        }
        if let Some(tz) = cfg.time_zone {
            return parse_time_zone(&cache_time_zone(&tz));
        }
    }
    parse_time_zone(&cache_time_zone("UTC"))
}

pub async fn init() {
    let _ = await_api_base_url().await;
    let _ = await_time_zone().await;
}
