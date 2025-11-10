use leptos::*;
use leptos_router::*;
use web_sys::console;

mod api;
mod components;
mod config;
mod pages;
mod state;
mod utils;

use pages::*;

fn main() {
    console_error_panic_hook::set_once();
    console::log_1(&"Starting Timekeeper Frontend".into());

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
