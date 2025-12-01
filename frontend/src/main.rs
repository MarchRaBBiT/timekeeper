use js_sys::Date;
use leptos::*;
use leptos_router::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

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
    let t0 = Date::now();
    console::log_1(&"Starting Timekeeper Frontend: initializing runtime config".into());

    spawn_local(async move {
        config::init().await;
        let elapsed = Date::now() - t0;
        console::log_1(&format!("Runtime config initialized ({} ms)", elapsed).into());
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
