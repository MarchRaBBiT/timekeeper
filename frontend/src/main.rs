use leptos::*;
use leptos_router::*;
use web_sys::console;
use wasm_bindgen_futures::spawn_local;

mod api;
mod components;
mod config;
mod pages;
mod state;
mod utils;

use pages::{
    admin::AdminPage, attendance::AttendancePage, dashboard::DashboardPage, home::HomePage,
    login::LoginPage, mfa::MfaRegisterPage, requests::RequestsPage,
};

fn main() {
    console_error_panic_hook::set_once();
    let perf = web_sys::window().and_then(|w| w.performance().ok());
    let t0 = perf.as_ref().map(|p| p.now());
    console::log_1(&"Starting Timekeeper Frontend: initializing runtime config".into());

    spawn_local(async move {
        config::init().await;
        if let (Some(p), Some(start)) = (perf.as_ref(), t0) {
            let elapsed = p.now() - start;
            console::log_1(&format!("Runtime config initialized ({} ms)", elapsed).into());
        } else {
            console::log_1(&"Runtime config initialized".into());
        }
        mount_to_body(|| {
            view! {
                <Router>
                    <Routes>
                        <Route path="/" view=HomePage/>
                        <Route path="/login" view=LoginPage/>
                        <Route path="/dashboard" view=DashboardPage/>
                        <Route path="/attendance" view=AttendancePage/>
                        <Route path="/requests" view=RequestsPage/>
                        <Route path="/mfa/register" view=MfaRegisterPage/>
                        <Route path="/admin" view=AdminPage/>
                    </Routes>
                </Router>
            }
        });
    });
}
