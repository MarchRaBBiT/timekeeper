use leptos::*;
use leptos_router::*;
use web_sys::console;

mod api;
mod components;
pub mod config;
mod pages;
mod state;
pub mod utils;

use pages::{
    admin::AdminPage, admin_export::AdminExportPage, admin_users::AdminUsersPage,
    attendance::AttendancePage, dashboard::DashboardPage, home::HomePage, login::LoginPage,
    mfa::MfaRegisterPage, requests::RequestsPage,
};

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console::log_1(&"Starting Timekeeper Frontend (wasm)".into());

    // Kick off runtime config load from ./config.json (non-blocking).
    // If window.__TIMEKEEPER_ENV is present (env.js), it takes precedence.
    leptos::spawn_local(async move {
        config::init().await;
        web_sys::console::log_1(&"Runtime config initialized".into());
    });

    mount_to_body(|| {
        view! {
            <crate::state::auth::AuthProvider>
                <Router>
                    <Routes>
                        <Route path="/" view=HomePage/>
                        <Route path="/login" view=LoginPage/>
                        <Route path="/dashboard" view=ProtectedDashboard/>
                        <Route path="/attendance" view=ProtectedAttendance/>
                        <Route path="/requests" view=ProtectedRequests/>
                        <Route path="/mfa/register" view=MfaRegisterPage/>
                        <Route path="/admin" view=ProtectedAdmin/>
                        <Route path="/admin/users" view=ProtectedAdminUsers/>
                        <Route path="/admin/export" view=ProtectedAdminExport/>
                    </Routes>
                </Router>
            </crate::state::auth::AuthProvider>
        }
    });
}

#[component]
fn ProtectedDashboard() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><DashboardPage/></crate::components::guard::RequireAuth> }
}

#[component]
fn ProtectedAttendance() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><AttendancePage/></crate::components::guard::RequireAuth> }
}

#[component]
fn ProtectedRequests() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><RequestsPage/></crate::components::guard::RequireAuth> }
}

#[component]
fn ProtectedAdmin() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><AdminPage/></crate::components::guard::RequireAuth> }
}

#[component]
fn ProtectedAdminUsers() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><AdminUsersPage/></crate::components::guard::RequireAuth> }
}

#[component]
fn ProtectedAdminExport() -> impl IntoView {
    view! { <crate::components::guard::RequireAuth><AdminExportPage/></crate::components::guard::RequireAuth> }
}
