use web_sys::{Storage, Window};

pub fn window() -> Result<Window, String> {
    web_sys::window().ok_or_else(|| "No window object".to_string())
}

pub fn local_storage() -> Result<Storage, String> {
    window()?
        .local_storage()
        .map_err(|_| "No localStorage".to_string())?
        .ok_or_else(|| "No localStorage".to_string())
}
