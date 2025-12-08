use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub api_base_url: Option<String>,
}

static API_BASE_URL: OnceLock<String> = OnceLock::new();
static TIME_ZONE: OnceLock<RwLock<TimeZoneCache>> = OnceLock::new();
#[cfg(test)]
static TIME_ZONE_FETCH_OVERRIDE: OnceLock<Mutex<Vec<Result<String, String>>>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
struct TimeZoneCache {
    value: Option<String>,
    is_fallback: bool,
    last_error: Option<String>,
    loading: bool,
}

fn time_zone_cache() -> &'static RwLock<TimeZoneCache> {
    TIME_ZONE.get_or_init(|| RwLock::new(TimeZoneCache::default()))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeZoneStatus {
    pub time_zone: Option<String>,
    pub is_fallback: bool,
    pub last_error: Option<String>,
    pub loading: bool,
}

pub fn time_zone_status() -> TimeZoneStatus {
    let guard = time_zone_cache().read().unwrap();
    TimeZoneStatus {
        time_zone: guard.value.clone(),
        is_fallback: guard.is_fallback,
        last_error: guard.last_error.clone(),
        loading: guard.loading,
    }
}

pub async fn refresh_time_zone() -> TimeZoneStatus {
    let _ = ensure_time_zone(true).await;
    time_zone_status()
}

#[cfg(test)]
pub(crate) fn overwrite_time_zone_status_for_test(_status: TimeZoneStatus) {
    let mut guard = time_zone_cache().write().unwrap();
    guard.value = _status.time_zone.clone();
    guard.is_fallback = _status.is_fallback;
    guard.last_error = _status.last_error.clone();
    guard.loading = _status.loading;
}

#[cfg(target_arch = "wasm32")]
fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

#[cfg(not(target_arch = "wasm32"))]
fn window() -> Option<web_sys::Window> {
    None
}

fn get_window_object(name: &str) -> Option<js_sys::Object> {
    let win = window()?;
    let any = js_sys::Reflect::get(&win, &name.into()).ok()?;
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
    get_env_value(&["TIME_ZONE", "time_zone"])
}

fn cache_base_url(value: &str) -> String {
    let value = value.to_string();
    let _ = API_BASE_URL.set(value.clone());
    value
}

fn cache_time_zone(value: &str, is_fallback: bool) -> String {
    let mut guard = time_zone_cache().write().unwrap();
    guard.value = Some(value.to_string());
    guard.is_fallback = is_fallback;
    if !is_fallback {
        guard.last_error = None;
    }
    guard.loading = false;
    value.to_string()
}

fn mark_time_zone_loading() {
    let mut guard = time_zone_cache().write().unwrap();
    guard.loading = true;
}

fn mark_time_zone_error(message: String) -> String {
    let mut guard = time_zone_cache().write().unwrap();
    guard.value = Some("UTC".to_string());
    guard.is_fallback = true;
    guard.last_error = Some(message);
    guard.loading = false;
    "UTC".to_string()
}

#[cfg(test)]
pub(crate) fn queue_mock_time_zone_fetch(result: Result<String, String>) {
    let mutex = TIME_ZONE_FETCH_OVERRIDE.get_or_init(|| Mutex::new(Vec::new()));
    mutex.lock().unwrap().push(result);
}

#[cfg(test)]
fn next_mock_time_zone_fetch() -> Option<Result<String, String>> {
    TIME_ZONE_FETCH_OVERRIDE
        .get()
        .and_then(|mutex| mutex.lock().ok().and_then(|mut stack| stack.pop()))
}

fn write_window_config(cfg: &RuntimeConfig) {
    let Some(w) = window() else {
        return;
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
    if let Some(existing) = snapshot_base_url_from_globals() {
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

fn parse_time_zone(value: &str) -> Tz {
    value.parse::<Tz>().unwrap_or(chrono_tz::UTC)
}

#[derive(Deserialize)]
struct TimeZoneResponse {
    time_zone: String,
}

async fn fetch_time_zone_from_api(base_url: &str) -> Result<String, String> {
    #[cfg(test)]
    if let Some(result) = next_mock_time_zone_fetch() {
        return result;
    }

    let client = reqwest_wasm::Client::new();
    let trimmed = base_url.trim_end_matches('/');
    let url = format!("{}/config/timezone", trimmed);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request {}: {}", url, e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Timezone endpoint {} returned status {}",
            url,
            resp.status()
        ));
    }
    let payload: TimeZoneResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse timezone response: {}", e))?;
    Ok(payload.time_zone)
}

pub fn current_time_zone() -> Tz {
    let cached = {
        let guard = time_zone_cache().read().unwrap();
        guard.value.clone()
    };
    cached
        .map(|value| parse_time_zone(&value))
        .unwrap_or(chrono_tz::UTC)
}

pub async fn await_time_zone() -> Tz {
    ensure_time_zone(false).await
}

async fn ensure_time_zone(force_refresh: bool) -> Tz {
    if !force_refresh {
        if let Some(cached) = {
            let guard = time_zone_cache().read().unwrap();
            guard.value.clone()
        } {
            return parse_time_zone(&cached);
        }
    }

    if let Some(existing) = snapshot_time_zone_from_globals() {
        let cached = cache_time_zone(&existing, false);
        return parse_time_zone(&cached);
    }

    mark_time_zone_loading();
    let base_url = await_api_base_url().await;
    match fetch_time_zone_from_api(&base_url).await {
        Ok(tz_name) => parse_time_zone(&cache_time_zone(&tz_name, false)),
        Err(err) => parse_time_zone(&mark_time_zone_error(err)),
    }
}

pub async fn init() {
    let _ = await_api_base_url().await;
    let _ = await_time_zone().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_fallback_status_when_marked() {
        overwrite_time_zone_status_for_test(TimeZoneStatus {
            time_zone: Some("UTC".into()),
            is_fallback: true,
            last_error: Some("network error".into()),
            loading: false,
        });
        let status = time_zone_status();
        assert_eq!(status.time_zone.as_deref(), Some("UTC"));
        assert!(status.is_fallback);
        assert_eq!(status.last_error.as_deref(), Some("network error"));
    }

    #[test]
    fn clears_error_after_refresh() {
        overwrite_time_zone_status_for_test(TimeZoneStatus {
            time_zone: Some("UTC".into()),
            is_fallback: true,
            last_error: Some("network error".into()),
            loading: false,
        });
        queue_mock_time_zone_fetch(Ok("Asia/Tokyo".into()));
        let refreshed = futures::executor::block_on(async { refresh_time_zone().await });
        assert!(!refreshed.is_fallback);
        assert!(refreshed.last_error.is_none());
        assert_eq!(refreshed.time_zone.as_deref(), Some("Asia/Tokyo"));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test(async)]
    async fn reads_api_base_url_from_window_env() {
        let window = web_sys::window().expect("window available");
        let env = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &env,
            &"API_BASE_URL".into(),
            &wasm_bindgen::JsValue::from_str("https://example.test/api"),
        );
        let _ = js_sys::Reflect::set(&window, &"__TIMEKEEPER_ENV".into(), &env);

        let resolved = await_api_base_url().await;
        assert_eq!(resolved, "https://example.test/api");
    }
}
