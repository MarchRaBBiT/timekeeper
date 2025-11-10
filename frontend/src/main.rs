use leptos::*;
use leptos_router::*;
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
    console::log_1(&"Starting Timekeeper Frontend".into());

    leptos::spawn_local(async move {
        config::init().await;
        console::log_1(&"Runtime config initialized".into());
    });

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
    })
}
