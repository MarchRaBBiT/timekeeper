use leptos::*;
use leptos_router::*;

use crate::{
    components::guard::RequireAuth,
    pages::{
        admin::AdminPage, admin_export::AdminExportPage, admin_users::AdminUsersPage,
        attendance::AttendancePage, dashboard::DashboardPage, home::HomePage, login::LoginPage,
        mfa::MfaRegisterPage, requests::RequestsPage,
    },
    state::auth::AuthProvider,
};

pub const ROUTE_PATHS: &[&str] = &[
    "/",
    "/login",
    "/dashboard",
    "/attendance",
    "/requests",
    "/mfa/register",
    "/admin",
    "/admin/users",
    "/admin/export",
];

pub const PROTECTED_ROUTE_PATHS: &[&str] = &[
    "/dashboard",
    "/attendance",
    "/requests",
    "/admin",
    "/admin/users",
    "/admin/export",
];

pub const PUBLIC_ROUTE_PATHS: &[&str] = &["/", "/login", "/mfa/register"];

pub fn mount_app() {
    mount_to_body(app_root);
}

pub fn app_root() -> impl IntoView {
    provide_context(crate::api::ApiClient::new());
    view! {
        <AuthProvider>
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
        </AuthProvider>
    }
}

#[component]
fn ProtectedDashboard() -> impl IntoView {
    view! { <RequireAuth><DashboardPage/></RequireAuth> }
}

#[component]
fn ProtectedAttendance() -> impl IntoView {
    view! { <RequireAuth><AttendancePage/></RequireAuth> }
}

#[component]
fn ProtectedRequests() -> impl IntoView {
    view! { <RequireAuth><RequestsPage/></RequireAuth> }
}

#[component]
fn ProtectedAdmin() -> impl IntoView {
    view! { <RequireAuth><AdminPage/></RequireAuth> }
}

#[component]
fn ProtectedAdminUsers() -> impl IntoView {
    view! { <RequireAuth><AdminUsersPage/></RequireAuth> }
}

#[component]
fn ProtectedAdminExport() -> impl IntoView {
    view! { <RequireAuth><AdminExportPage/></RequireAuth> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn route_paths_include_admin_routes() {
        assert!(ROUTE_PATHS.contains(&"/admin/users"));
        assert!(ROUTE_PATHS.contains(&"/admin/export"));
    }

    #[test]
    fn protected_routes_are_subset_of_all() {
        let all: HashSet<&str> = ROUTE_PATHS.iter().copied().collect();
        for path in PROTECTED_ROUTE_PATHS {
            assert!(
                all.contains(path),
                "protected path missing from ROUTE_PATHS: {}",
                path
            );
        }
    }

    #[test]
    fn no_duplicate_routes() {
        let unique: HashSet<&str> = ROUTE_PATHS.iter().copied().collect();
        assert_eq!(unique.len(), ROUTE_PATHS.len());
    }
}
