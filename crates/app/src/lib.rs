pub mod attendance {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
    use thiserror::Error;
    use timekeeper_domain::WorkDate;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ClockInCommand {
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_in_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ClockOutCommand {
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_out_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct StartBreakCommand {
        pub user_id: String,
        pub attendance_id: String,
        pub break_start_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BreakEndCommand {
        pub user_id: String,
        pub break_id: String,
        pub break_end_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ForceEndBreakCommand {
        pub break_id: String,
        pub break_end_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpsertAttendanceCommand {
        pub user_id: String,
        pub date: NaiveDate,
        pub clock_in_time: NaiveDateTime,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Vec<UpsertBreakInput>,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpsertBreakInput {
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceDay {
        pub attendance_id: String,
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceStatusQuery {
        pub user_id: String,
        pub work_date: WorkDate,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceStatus {
        pub status: String,
        pub attendance_id: Option<String>,
        pub active_break_id: Option<String>,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ActiveBreakSummary {
        pub break_id: String,
        pub attendance_id: String,
        pub user_id: String,
        pub username: String,
        pub full_name: Option<String>,
        pub break_start_time: NaiveDateTime,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GetBreaksByAttendanceQuery {
        pub user_id: String,
        pub attendance_id: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AttendanceRecord {
        pub attendance_id: String,
        pub user_id: String,
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub status: String,
        pub total_work_hours: Option<f64>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AttendancePageItem {
        pub attendance: AttendanceRecord,
        pub break_periods: Vec<BreakPeriod>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AttendancePage {
        pub items: Vec<AttendancePageItem>,
        pub total: i64,
        pub limit: i64,
        pub offset: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ListAttendancePageQuery {
        pub limit: i64,
        pub offset: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ListUserAttendanceQuery {
        pub user_id: String,
        pub from: NaiveDate,
        pub to: NaiveDate,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GetUserAttendanceSummaryQuery {
        pub user_id: String,
        pub year: i32,
        pub month: u32,
        pub from: NaiveDate,
        pub to: NaiveDate,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct UserAttendanceSummary {
        pub month: u32,
        pub year: i32,
        pub total_work_hours: f64,
        pub total_work_days: i32,
        pub average_daily_hours: f64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExportUserAttendanceQuery {
        pub user_id: String,
        pub username: String,
        pub full_name: String,
        pub from: Option<NaiveDate>,
        pub to: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct UserAttendanceExport {
        pub rows: Vec<UserAttendanceExportRow>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct UserAttendanceExportRow {
        pub username: String,
        pub full_name: String,
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub total_work_hours: Option<f64>,
        pub status: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExportAdminAttendanceQuery {
        pub requester_id: String,
        pub requester_is_manager: bool,
        pub requester_is_system_admin: bool,
        pub username: Option<String>,
        pub from: Option<NaiveDate>,
        pub to: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AdminAttendanceExport {
        pub rows: Vec<AdminAttendanceExportRow>,
        pub pii_masked: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AdminAttendanceExportRow {
        pub username: String,
        pub full_name_encrypted: String,
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub total_work_hours: Option<f64>,
        pub status: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AdminAttendanceExportFilters {
        pub username: Option<String>,
        pub from: Option<NaiveDate>,
        pub to: Option<NaiveDate>,
        pub allowed_user_ids: Option<Vec<String>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceCorrectionBreak {
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceCorrectionSnapshot {
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Vec<AttendanceCorrectionBreak>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CorrectionAttendance {
        pub attendance_id: String,
        pub user_id: String,
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AttendanceCorrectionRequestStatus {
        Pending,
        Approved,
        Rejected,
        Cancelled,
        Conflict,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttendanceCorrectionRecord {
        pub id: String,
        pub user_id: String,
        pub attendance_id: String,
        pub date: NaiveDate,
        pub status: AttendanceCorrectionRequestStatus,
        pub reason: String,
        pub original_snapshot: AttendanceCorrectionSnapshot,
        pub proposed_values: AttendanceCorrectionSnapshot,
        pub decision_comment: Option<String>,
        pub approved_by: Option<String>,
        pub approved_at: Option<DateTime<Utc>>,
        pub rejected_by: Option<String>,
        pub rejected_at: Option<DateTime<Utc>>,
        pub cancelled_at: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CreateAttendanceCorrectionCommand {
        pub request_id: String,
        pub user_id: String,
        pub date: NaiveDate,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Option<Vec<AttendanceCorrectionBreak>>,
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpdateAttendanceCorrectionCommand {
        pub request_id: String,
        pub user_id: String,
        pub clock_in_time: Option<NaiveDateTime>,
        pub clock_out_time: Option<NaiveDateTime>,
        pub breaks: Option<Vec<AttendanceCorrectionBreak>>,
        pub reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CancelAttendanceCorrectionCommand {
        pub request_id: String,
        pub user_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ApproveAttendanceCorrectionCommand {
        pub request_id: String,
        pub approver_id: String,
        pub approver_is_manager: bool,
        pub approver_is_system_admin: bool,
        pub comment: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RejectAttendanceCorrectionCommand {
        pub request_id: String,
        pub approver_id: String,
        pub approver_is_manager: bool,
        pub approver_is_system_admin: bool,
        pub comment: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ListAdminAttendanceCorrectionRequestsQuery {
        pub requester_id: String,
        pub requester_is_manager: bool,
        pub requester_is_system_admin: bool,
        pub status: Option<String>,
        pub user_id: Option<String>,
        pub page: Option<i64>,
        pub per_page: Option<i64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GetAdminAttendanceCorrectionRequestQuery {
        pub requester_id: String,
        pub requester_is_manager: bool,
        pub requester_is_system_admin: bool,
        pub request_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AdminAttendanceCorrectionListFilters {
        pub status: Option<String>,
        pub user_id: Option<String>,
        pub allowed_user_ids: Option<Vec<String>>,
        pub page: i64,
        pub per_page: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NewAttendanceCorrectionRequest {
        pub id: String,
        pub user_id: String,
        pub attendance_id: String,
        pub date: NaiveDate,
        pub reason: String,
        pub original_snapshot: AttendanceCorrectionSnapshot,
        pub proposed_values: AttendanceCorrectionSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpdatedAttendanceCorrectionRequest {
        pub id: String,
        pub user_id: String,
        pub reason: String,
        pub proposed_values: AttendanceCorrectionSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ApprovedAttendanceCorrectionRequest {
        pub id: String,
        pub attendance_id: String,
        pub approver_id: String,
        pub comment: String,
        pub original_snapshot: AttendanceCorrectionSnapshot,
        pub proposed_values: AttendanceCorrectionSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RejectedAttendanceCorrectionRequest {
        pub id: String,
        pub approver_id: String,
        pub comment: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct EffectiveAttendanceCorrection {
        pub attendance_id: String,
        pub clock_in_time_corrected: Option<NaiveDateTime>,
        pub clock_out_time_corrected: Option<NaiveDateTime>,
        pub corrected_breaks: Vec<BreakPeriod>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AttendanceReplacement {
        pub user_id: String,
        pub date: NaiveDate,
        pub clock_in_time: NaiveDateTime,
        pub clock_out_time: Option<NaiveDateTime>,
        pub total_work_hours: Option<f64>,
        pub breaks: Vec<ReplacementBreak>,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ReplacementBreak {
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
        pub duration_minutes: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NewClockIn {
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_in_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExistingClockIn {
        pub attendance_id: String,
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_in_time: NaiveDateTime,
        pub clock_out_time: Option<NaiveDateTime>,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ExistingClockOut {
        pub attendance_id: String,
        pub user_id: String,
        pub work_date: WorkDate,
        pub clock_in_time: NaiveDateTime,
        pub clock_out_time: NaiveDateTime,
        pub total_work_hours: Option<f64>,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NewBreakPeriod {
        pub attendance_id: String,
        pub break_start_time: NaiveDateTime,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BreakPeriod {
        pub break_id: String,
        pub attendance_id: String,
        pub break_start_time: NaiveDateTime,
        pub break_end_time: Option<NaiveDateTime>,
        pub duration_minutes: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EndedBreakPeriod {
        pub break_id: String,
        pub attendance_id: String,
        pub break_start_time: NaiveDateTime,
        pub break_end_time: NaiveDateTime,
        pub duration_minutes: i32,
        pub recorded_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum HolidayDecision {
        WorkingDay,
        Holiday { reason: String },
    }

    #[derive(Debug, Error)]
    pub enum ClockInError {
        #[error("already clocked in")]
        AlreadyClockedIn,
        #[error("clock-in rejected for holiday on {work_date}: {reason}")]
        Holiday { work_date: WorkDate, reason: String },
        #[error("attendance repository error: {0}")]
        Repository(String),
        #[error("holiday calendar error: {0}")]
        HolidayCalendar(String),
    }

    #[derive(Debug, Error)]
    pub enum ClockOutError {
        #[error("attendance record not found")]
        AttendanceNotFound,
        #[error("must clock in before clocking out")]
        ClockInRequired,
        #[error("already clocked out")]
        AlreadyClockedOut,
        #[error("break in progress")]
        ActiveBreakInProgress,
        #[error("clock-out rejected for holiday on {work_date}: {reason}")]
        Holiday { work_date: WorkDate, reason: String },
        #[error("attendance repository error: {0}")]
        Repository(String),
        #[error("holiday calendar error: {0}")]
        HolidayCalendar(String),
    }

    #[derive(Debug, Error)]
    pub enum StartBreakError {
        #[error("attendance record not found")]
        AttendanceNotFound,
        #[error("forbidden")]
        Forbidden,
        #[error("must be clocked in to start break")]
        ClockInRequired,
        #[error("break already in progress")]
        ActiveBreakInProgress,
        #[error("break repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum BreakEndError {
        #[error("break record not found")]
        BreakNotFound,
        #[error("attendance record not found")]
        AttendanceNotFound,
        #[error("forbidden")]
        Forbidden,
        #[error("break already ended")]
        BreakAlreadyEnded,
        #[error("break repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum AttendanceStatusError {
        #[error("attendance status repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum GetBreaksByAttendanceError {
        #[error("attendance record not found")]
        AttendanceNotFound,
        #[error("forbidden")]
        Forbidden,
        #[error("break repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum ListActiveBreaksError {
        #[error("active break repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum ListAttendancePageError {
        #[error("attendance page repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum UpsertAttendanceError {
        #[error("attendance upsert repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum ListUserAttendanceError {
        #[error("user attendance repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error)]
    pub enum ExportAdminAttendanceError {
        #[error("forbidden")]
        Forbidden,
        #[error("admin attendance export repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum CreateAttendanceCorrectionError {
        #[error("attendance correction request not found")]
        RequestNotFound,
        #[error("reason is required")]
        ReasonRequired,
        #[error("reason must be between 1 and 500 characters")]
        ReasonTooLong,
        #[error("attendance record not found")]
        AttendanceNotFound,
        #[error("only pending requests can be updated")]
        NotPendingUpdate,
        #[error("only pending requests can be cancelled")]
        NotPendingCancel,
        #[error("at least one field must be changed")]
        NoChanges,
        #[error("clock_in_time is required")]
        ClockInRequired,
        #[error("clock_out_time must be later than clock_in_time")]
        ClockOutBeforeClockIn,
        #[error("break_end_time must be later than break_start_time")]
        BreakEndBeforeStart,
        #[error("break_start_time must be later than clock_in_time")]
        BreakStartBeforeClockIn,
        #[error("break_end_time must be earlier than clock_out_time")]
        BreakEndAfterClockOut,
        #[error("attendance correction repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum AttendanceCorrectionDecisionError {
        #[error("attendance correction request not found")]
        RequestNotFound,
        #[error("forbidden")]
        Forbidden,
        #[error("manager is not authorized for this applicant")]
        ManagerNotAuthorized,
        #[error("admins cannot approve or reject their own requests")]
        SelfDecision,
        #[error("comment is required")]
        CommentRequired,
        #[error("comment must be between 1 and 500 characters")]
        CommentTooLong,
        #[error("request not found or already processed")]
        AlreadyProcessed,
        #[error("attendance record changed after request submission")]
        AttendanceChanged,
        #[error("attendance correction decision repository error: {0}")]
        Repository(String),
    }

    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum AdminAttendanceCorrectionReadError {
        #[error("attendance correction request not found")]
        RequestNotFound,
        #[error("invalid user_id")]
        InvalidUserId,
        #[error("forbidden")]
        Forbidden,
        #[error("manager is not authorized for this applicant")]
        ManagerNotAuthorized,
        #[error("admin attendance correction read repository error: {0}")]
        Repository(String),
    }

    #[async_trait]
    pub trait AttendanceRepository: Send + Sync {
        async fn find_by_user_and_date(
            &self,
            user_id: &str,
            work_date: WorkDate,
        ) -> Result<Option<AttendanceDay>, ClockInError>;

        async fn create_clock_in(&self, record: NewClockIn) -> Result<AttendanceDay, ClockInError>;

        async fn update_clock_in(
            &self,
            record: ExistingClockIn,
        ) -> Result<AttendanceDay, ClockInError>;
    }

    #[async_trait]
    pub trait HolidayCalendar: Send + Sync {
        async fn decision_for(
            &self,
            user_id: &str,
            work_date: WorkDate,
        ) -> Result<HolidayDecision, ClockInError>;
    }

    #[async_trait]
    pub trait WorkdayCalendar<E>: Send + Sync {
        async fn decision_for(
            &self,
            user_id: &str,
            work_date: WorkDate,
        ) -> Result<HolidayDecision, E>;
    }

    #[async_trait]
    pub trait ClockOutRepository: Send + Sync {
        async fn find_by_user_and_date(
            &self,
            user_id: &str,
            work_date: WorkDate,
        ) -> Result<Option<AttendanceDay>, ClockOutError>;

        async fn has_active_break(&self, attendance_id: &str) -> Result<bool, ClockOutError>;

        async fn total_break_minutes(&self, attendance_id: &str) -> Result<i64, ClockOutError>;

        async fn update_clock_out(
            &self,
            record: ExistingClockOut,
        ) -> Result<AttendanceDay, ClockOutError>;
    }

    #[async_trait]
    pub trait StartBreakRepository: Send + Sync {
        async fn find_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<AttendanceDay, StartBreakError>;

        async fn has_active_break(&self, attendance_id: &str) -> Result<bool, StartBreakError>;

        async fn create_break(
            &self,
            record: NewBreakPeriod,
        ) -> Result<BreakPeriod, StartBreakError>;
    }

    #[async_trait]
    pub trait BreakEndRepository: Send + Sync {
        async fn find_break(&self, break_id: &str) -> Result<BreakPeriod, BreakEndError>;

        async fn find_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<AttendanceDay, BreakEndError>;

        async fn update_break(
            &self,
            record: EndedBreakPeriod,
        ) -> Result<BreakPeriod, BreakEndError>;

        async fn recalculate_total_hours(
            &self,
            attendance_id: &str,
            recorded_at: DateTime<Utc>,
        ) -> Result<(), BreakEndError>;
    }

    #[async_trait]
    pub trait AttendanceStatusReadRepository: Send + Sync {
        async fn find_by_user_and_date(
            &self,
            user_id: &str,
            work_date: WorkDate,
        ) -> Result<Option<AttendanceDay>, AttendanceStatusError>;

        async fn active_break_id(
            &self,
            attendance_id: &str,
        ) -> Result<Option<String>, AttendanceStatusError>;
    }

    #[async_trait]
    pub trait GetBreaksByAttendanceRepository: Send + Sync {
        async fn find_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<AttendanceDay, GetBreaksByAttendanceError>;

        async fn breaks_for_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<Vec<BreakPeriod>, GetBreaksByAttendanceError>;
    }

    #[async_trait]
    pub trait ListActiveBreaksRepository: Send + Sync {
        async fn list_active_breaks(
            &self,
        ) -> Result<Vec<ActiveBreakSummary>, ListActiveBreaksError>;
    }

    #[async_trait]
    pub trait ListAttendancePageRepository: Send + Sync {
        async fn count_attendance(&self) -> Result<i64, ListAttendancePageError>;

        async fn list_attendance(
            &self,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<AttendanceRecord>, ListAttendancePageError>;

        async fn breaks_for_attendance_ids(
            &self,
            attendance_ids: &[String],
        ) -> Result<Vec<BreakPeriod>, ListAttendancePageError>;
    }

    #[async_trait]
    pub trait UpsertAttendanceRepository: Send + Sync {
        async fn replace_attendance(
            &self,
            replacement: AttendanceReplacement,
        ) -> Result<AttendancePageItem, UpsertAttendanceError>;
    }

    #[async_trait]
    pub trait ListUserAttendanceRepository: Send + Sync {
        async fn list_user_attendance(
            &self,
            user_id: &str,
            from: NaiveDate,
            to: NaiveDate,
        ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError>;

        async fn breaks_for_attendance_ids(
            &self,
            attendance_ids: &[String],
        ) -> Result<Vec<BreakPeriod>, ListUserAttendanceError>;

        async fn effective_corrections_for_attendance_ids(
            &self,
            attendance_ids: &[String],
        ) -> Result<Vec<EffectiveAttendanceCorrection>, ListUserAttendanceError>;

        async fn list_user_attendance_with_optional_range(
            &self,
            user_id: &str,
            from: Option<NaiveDate>,
            to: Option<NaiveDate>,
        ) -> Result<Vec<AttendanceRecord>, ListUserAttendanceError>;
    }

    #[async_trait]
    pub trait ExportAdminAttendanceRepository: Send + Sync {
        async fn list_subordinate_user_ids(
            &self,
            manager_id: &str,
        ) -> Result<Vec<String>, ExportAdminAttendanceError>;

        async fn list_admin_attendance_export(
            &self,
            filters: AdminAttendanceExportFilters,
        ) -> Result<Vec<AdminAttendanceExportRow>, ExportAdminAttendanceError>;
    }

    #[async_trait]
    pub trait CreateAttendanceCorrectionRepository: Send + Sync {
        async fn find_attendance_by_user_and_date(
            &self,
            user_id: &str,
            date: NaiveDate,
        ) -> Result<Option<CorrectionAttendance>, CreateAttendanceCorrectionError>;

        async fn breaks_for_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<Vec<AttendanceCorrectionBreak>, CreateAttendanceCorrectionError>;

        async fn create_attendance_correction_request(
            &self,
            request: NewAttendanceCorrectionRequest,
        ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError>;
    }

    #[async_trait]
    pub trait UpdateAttendanceCorrectionRepository: Send + Sync {
        async fn find_attendance_correction_request_for_user(
            &self,
            request_id: &str,
            user_id: &str,
        ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError>;

        async fn update_pending_attendance_correction_request(
            &self,
            request: UpdatedAttendanceCorrectionRequest,
        ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError>;
    }

    #[async_trait]
    pub trait CancelAttendanceCorrectionRepository: Send + Sync {
        async fn cancel_pending_attendance_correction_request(
            &self,
            request_id: &str,
            user_id: &str,
        ) -> Result<(), CreateAttendanceCorrectionError>;
    }

    #[async_trait]
    pub trait DecideAttendanceCorrectionRepository: Send + Sync {
        async fn find_attendance_correction_request(
            &self,
            request_id: &str,
        ) -> Result<AttendanceCorrectionRecord, AttendanceCorrectionDecisionError>;

        async fn can_manager_approve(
            &self,
            manager_id: &str,
            applicant_id: &str,
        ) -> Result<bool, AttendanceCorrectionDecisionError>;

        async fn approve_attendance_correction_request(
            &self,
            request: ApprovedAttendanceCorrectionRequest,
        ) -> Result<(), AttendanceCorrectionDecisionError>;

        async fn reject_attendance_correction_request(
            &self,
            request: RejectedAttendanceCorrectionRequest,
        ) -> Result<(), AttendanceCorrectionDecisionError>;
    }

    #[async_trait]
    pub trait ListAdminAttendanceCorrectionRequestsRepository: Send + Sync {
        async fn list_subordinate_user_ids(
            &self,
            manager_id: &str,
        ) -> Result<Vec<String>, AdminAttendanceCorrectionReadError>;

        async fn list_admin_attendance_correction_requests(
            &self,
            filters: AdminAttendanceCorrectionListFilters,
        ) -> Result<Vec<AttendanceCorrectionRecord>, AdminAttendanceCorrectionReadError>;

        async fn find_admin_attendance_correction_request(
            &self,
            request_id: &str,
        ) -> Result<AttendanceCorrectionRecord, AdminAttendanceCorrectionReadError>;

        async fn can_manager_view_request(
            &self,
            manager_id: &str,
            applicant_id: &str,
        ) -> Result<bool, AdminAttendanceCorrectionReadError>;
    }

    #[derive(Debug, Clone)]
    pub struct ClockIn<R, H> {
        repository: R,
        holiday_calendar: H,
    }

    #[derive(Debug, Clone)]
    pub struct ClockOut<R, H> {
        repository: R,
        holiday_calendar: H,
    }

    #[derive(Debug, Clone)]
    pub struct StartBreak<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct BreakEnd<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ForceEndBreak<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct GetAttendanceStatus<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct GetBreaksByAttendance<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ListActiveBreaks<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ListAttendancePage<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct UpsertAttendance<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ListUserAttendance<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct GetUserAttendanceSummary<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ExportUserAttendance<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ExportAdminAttendance<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct CreateAttendanceCorrectionRequest<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct UpdateAttendanceCorrectionRequest<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct CancelAttendanceCorrectionRequest<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ApproveAttendanceCorrectionRequest<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct RejectAttendanceCorrectionRequest<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct ListAdminAttendanceCorrectionRequests<R> {
        repository: R,
    }

    #[derive(Debug, Clone)]
    pub struct GetAdminAttendanceCorrectionRequest<R> {
        repository: R,
    }

    impl<R, H> ClockIn<R, H>
    where
        R: AttendanceRepository,
        H: HolidayCalendar,
    {
        pub fn new(repository: R, holiday_calendar: H) -> Self {
            Self {
                repository,
                holiday_calendar,
            }
        }

        pub fn repository(&self) -> &R {
            &self.repository
        }

        pub async fn execute(
            &self,
            command: ClockInCommand,
        ) -> Result<AttendanceDay, ClockInError> {
            match self
                .holiday_calendar
                .decision_for(&command.user_id, command.work_date)
                .await?
            {
                HolidayDecision::WorkingDay => {}
                HolidayDecision::Holiday { reason } => {
                    return Err(ClockInError::Holiday {
                        work_date: command.work_date,
                        reason,
                    })
                }
            }

            let existing = self
                .repository
                .find_by_user_and_date(&command.user_id, command.work_date)
                .await?;

            match existing {
                Some(day) if day.clock_in_time.is_some() => Err(ClockInError::AlreadyClockedIn),
                Some(day) => {
                    self.repository
                        .update_clock_in(ExistingClockIn {
                            attendance_id: day.attendance_id,
                            user_id: command.user_id,
                            work_date: command.work_date,
                            clock_in_time: command.clock_in_time,
                            clock_out_time: day.clock_out_time,
                            recorded_at: command.recorded_at,
                        })
                        .await
                }
                None => {
                    self.repository
                        .create_clock_in(NewClockIn {
                            user_id: command.user_id,
                            work_date: command.work_date,
                            clock_in_time: command.clock_in_time,
                            recorded_at: command.recorded_at,
                        })
                        .await
                }
            }
        }
    }

    impl<R, H> ClockOut<R, H>
    where
        R: ClockOutRepository,
        H: WorkdayCalendar<ClockOutError>,
    {
        pub fn new(repository: R, holiday_calendar: H) -> Self {
            Self {
                repository,
                holiday_calendar,
            }
        }

        pub async fn execute(
            &self,
            command: ClockOutCommand,
        ) -> Result<AttendanceDay, ClockOutError> {
            match self
                .holiday_calendar
                .decision_for(&command.user_id, command.work_date)
                .await?
            {
                HolidayDecision::WorkingDay => {}
                HolidayDecision::Holiday { reason } => {
                    return Err(ClockOutError::Holiday {
                        work_date: command.work_date,
                        reason,
                    })
                }
            }

            let day = self
                .repository
                .find_by_user_and_date(&command.user_id, command.work_date)
                .await?
                .ok_or(ClockOutError::AttendanceNotFound)?;

            let Some(clock_in_time) = day.clock_in_time else {
                return Err(ClockOutError::ClockInRequired);
            };
            if day.clock_out_time.is_some() {
                return Err(ClockOutError::AlreadyClockedOut);
            }
            if self.repository.has_active_break(&day.attendance_id).await? {
                return Err(ClockOutError::ActiveBreakInProgress);
            }

            let break_minutes = self
                .repository
                .total_break_minutes(&day.attendance_id)
                .await?;
            let total_work_hours =
                calculate_total_work_hours(clock_in_time, command.clock_out_time, break_minutes);

            self.repository
                .update_clock_out(ExistingClockOut {
                    attendance_id: day.attendance_id,
                    user_id: command.user_id,
                    work_date: command.work_date,
                    clock_in_time,
                    clock_out_time: command.clock_out_time,
                    total_work_hours,
                    recorded_at: command.recorded_at,
                })
                .await
        }
    }

    impl<R> StartBreak<R>
    where
        R: StartBreakRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: StartBreakCommand,
        ) -> Result<BreakPeriod, StartBreakError> {
            let attendance = self
                .repository
                .find_attendance(&command.attendance_id)
                .await?;

            if attendance.user_id != command.user_id {
                return Err(StartBreakError::Forbidden);
            }
            if attendance.clock_in_time.is_none() || attendance.clock_out_time.is_some() {
                return Err(StartBreakError::ClockInRequired);
            }
            if self
                .repository
                .has_active_break(&command.attendance_id)
                .await?
            {
                return Err(StartBreakError::ActiveBreakInProgress);
            }

            self.repository
                .create_break(NewBreakPeriod {
                    attendance_id: command.attendance_id,
                    break_start_time: command.break_start_time,
                    recorded_at: command.recorded_at,
                })
                .await
        }
    }

    impl<R> BreakEnd<R>
    where
        R: BreakEndRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: BreakEndCommand,
        ) -> Result<BreakPeriod, BreakEndError> {
            let break_period = self.repository.find_break(&command.break_id).await?;
            if break_period.break_end_time.is_some() {
                return Err(BreakEndError::BreakAlreadyEnded);
            }

            let attendance = self
                .repository
                .find_attendance(&break_period.attendance_id)
                .await?;
            if attendance.user_id != command.user_id {
                return Err(BreakEndError::Forbidden);
            }

            let duration_minutes = command
                .break_end_time
                .signed_duration_since(break_period.break_start_time)
                .num_minutes() as i32;
            let ended_break = self
                .repository
                .update_break(EndedBreakPeriod {
                    break_id: break_period.break_id,
                    attendance_id: break_period.attendance_id,
                    break_start_time: break_period.break_start_time,
                    break_end_time: command.break_end_time,
                    duration_minutes,
                    recorded_at: command.recorded_at,
                })
                .await?;

            if attendance.clock_out_time.is_some() {
                self.repository
                    .recalculate_total_hours(&attendance.attendance_id, command.recorded_at)
                    .await?;
            }

            Ok(ended_break)
        }
    }

    impl<R> ForceEndBreak<R>
    where
        R: BreakEndRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: ForceEndBreakCommand,
        ) -> Result<BreakPeriod, BreakEndError> {
            let break_period = self.repository.find_break(&command.break_id).await?;
            if break_period.break_end_time.is_some() {
                return Err(BreakEndError::BreakAlreadyEnded);
            }

            let attendance = self
                .repository
                .find_attendance(&break_period.attendance_id)
                .await?;
            let duration_minutes = command
                .break_end_time
                .signed_duration_since(break_period.break_start_time)
                .num_minutes() as i32;
            let ended_break = self
                .repository
                .update_break(EndedBreakPeriod {
                    break_id: break_period.break_id,
                    attendance_id: break_period.attendance_id,
                    break_start_time: break_period.break_start_time,
                    break_end_time: command.break_end_time,
                    duration_minutes,
                    recorded_at: command.recorded_at,
                })
                .await?;

            if attendance.clock_out_time.is_some() {
                self.repository
                    .recalculate_total_hours(&attendance.attendance_id, command.recorded_at)
                    .await?;
            }

            Ok(ended_break)
        }
    }

    impl<R> GetAttendanceStatus<R>
    where
        R: AttendanceStatusReadRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: AttendanceStatusQuery,
        ) -> Result<AttendanceStatus, AttendanceStatusError> {
            let Some(attendance) = self
                .repository
                .find_by_user_and_date(&query.user_id, query.work_date)
                .await?
            else {
                return Ok(not_started_status(None));
            };

            let attendance_id = Some(attendance.attendance_id.clone());
            let Some(clock_in_time) = attendance.clock_in_time else {
                return Ok(not_started_status(attendance_id));
            };

            if attendance.clock_out_time.is_some() {
                return Ok(AttendanceStatus {
                    status: "clocked_out".to_string(),
                    attendance_id,
                    active_break_id: None,
                    clock_in_time: Some(clock_in_time),
                    clock_out_time: attendance.clock_out_time,
                });
            }

            if let Some(active_break_id) = self
                .repository
                .active_break_id(&attendance.attendance_id)
                .await?
            {
                return Ok(AttendanceStatus {
                    status: "on_break".to_string(),
                    attendance_id,
                    active_break_id: Some(active_break_id),
                    clock_in_time: Some(clock_in_time),
                    clock_out_time: None,
                });
            }

            Ok(AttendanceStatus {
                status: "clocked_in".to_string(),
                attendance_id,
                active_break_id: None,
                clock_in_time: Some(clock_in_time),
                clock_out_time: None,
            })
        }
    }

    impl<R> GetBreaksByAttendance<R>
    where
        R: GetBreaksByAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: GetBreaksByAttendanceQuery,
        ) -> Result<Vec<BreakPeriod>, GetBreaksByAttendanceError> {
            let attendance = self
                .repository
                .find_attendance(&query.attendance_id)
                .await?;
            if attendance.user_id != query.user_id {
                return Err(GetBreaksByAttendanceError::Forbidden);
            }
            self.repository
                .breaks_for_attendance(&query.attendance_id)
                .await
        }
    }

    impl<R> ListActiveBreaks<R>
    where
        R: ListActiveBreaksRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(&self) -> Result<Vec<ActiveBreakSummary>, ListActiveBreaksError> {
            self.repository.list_active_breaks().await
        }
    }

    impl<R> ListAttendancePage<R>
    where
        R: ListAttendancePageRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: ListAttendancePageQuery,
        ) -> Result<AttendancePage, ListAttendancePageError> {
            let total = self.repository.count_attendance().await?;
            let attendances = self
                .repository
                .list_attendance(query.limit, query.offset)
                .await?;
            let attendance_ids = attendances
                .iter()
                .map(|attendance| attendance.attendance_id.clone())
                .collect::<Vec<_>>();
            let break_periods = if attendance_ids.is_empty() {
                Vec::new()
            } else {
                self.repository
                    .breaks_for_attendance_ids(&attendance_ids)
                    .await?
            };
            let mut breaks_by_attendance = group_breaks_by_attendance(break_periods);
            let items = attendances
                .into_iter()
                .map(|attendance| {
                    let break_periods = breaks_by_attendance
                        .remove(&attendance.attendance_id)
                        .unwrap_or_default();
                    AttendancePageItem {
                        attendance,
                        break_periods,
                    }
                })
                .collect();

            Ok(AttendancePage {
                items,
                total,
                limit: query.limit,
                offset: query.offset,
            })
        }
    }

    impl<R> UpsertAttendance<R>
    where
        R: UpsertAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: UpsertAttendanceCommand,
        ) -> Result<AttendancePageItem, UpsertAttendanceError> {
            let breaks = command
                .breaks
                .into_iter()
                .map(replacement_break_from_input)
                .collect::<Vec<_>>();
            let total_break_minutes = breaks
                .iter()
                .filter_map(|break_period| break_period.duration_minutes)
                .map(i64::from)
                .sum::<i64>();
            let total_work_hours = command.clock_out_time.map(|clock_out_time| {
                calculate_total_work_hours(
                    command.clock_in_time,
                    clock_out_time,
                    total_break_minutes,
                )
                .expect("clock-in and clock-out calculate total hours")
            });

            self.repository
                .replace_attendance(AttendanceReplacement {
                    user_id: command.user_id,
                    date: command.date,
                    clock_in_time: command.clock_in_time,
                    clock_out_time: command.clock_out_time,
                    total_work_hours,
                    breaks,
                    recorded_at: command.recorded_at,
                })
                .await
        }
    }

    impl<R> ListUserAttendance<R>
    where
        R: ListUserAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: ListUserAttendanceQuery,
        ) -> Result<Vec<AttendancePageItem>, ListUserAttendanceError> {
            list_user_attendance_items(&self.repository, query).await
        }
    }

    impl<R> GetUserAttendanceSummary<R>
    where
        R: ListUserAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: GetUserAttendanceSummaryQuery,
        ) -> Result<UserAttendanceSummary, ListUserAttendanceError> {
            let items = list_user_attendance_items(
                &self.repository,
                ListUserAttendanceQuery {
                    user_id: query.user_id,
                    from: query.from,
                    to: query.to,
                },
            )
            .await?;
            let total_work_hours = items
                .iter()
                .filter_map(|item| item.attendance.total_work_hours)
                .filter(|hours| *hours > 0.0)
                .sum::<f64>();
            let total_work_days = items
                .iter()
                .filter_map(|item| item.attendance.total_work_hours)
                .filter(|hours| *hours > 0.0)
                .count() as i32;
            let average_daily_hours = if total_work_days > 0 {
                total_work_hours / f64::from(total_work_days)
            } else {
                0.0
            };

            Ok(UserAttendanceSummary {
                month: query.month,
                year: query.year,
                total_work_hours,
                total_work_days,
                average_daily_hours,
            })
        }
    }

    impl<R> ExportUserAttendance<R>
    where
        R: ListUserAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: ExportUserAttendanceQuery,
        ) -> Result<UserAttendanceExport, ListUserAttendanceError> {
            let attendances = self
                .repository
                .list_user_attendance_with_optional_range(&query.user_id, query.from, query.to)
                .await?;
            let items = attendance_items_from_records(&self.repository, attendances).await?;
            let rows = items
                .into_iter()
                .map(|item| UserAttendanceExportRow {
                    username: query.username.clone(),
                    full_name: query.full_name.clone(),
                    date: item.attendance.date,
                    clock_in_time: item.attendance.clock_in_time,
                    clock_out_time: item.attendance.clock_out_time,
                    total_work_hours: item.attendance.total_work_hours,
                    status: item.attendance.status,
                })
                .collect();

            Ok(UserAttendanceExport { rows })
        }
    }

    impl<R> ExportAdminAttendance<R>
    where
        R: ExportAdminAttendanceRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub fn repository(&self) -> &R {
            &self.repository
        }

        pub async fn execute(
            &self,
            query: ExportAdminAttendanceQuery,
        ) -> Result<AdminAttendanceExport, ExportAdminAttendanceError> {
            if !(query.requester_is_manager || query.requester_is_system_admin) {
                return Err(ExportAdminAttendanceError::Forbidden);
            }

            let allowed_user_ids = if query.requester_is_manager && !query.requester_is_system_admin
            {
                Some(
                    self.repository
                        .list_subordinate_user_ids(&query.requester_id)
                        .await?,
                )
            } else {
                None
            };

            let rows = self
                .repository
                .list_admin_attendance_export(AdminAttendanceExportFilters {
                    username: query.username,
                    from: query.from,
                    to: query.to,
                    allowed_user_ids,
                })
                .await?;

            Ok(AdminAttendanceExport {
                rows,
                pii_masked: !query.requester_is_system_admin,
            })
        }
    }

    impl<R> CreateAttendanceCorrectionRequest<R>
    where
        R: CreateAttendanceCorrectionRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: CreateAttendanceCorrectionCommand,
        ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
            validate_correction_reason(&command.reason)?;

            let attendance = self
                .repository
                .find_attendance_by_user_and_date(&command.user_id, command.date)
                .await?
                .ok_or(CreateAttendanceCorrectionError::AttendanceNotFound)?;
            let breaks = self
                .repository
                .breaks_for_attendance(&attendance.attendance_id)
                .await?;

            let original_snapshot = AttendanceCorrectionSnapshot {
                clock_in_time: attendance.clock_in_time,
                clock_out_time: attendance.clock_out_time,
                breaks,
            };
            let proposed_values = build_correction_proposed_snapshot(
                &original_snapshot,
                command.clock_in_time,
                command.clock_out_time,
                command.breaks,
            );

            if original_snapshot == proposed_values {
                return Err(CreateAttendanceCorrectionError::NoChanges);
            }
            validate_correction_snapshot(&proposed_values)?;

            self.repository
                .create_attendance_correction_request(NewAttendanceCorrectionRequest {
                    id: command.request_id,
                    user_id: command.user_id,
                    attendance_id: attendance.attendance_id,
                    date: command.date,
                    reason: command.reason,
                    original_snapshot,
                    proposed_values,
                })
                .await
        }
    }

    impl<R> UpdateAttendanceCorrectionRequest<R>
    where
        R: UpdateAttendanceCorrectionRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: UpdateAttendanceCorrectionCommand,
        ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
            validate_correction_reason(&command.reason)?;

            let current = self
                .repository
                .find_attendance_correction_request_for_user(&command.request_id, &command.user_id)
                .await?;
            if current.status != AttendanceCorrectionRequestStatus::Pending {
                return Err(CreateAttendanceCorrectionError::NotPendingUpdate);
            }

            let proposed_values = build_correction_proposed_snapshot(
                &current.original_snapshot,
                command.clock_in_time,
                command.clock_out_time,
                command.breaks,
            );
            if current.original_snapshot == proposed_values {
                return Err(CreateAttendanceCorrectionError::NoChanges);
            }
            validate_correction_snapshot(&proposed_values)?;

            self.repository
                .update_pending_attendance_correction_request(UpdatedAttendanceCorrectionRequest {
                    id: command.request_id,
                    user_id: command.user_id,
                    reason: command.reason,
                    proposed_values,
                })
                .await
        }
    }

    impl<R> CancelAttendanceCorrectionRequest<R>
    where
        R: CancelAttendanceCorrectionRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: CancelAttendanceCorrectionCommand,
        ) -> Result<(), CreateAttendanceCorrectionError> {
            self.repository
                .cancel_pending_attendance_correction_request(&command.request_id, &command.user_id)
                .await
        }
    }

    impl<R> ApproveAttendanceCorrectionRequest<R>
    where
        R: DecideAttendanceCorrectionRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: ApproveAttendanceCorrectionCommand,
        ) -> Result<(), AttendanceCorrectionDecisionError> {
            validate_decision_comment(&command.comment)?;
            let request = self
                .repository
                .find_attendance_correction_request(&command.request_id)
                .await?;
            ensure_decision_allowed(
                &self.repository,
                &request,
                &command.approver_id,
                command.approver_is_manager,
                command.approver_is_system_admin,
            )
            .await?;

            self.repository
                .approve_attendance_correction_request(ApprovedAttendanceCorrectionRequest {
                    id: command.request_id,
                    attendance_id: request.attendance_id,
                    approver_id: command.approver_id,
                    comment: command.comment,
                    original_snapshot: request.original_snapshot,
                    proposed_values: request.proposed_values,
                })
                .await
        }
    }

    impl<R> RejectAttendanceCorrectionRequest<R>
    where
        R: DecideAttendanceCorrectionRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            command: RejectAttendanceCorrectionCommand,
        ) -> Result<(), AttendanceCorrectionDecisionError> {
            validate_decision_comment(&command.comment)?;
            let request = self
                .repository
                .find_attendance_correction_request(&command.request_id)
                .await?;
            ensure_decision_allowed(
                &self.repository,
                &request,
                &command.approver_id,
                command.approver_is_manager,
                command.approver_is_system_admin,
            )
            .await?;

            self.repository
                .reject_attendance_correction_request(RejectedAttendanceCorrectionRequest {
                    id: command.request_id,
                    approver_id: command.approver_id,
                    comment: command.comment,
                })
                .await
        }
    }

    impl<R> ListAdminAttendanceCorrectionRequests<R>
    where
        R: ListAdminAttendanceCorrectionRequestsRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub fn repository(&self) -> &R {
            &self.repository
        }

        pub async fn execute(
            &self,
            query: ListAdminAttendanceCorrectionRequestsQuery,
        ) -> Result<Vec<AttendanceCorrectionRecord>, AdminAttendanceCorrectionReadError> {
            ensure_admin_correction_read_role(
                query.requester_is_manager,
                query.requester_is_system_admin,
            )?;
            let allowed_user_ids = if query.requester_is_manager && !query.requester_is_system_admin
            {
                Some(
                    self.repository
                        .list_subordinate_user_ids(&query.requester_id)
                        .await?,
                )
            } else {
                None
            };

            self.repository
                .list_admin_attendance_correction_requests(AdminAttendanceCorrectionListFilters {
                    status: query.status,
                    user_id: query.user_id,
                    allowed_user_ids,
                    page: query.page.unwrap_or(1).max(1),
                    per_page: query.per_page.unwrap_or(20).clamp(1, 100),
                })
                .await
        }
    }

    impl<R> GetAdminAttendanceCorrectionRequest<R>
    where
        R: ListAdminAttendanceCorrectionRequestsRepository,
    {
        pub fn new(repository: R) -> Self {
            Self { repository }
        }

        pub async fn execute(
            &self,
            query: GetAdminAttendanceCorrectionRequestQuery,
        ) -> Result<AttendanceCorrectionRecord, AdminAttendanceCorrectionReadError> {
            ensure_admin_correction_read_role(
                query.requester_is_manager,
                query.requester_is_system_admin,
            )?;
            let request = self
                .repository
                .find_admin_attendance_correction_request(&query.request_id)
                .await?;
            if query.requester_is_manager && !query.requester_is_system_admin {
                let can_view = self
                    .repository
                    .can_manager_view_request(&query.requester_id, &request.user_id)
                    .await?;
                if !can_view {
                    return Err(AdminAttendanceCorrectionReadError::ManagerNotAuthorized);
                }
            }
            Ok(request)
        }
    }

    fn not_started_status(attendance_id: Option<String>) -> AttendanceStatus {
        AttendanceStatus {
            status: "not_started".to_string(),
            attendance_id,
            active_break_id: None,
            clock_in_time: None,
            clock_out_time: None,
        }
    }

    fn validate_correction_reason(reason: &str) -> Result<(), CreateAttendanceCorrectionError> {
        if reason.trim().is_empty() {
            return Err(CreateAttendanceCorrectionError::ReasonRequired);
        }
        if reason.chars().count() > 500 {
            return Err(CreateAttendanceCorrectionError::ReasonTooLong);
        }
        Ok(())
    }

    fn validate_decision_comment(comment: &str) -> Result<(), AttendanceCorrectionDecisionError> {
        if comment.trim().is_empty() {
            return Err(AttendanceCorrectionDecisionError::CommentRequired);
        }
        if comment.chars().count() > 500 {
            return Err(AttendanceCorrectionDecisionError::CommentTooLong);
        }
        Ok(())
    }

    async fn ensure_decision_allowed<R>(
        repository: &R,
        request: &AttendanceCorrectionRecord,
        approver_id: &str,
        approver_is_manager: bool,
        approver_is_system_admin: bool,
    ) -> Result<(), AttendanceCorrectionDecisionError>
    where
        R: DecideAttendanceCorrectionRepository,
    {
        if !(approver_is_manager || approver_is_system_admin) {
            return Err(AttendanceCorrectionDecisionError::Forbidden);
        }
        if request.user_id == approver_id {
            return Err(AttendanceCorrectionDecisionError::SelfDecision);
        }
        if approver_is_system_admin {
            return Ok(());
        }
        if repository
            .can_manager_approve(approver_id, &request.user_id)
            .await?
        {
            return Ok(());
        }
        Err(AttendanceCorrectionDecisionError::ManagerNotAuthorized)
    }

    fn ensure_admin_correction_read_role(
        requester_is_manager: bool,
        requester_is_system_admin: bool,
    ) -> Result<(), AdminAttendanceCorrectionReadError> {
        if requester_is_manager || requester_is_system_admin {
            Ok(())
        } else {
            Err(AdminAttendanceCorrectionReadError::Forbidden)
        }
    }

    fn build_correction_proposed_snapshot(
        original: &AttendanceCorrectionSnapshot,
        clock_in_time: Option<NaiveDateTime>,
        clock_out_time: Option<NaiveDateTime>,
        breaks: Option<Vec<AttendanceCorrectionBreak>>,
    ) -> AttendanceCorrectionSnapshot {
        AttendanceCorrectionSnapshot {
            clock_in_time: clock_in_time.or(original.clock_in_time),
            clock_out_time: clock_out_time.or(original.clock_out_time),
            breaks: breaks.unwrap_or_else(|| original.breaks.clone()),
        }
    }

    fn validate_correction_snapshot(
        snapshot: &AttendanceCorrectionSnapshot,
    ) -> Result<(), CreateAttendanceCorrectionError> {
        let Some(clock_in) = snapshot.clock_in_time else {
            return Err(CreateAttendanceCorrectionError::ClockInRequired);
        };
        if let Some(clock_out) = snapshot.clock_out_time {
            if clock_in > clock_out {
                return Err(CreateAttendanceCorrectionError::ClockOutBeforeClockIn);
            }
        }

        for break_period in &snapshot.breaks {
            if let Some(break_end_time) = break_period.break_end_time {
                if break_period.break_start_time > break_end_time {
                    return Err(CreateAttendanceCorrectionError::BreakEndBeforeStart);
                }
                if break_period.break_start_time < clock_in {
                    return Err(CreateAttendanceCorrectionError::BreakStartBeforeClockIn);
                }
                if let Some(clock_out) = snapshot.clock_out_time {
                    if break_end_time > clock_out {
                        return Err(CreateAttendanceCorrectionError::BreakEndAfterClockOut);
                    }
                }
            }
        }

        Ok(())
    }

    fn calculate_total_work_hours(
        clock_in_time: NaiveDateTime,
        clock_out_time: NaiveDateTime,
        break_minutes: i64,
    ) -> Option<f64> {
        let gross_minutes = clock_out_time
            .signed_duration_since(clock_in_time)
            .num_minutes()
            .max(0);
        let net_minutes = gross_minutes - break_minutes.max(0);
        Some(net_minutes.max(0) as f64 / 60.0)
    }

    fn group_breaks_by_attendance(
        break_periods: Vec<BreakPeriod>,
    ) -> HashMap<String, Vec<BreakPeriod>> {
        let mut grouped: HashMap<String, Vec<BreakPeriod>> = HashMap::new();
        for break_period in break_periods {
            grouped
                .entry(break_period.attendance_id.clone())
                .or_default()
                .push(break_period);
        }
        grouped
    }

    fn apply_effective_correction(
        attendance: AttendanceRecord,
        correction: EffectiveAttendanceCorrection,
    ) -> AttendancePageItem {
        let clock_in_time = correction
            .clock_in_time_corrected
            .or(attendance.clock_in_time);
        let clock_out_time = correction
            .clock_out_time_corrected
            .or(attendance.clock_out_time);
        let total_work_hours = match (clock_in_time, clock_out_time) {
            (Some(clock_in), Some(clock_out)) => Some(
                calculate_total_work_hours(
                    clock_in,
                    clock_out,
                    correction
                        .corrected_breaks
                        .iter()
                        .filter_map(|break_period| break_period.duration_minutes)
                        .map(i64::from)
                        .sum(),
                )
                .expect("clock-in and clock-out calculate total hours"),
            ),
            _ => None,
        };

        AttendancePageItem {
            attendance: AttendanceRecord {
                clock_in_time,
                clock_out_time,
                total_work_hours,
                ..attendance
            },
            break_periods: correction.corrected_breaks,
        }
    }

    async fn list_user_attendance_items<R>(
        repository: &R,
        query: ListUserAttendanceQuery,
    ) -> Result<Vec<AttendancePageItem>, ListUserAttendanceError>
    where
        R: ListUserAttendanceRepository,
    {
        let attendances = repository
            .list_user_attendance(&query.user_id, query.from, query.to)
            .await?;
        attendance_items_from_records(repository, attendances).await
    }

    async fn attendance_items_from_records<R>(
        repository: &R,
        attendances: Vec<AttendanceRecord>,
    ) -> Result<Vec<AttendancePageItem>, ListUserAttendanceError>
    where
        R: ListUserAttendanceRepository,
    {
        let attendance_ids = attendances
            .iter()
            .map(|attendance| attendance.attendance_id.clone())
            .collect::<Vec<_>>();
        if attendance_ids.is_empty() {
            return Ok(Vec::new());
        }

        let break_periods = repository
            .breaks_for_attendance_ids(&attendance_ids)
            .await?;
        let corrections = repository
            .effective_corrections_for_attendance_ids(&attendance_ids)
            .await?;
        let mut breaks_by_attendance = group_breaks_by_attendance(break_periods);
        let mut corrections_by_attendance = corrections
            .into_iter()
            .map(|correction| (correction.attendance_id.clone(), correction))
            .collect::<HashMap<_, _>>();

        Ok(attendances
            .into_iter()
            .map(|attendance| {
                let break_periods = breaks_by_attendance
                    .remove(&attendance.attendance_id)
                    .unwrap_or_default();
                if let Some(correction) =
                    corrections_by_attendance.remove(&attendance.attendance_id)
                {
                    return apply_effective_correction(attendance, correction);
                }
                AttendancePageItem {
                    attendance,
                    break_periods,
                }
            })
            .collect())
    }

    fn replacement_break_from_input(input: UpsertBreakInput) -> ReplacementBreak {
        let duration_minutes = input.break_end_time.map(|break_end_time| {
            break_end_time
                .signed_duration_since(input.break_start_time)
                .num_minutes()
                .max(0) as i32
        });
        ReplacementBreak {
            break_start_time: input.break_start_time,
            break_end_time: input.break_end_time,
            duration_minutes,
        }
    }
}
