use leptos::*;
use leptos_router::*;

use crate::{
    components::guard::RequireAuth,
    pages::{
        admin::AdminPage, admin_audit_logs::AdminAuditLogsPage, admin_export::AdminExportPage,
        admin_users::AdminUsersPage, attendance::AttendancePage, dashboard::DashboardPage,
        forgot_password::ForgotPasswordPage, home::HomePage, login::LoginPage,
        mfa::MfaRegisterPage, requests::RequestsPage, reset_password::ResetPasswordPage,
        settings::SettingsPage,
    },
    state::auth::AuthProvider,
};

pub const ROUTE_PATHS: &[&str] = &[
    "/",
    "/login",
    "/forgot-password",
    "/reset-password",
    "/dashboard",
    "/attendance",
    "/requests",
    "/mfa/register",
    "/settings",
    "/admin",
    "/admin/users",
    "/admin/export",
    "/admin/audit-logs",
];

pub const PROTECTED_ROUTE_PATHS: &[&str] = &[
    "/dashboard",
    "/attendance",
    "/requests",
    "/settings",
    "/admin",
    "/admin/users",
    "/admin/export",
    "/admin/audit-logs",
];

pub const PUBLIC_ROUTE_PATHS: &[&str] = &[
    "/",
    "/login",
    "/mfa/register",
    "/forgot-password",
    "/reset-password",
];

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
                    <Route path="/forgot-password" view=ForgotPasswordPage/>
                    <Route path="/reset-password" view=ResetPasswordPage/>
                    <Route path="/dashboard" view=ProtectedDashboard/>
                    <Route path="/attendance" view=ProtectedAttendance/>
                    <Route path="/requests" view=ProtectedRequests/>
                    <Route path="/mfa/register" view=MfaRegisterPage/> // Keeping for direct access if needed, or redirect?
                    <Route path="/settings" view=ProtectedSettings/>
                    <Route path="/admin" view=ProtectedAdmin/>
                    <Route path="/admin/users" view=ProtectedAdminUsers/>
                    <Route path="/admin/export" view=ProtectedAdminExport/>
                    <Route path="/admin/audit-logs" view=ProtectedAdminAuditLogs/>
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
fn ProtectedSettings() -> impl IntoView {
    view! { <RequireAuth><SettingsPage/></RequireAuth> }
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

#[component]
fn ProtectedAdminAuditLogs() -> impl IntoView {
    view! { <RequireAuth><AdminAuditLogsPage/></RequireAuth> }
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

    #[test]
    fn public_and_protected_routes_are_disjoint() {
        let protected: HashSet<&str> = PROTECTED_ROUTE_PATHS.iter().copied().collect();
        let public: HashSet<&str> = PUBLIC_ROUTE_PATHS.iter().copied().collect();
        assert!(protected.is_disjoint(&public));
    }

    #[test]
    fn public_routes_are_subset_of_all() {
        let all: HashSet<&str> = ROUTE_PATHS.iter().copied().collect();
        for path in PUBLIC_ROUTE_PATHS {
            assert!(
                all.contains(path),
                "public path missing from ROUTE_PATHS: {}",
                path
            );
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::{render_to_string, with_local_runtime_async};
    use leptos_router::{RouterIntegrationContext, ServerIntegration};

    #[test]
    fn app_root_renders_route_shell() {
        with_local_runtime_async(|| async {
            let runtime = leptos::create_runtime();
            provide_context(RouterIntegrationContext::new(ServerIntegration {
                path: "http://localhost/".to_string(),
            }));
            leptos_reactive::suppress_resource_load(true);
            let html = app_root().into_view().render_to_string().to_string();
            leptos_reactive::suppress_resource_load(false);
            assert!(!html.is_empty());
            runtime.dispose();
        });
    }

    #[test]
    fn protected_views_render_with_auth_context() {
        let html = render_to_string(move || {
            provide_context(RouterIntegrationContext::new(ServerIntegration {
                path: "http://localhost/dashboard".to_string(),
            }));
            provide_auth(Some(admin_user(true)));
            view! {
                <div>
                    <ProtectedDashboard />
                    <ProtectedAttendance />
                    <ProtectedRequests />
                    <ProtectedSettings />
                    <ProtectedAdmin />
                    <ProtectedAdminUsers />
                    <ProtectedAdminExport />
                    <ProtectedAdminAuditLogs />
                </div>
            }
        });
        assert!(!html.is_empty());
    }
}
