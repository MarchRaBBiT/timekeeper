use js_sys::Date;
use leptos::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

mod api;
mod components;
pub mod config;
mod pages;
pub mod router;
mod state;
pub mod theme;
pub mod utils;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    theme::init_system_theme();
    let t0 = Date::now();
    console::log_1(&"Starting Timekeeper Frontend (wasm): initializing runtime config".into());

    spawn_local(async move {
        config::init().await;
        let elapsed = Date::now() - t0;
        let msg = format!("Runtime config initialized ({} ms)", elapsed);
        web_sys::console::log_1(&msg.into());
        router::mount_app();
    });
}
