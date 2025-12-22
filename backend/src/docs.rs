#![allow(dead_code)] // OpenAPI doc stubs are only referenced by utoipa macros.

use crate::{
    handlers::{
        admin::{
            AdminAttendanceUpsert, AdminBreakItem, AdminHolidayKind, AdminHolidayListItem,
            AdminHolidayListQuery, AdminHolidayListResponse, AdminRequestListPageInfo,
            AdminRequestListResponse, ExportQuery, RequestListQuery, ResetMfaPayload,
        },
        attendance::{AttendanceExportQuery, AttendanceQuery, AttendanceStatusResponse},
    },
    models::{
        attendance::{
            AttendanceResponse, AttendanceSummary, BreakEndRequest, BreakStartRequest,
            ClockInRequest, ClockOutRequest,
        },
        break_record::BreakRecordResponse,
        holiday::{
            CreateHolidayPayload, CreateWeeklyHolidayPayload, HolidayResponse,
            WeeklyHolidayResponse,
        },
        leave_request::{CreateLeaveRequest, LeaveRequestResponse, LeaveType},
        overtime_request::{CreateOvertimeRequest, OvertimeRequestResponse},
        request::RequestStatus,
        user::{
            ChangePasswordRequest, CreateUser, LoginRequest, LoginResponse, MfaCodeRequest,
            MfaSetupResponse, MfaStatusResponse, UpdateUser, UserResponse,
        },
    },
};
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        login_doc,
        refresh_doc,
        me_doc,
        mfa_status_doc,
        mfa_setup_doc,
        mfa_activate_doc,
        mfa_disable_doc,
        change_password_doc,
        logout_doc,
        clock_in_doc,
        clock_out_doc,
        break_start_doc,
        break_end_doc,
        attendance_status_doc,
        my_attendance_doc,
        my_attendance_summary_doc,
        export_attendance_doc,
        create_leave_doc,
        create_overtime_doc,
        my_requests_doc,
        admin_list_requests_doc,
        admin_request_detail_doc,
        admin_approve_request_doc,
        admin_reject_request_doc,
        admin_get_users_doc,
        admin_create_user_doc,
        admin_list_holidays_doc,
        admin_create_holiday_doc,
        admin_delete_holiday_doc,
        admin_list_weekly_holidays_doc,
        admin_create_weekly_holiday_doc,
        admin_export_doc,
        system_admin_reset_mfa_doc
    ),
    components(
        schemas(
            // auth
            LoginRequest,
            LoginResponse,
            ChangePasswordRequest,
            MfaCodeRequest,
            MfaSetupResponse,
            MfaStatusResponse,
            // users
            CreateUser,
            UpdateUser,
            UserResponse,
            // attendance & breaks
            ClockInRequest,
            ClockOutRequest,
            BreakStartRequest,
            BreakEndRequest,
            AttendanceResponse,
            AttendanceSummary,
            AttendanceStatusResponse,
            BreakRecordResponse,
            AttendanceQuery,
            AttendanceExportQuery,
            // requests
            CreateLeaveRequest,
            LeaveRequestResponse,
            LeaveType,
            CreateOvertimeRequest,
            OvertimeRequestResponse,
            RequestStatus,
            RequestListQuery,
            AdminRequestListResponse,
            AdminRequestListPageInfo,
            // holidays
            CreateHolidayPayload,
            HolidayResponse,
            CreateWeeklyHolidayPayload,
            WeeklyHolidayResponse,
            // admin-specific payloads
            AdminAttendanceUpsert,
            AdminBreakItem,
            ResetMfaPayload,
            AdminHolidayListQuery,
            AdminHolidayListResponse,
            AdminHolidayKind,
            AdminHolidayListItem,
            ExportQuery
        )
    ),
    modifiers(&SecuritySchemes),
    tags(
        (name = "Auth", description = "認証・MFA・パスワード関連"),
        (name = "Attendance", description = "勤怠・休憩・サマリー API"),
        (name = "Requests", description = "申請 API (休暇/残業)"),
        (name = "Admin", description = "管理者向け API")
    ),
    security(("BearerAuth" = []))
)]
pub struct ApiDoc;

struct SecuritySchemes;

impl Modify for SecuritySchemes {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();

        let mut bearer = Http::new(HttpAuthScheme::Bearer);
        bearer.bearer_format = Some("JWT".to_string());

        components.add_security_scheme("BearerAuth", SecurityScheme::Http(bearer));
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "ログイン成功", body = LoginResponse),
        (status = 401, description = "認証失敗")
    ),
    tag = "Auth",
    security(())
)]
fn login_doc() {}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = serde_json::Value,
    responses((status = 200, description = "トークン更新", body = LoginResponse)),
    tag = "Auth",
    security(())
)]
fn refresh_doc() {}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses((status = 200, description = "ログイン中のユーザー", body = UserResponse)),
    tag = "Auth"
)]
fn me_doc() {}

#[utoipa::path(
    get,
    path = "/api/auth/mfa",
    responses((status = 200, body = MfaStatusResponse)),
    tag = "Auth"
)]
fn mfa_status_doc() {}

#[utoipa::path(
    post,
    path = "/api/auth/mfa/setup",
    responses((status = 200, body = MfaSetupResponse)),
    tag = "Auth"
)]
fn mfa_setup_doc() {}

#[utoipa::path(
    post,
    path = "/api/auth/mfa/activate",
    request_body = MfaCodeRequest,
    responses((status = 200, description = "MFA 有効化", body = serde_json::Value)),
    tag = "Auth"
)]
fn mfa_activate_doc() {}

