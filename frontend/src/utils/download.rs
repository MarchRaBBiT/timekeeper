#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub fn trigger_csv_download(filename: &str, csv_data: &str) -> Result<(), String> {
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(csv_data));
    let blob = web_sys::Blob::new_with_str_sequence(&array)
        .map_err(|_| "Failed to create blob".to_string())?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Failed to create object URL".to_string())?;

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("No document")?;
    let element = document
        .create_element("a")
        .map_err(|_| "Failed to create link".to_string())?;
    let a = element
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "Failed to cast anchor".to_string())?;
    a.set_href(&url);
    a.set_download(filename);
    a.style().set_property("display", "none").ok();
    document
        .body()
        .ok_or("No body")?
        .append_child(&a)
        .map_err(|_| "Append failed".to_string())?;
    a.click();
    a.remove();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_csv_download(_filename: &str, _csv_data: &str) -> Result<(), String> {
    Err("CSV download is only available in wasm".to_string())
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn trigger_csv_download_succeeds() {
        let result = trigger_csv_download("test.csv", "a,b\n1,2\n");
        assert!(result.is_ok());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;

    #[test]
    fn trigger_csv_download_returns_error_on_host() {
        let result = trigger_csv_download("test.csv", "a,b\n1,2\n");
        assert!(result.is_err());
    }
}
