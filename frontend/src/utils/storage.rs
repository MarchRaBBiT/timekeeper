use web_sys::{Storage, Window};

fn window() -> Result<Window, String> {
    web_sys::window().ok_or_else(|| "No window object".to_string())
}

pub fn local_storage() -> Result<Storage, String> {
    window()?
        .local_storage()
        .map_err(|_| "No localStorage".to_string())?
        .ok_or_else(|| "No localStorage".to_string())
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn local_storage_is_available() {
        assert!(local_storage().is_ok());
    }
}