#[utoipa::path(
    delete,
    path = "/api/auth/mfa",
    request_body = MfaCodeRequest,
    responses((status = 200, description = "MFA 無効化", body = serde_json::Value)),
    tag = "Auth"
)]
fn mfa_disable_doc() {}

#[utoipa::path(
    put,
    path = "/api/auth/change-password",
    request_body = ChangePasswordRequest,
    responses((status = 200, description = "パスワード変更完了", body = serde_json::Value)),
    tag = "Auth"
)]
fn change_password_doc() {}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    request_body = serde_json::Value,
    responses((status = 200, description = "ログアウト", body = serde_json::Value)),
    tag = "Auth"
)]
fn logout_doc() {}

#[utoipa::path(
    post,
    path = "/api/attendance/clock-in",
    request_body = ClockInRequest,
    responses((status = 200, body = AttendanceResponse)),
    tag = "Attendance"
)]
fn clock_in_doc() {}

#[utoipa::path(
    post,
    path = "/api/attendance/clock-out",
    request_body = ClockOutRequest,
    responses((status = 200, body = AttendanceResponse)),
    tag = "Attendance"
)]
fn clock_out_doc() {}

#[utoipa::path(
    post,
    path = "/api/attendance/break-start",
    request_body = BreakStartRequest,
    responses((status = 200, body = BreakRecordResponse)),
    tag = "Attendance"
)]
fn break_start_doc() {}

#[utoipa::path(
    post,
    path = "/api/attendance/break-end",
    request_body = BreakEndRequest,
    responses((status = 200, body = BreakRecordResponse)),
    tag = "Attendance"
)]
fn break_end_doc() {}

#[utoipa::path(
    get,
    path = "/api/attendance/status",
    params(
        ("date" = String, Query, description = "対象日 YYYY-MM-DD")
    ),
    responses((status = 200, body = AttendanceStatusResponse)),
    tag = "Attendance"
)]
fn attendance_status_doc() {}

#[utoipa::path(
    get,
    path = "/api/attendance/me",
    params(AttendanceQuery),
    responses((status = 200, body = [AttendanceResponse])),
    tag = "Attendance"
)]
fn my_attendance_doc() {}

#[utoipa::path(
    get,
    path = "/api/attendance/me/summary",
    params(AttendanceQuery),
    responses((status = 200, body = AttendanceSummary)),
    tag = "Attendance"
)]
fn my_attendance_summary_doc() {}

#[utoipa::path(
    get,
    path = "/api/attendance/export",
    params(AttendanceExportQuery),
    responses((status = 200, description = "CSV データを含む JSON", body = serde_json::Value)),
    tag = "Attendance"
)]
fn export_attendance_doc() {}

#[utoipa::path(
    post,
    path = "/api/requests/leave",
    request_body = CreateLeaveRequest,
    responses((status = 200, body = LeaveRequestResponse)),
    tag = "Requests"
)]
fn create_leave_doc() {}

#[utoipa::path(
    post,
    path = "/api/requests/overtime",
    request_body = CreateOvertimeRequest,
    responses((status = 200, body = OvertimeRequestResponse)),
    tag = "Requests"
)]
fn create_overtime_doc() {}

#[utoipa::path(
    get,
    path = "/api/requests/me",
    responses((status = 200, body = serde_json::Value)),
    tag = "Requests"
)]
fn my_requests_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/requests",
    params(RequestListQuery),
    responses((status = 200, body = AdminRequestListResponse)),
    tag = "Admin"
)]
fn admin_list_requests_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/requests/{id}",
    params(("id" = String, Path, description = "申請ID")),
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn admin_request_detail_doc() {}

#[utoipa::path(
    put,
    path = "/api/admin/requests/{id}/approve",
    params(("id" = String, Path, description = "申請ID")),
    request_body = ApprovePayload,
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn admin_approve_request_doc() {}

#[utoipa::path(
    put,
    path = "/api/admin/requests/{id}/reject",
    params(("id" = String, Path, description = "申請ID")),
    request_body = RejectPayload,
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn admin_reject_request_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/users",
    responses((status = 200, body = [UserResponse])),
    tag = "Admin"
)]
fn admin_get_users_doc() {}

#[utoipa::path(
    post,
    path = "/api/admin/users",
    request_body = CreateUser,
    responses((status = 200, body = UserResponse)),
    tag = "Admin"
)]
fn admin_create_user_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/holidays",
    params(AdminHolidayListQuery),
    responses((status = 200, body = AdminHolidayListResponse)),
    tag = "Admin"
)]
fn admin_list_holidays_doc() {}

#[utoipa::path(
    post,
    path = "/api/admin/holidays",
    request_body = CreateHolidayPayload,
    responses((status = 200, body = HolidayResponse)),
    tag = "Admin"
)]
fn admin_create_holiday_doc() {}

#[utoipa::path(
    delete,
    path = "/api/admin/holidays/{id}",
    params(("id" = String, Path, description = "休日ID")),
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn admin_delete_holiday_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/holidays/weekly",
    responses((status = 200, body = [WeeklyHolidayResponse])),
    tag = "Admin"
)]
fn admin_list_weekly_holidays_doc() {}

#[utoipa::path(
    post,
    path = "/api/admin/holidays/weekly",
    request_body = CreateWeeklyHolidayPayload,
    responses((status = 200, body = WeeklyHolidayResponse)),
    tag = "Admin"
)]
fn admin_create_weekly_holiday_doc() {}

#[utoipa::path(
    get,
    path = "/api/admin/export",
    params(ExportQuery),
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn admin_export_doc() {}

#[utoipa::path(
    post,
    path = "/api/admin/mfa/reset",
    request_body = ResetMfaPayload,
    responses((status = 200, body = serde_json::Value)),
    tag = "Admin"
)]
fn system_admin_reset_mfa_doc() {}
