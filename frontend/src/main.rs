#[cfg(target_arch = "wasm32")]
fn main() {
    use js_sys::Date;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::console;

    use timekeeper_frontend::{config, router, theme};

    console_error_panic_hook::set_once();
    theme::init_system_theme();
    let t0 = Date::now();
    console::log_1(&"Starting Timekeeper Frontend: initializing runtime config".into());

    spawn_local(async move {
        config::init().await;
        let elapsed = Date::now() - t0;
        console::log_1(&format!("Runtime config initialized ({} ms)", elapsed).into());
        router::mount_app();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    #[test]
    fn main_is_noop_on_host() {
        super::main();
    }
}
