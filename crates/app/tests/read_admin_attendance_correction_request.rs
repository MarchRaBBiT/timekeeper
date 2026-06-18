use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AdminAttendanceCorrectionListFilters, AdminAttendanceCorrectionReadError,
    AttendanceCorrectionBreak, AttendanceCorrectionRecord, AttendanceCorrectionRequestStatus,
    AttendanceCorrectionSnapshot, GetAdminAttendanceCorrectionRequest,
    GetAdminAttendanceCorrectionRequestQuery, ListAdminAttendanceCorrectionRequests,
    ListAdminAttendanceCorrectionRequestsQuery, ListAdminAttendanceCorrectionRequestsRepository,
};

#[derive(Default)]
struct RecordingAdminCorrectionReadRepository {
    subordinates: Mutex<Vec<String>>,
    can_manager_view: Mutex<bool>,
    list_filters: Mutex<Option<AdminAttendanceCorrectionListFilters>>,
    list_result: Mutex<Vec<AttendanceCorrectionRecord>>,
    detail_result: Mutex<Option<AttendanceCorrectionRecord>>,
}

#[async_trait::async_trait]
impl ListAdminAttendanceCorrectionRequestsRepository for RecordingAdminCorrectionReadRepository {
    async fn list_subordinate_user_ids(
        &self,
        _manager_id: &str,
    ) -> Result<Vec<String>, AdminAttendanceCorrectionReadError> {
        Ok(self.subordinates.lock().expect("subordinates lock").clone())
    }

    async fn list_admin_attendance_correction_requests(
        &self,
        filters: AdminAttendanceCorrectionListFilters,
    ) -> Result<Vec<AttendanceCorrectionRecord>, AdminAttendanceCorrectionReadError> {
        *self.list_filters.lock().expect("filters lock") = Some(filters);
        Ok(self.list_result.lock().expect("list lock").clone())
    }

    async fn find_admin_attendance_correction_request(
        &self,
        _request_id: &str,
    ) -> Result<AttendanceCorrectionRecord, AdminAttendanceCorrectionReadError> {
        self.detail_result
            .lock()
            .expect("detail lock")
            .clone()
            .ok_or(AdminAttendanceCorrectionReadError::RequestNotFound)
    }

    async fn can_manager_view_request(
        &self,
        _manager_id: &str,
        _applicant_id: &str,
    ) -> Result<bool, AdminAttendanceCorrectionReadError> {
        Ok(*self.can_manager_view.lock().expect("view lock"))
    }
}

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 13).expect("date")
}

fn time(hour: u32) -> NaiveDateTime {
    date().and_hms_opt(hour, 0, 0).expect("time")
}

fn recorded_at() -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(time(9), Utc)
}

fn record_for(user_id: &str) -> AttendanceCorrectionRecord {
    AttendanceCorrectionRecord {
        id: "request-1".to_string(),
        user_id: user_id.to_string(),
        attendance_id: "attendance-1".to_string(),
        date: date(),
        status: AttendanceCorrectionRequestStatus::Pending,
        reason: "reason".to_string(),
        original_snapshot: snapshot(18),
        proposed_values: snapshot(19),
        decision_comment: None,
        approved_by: None,
        approved_at: None,
        rejected_by: None,
        rejected_at: None,
        cancelled_at: None,
        created_at: recorded_at(),
        updated_at: recorded_at(),
    }
}

fn snapshot(clock_out_hour: u32) -> AttendanceCorrectionSnapshot {
    AttendanceCorrectionSnapshot {
        clock_in_time: Some(time(9)),
        clock_out_time: Some(time(clock_out_hour)),
        breaks: vec![AttendanceCorrectionBreak {
            break_start_time: time(12),
            break_end_time: Some(time(13)),
        }],
    }
}

#[tokio::test]
async fn manager_list_is_scoped_to_subordinate_users_and_normalizes_pagination() {
    let repository = RecordingAdminCorrectionReadRepository {
        subordinates: Mutex::new(vec!["employee-1".to_string(), "employee-2".to_string()]),
        can_manager_view: Mutex::new(false),
        list_filters: Mutex::new(None),
        list_result: Mutex::new(vec![record_for("employee-1")]),
        detail_result: Mutex::new(None),
    };
    let use_case = ListAdminAttendanceCorrectionRequests::new(repository);

    let records = use_case
        .execute(ListAdminAttendanceCorrectionRequestsQuery {
            requester_id: "manager-1".to_string(),
            requester_is_manager: true,
            requester_is_system_admin: false,
            status: Some("pending".to_string()),
            user_id: None,
            page: Some(0),
            per_page: Some(500),
        })
        .await
        .expect("list succeeds");

    assert_eq!(records.len(), 1);
    let filters = use_case
        .repository()
        .list_filters
        .lock()
        .expect("filters lock")
        .clone()
        .expect("filters recorded");
    assert_eq!(filters.status.as_deref(), Some("pending"));
    assert_eq!(
        filters.allowed_user_ids,
        Some(vec!["employee-1".to_string(), "employee-2".to_string()])
    );
    assert_eq!(filters.page, 1);
    assert_eq!(filters.per_page, 100);
}

#[tokio::test]
async fn system_admin_list_is_not_scoped_to_subordinate_users() {
    let repository = RecordingAdminCorrectionReadRepository {
        subordinates: Mutex::new(vec!["employee-1".to_string()]),
        can_manager_view: Mutex::new(false),
        list_filters: Mutex::new(None),
        list_result: Mutex::new(vec![record_for("employee-1")]),
        detail_result: Mutex::new(None),
    };
    let use_case = ListAdminAttendanceCorrectionRequests::new(repository);

    use_case
        .execute(ListAdminAttendanceCorrectionRequestsQuery {
            requester_id: "admin-1".to_string(),
            requester_is_manager: false,
            requester_is_system_admin: true,
            status: None,
            user_id: Some("employee-1".to_string()),
            page: None,
            per_page: None,
        })
        .await
        .expect("list succeeds");

    let filters = use_case
        .repository()
        .list_filters
        .lock()
        .expect("filters lock")
        .clone()
        .expect("filters recorded");
    assert_eq!(filters.user_id.as_deref(), Some("employee-1"));
    assert_eq!(filters.allowed_user_ids, None);
    assert_eq!(filters.page, 1);
    assert_eq!(filters.per_page, 20);
}

#[tokio::test]
async fn employee_cannot_list_admin_corrections() {
    let use_case = ListAdminAttendanceCorrectionRequests::new(
        RecordingAdminCorrectionReadRepository::default(),
    );

    let error = use_case
        .execute(ListAdminAttendanceCorrectionRequestsQuery {
            requester_id: "employee-1".to_string(),
            requester_is_manager: false,
            requester_is_system_admin: false,
            status: None,
            user_id: None,
            page: None,
            per_page: None,
        })
        .await
        .expect_err("employee list is rejected");

    assert_eq!(error, AdminAttendanceCorrectionReadError::Forbidden);
}

#[tokio::test]
async fn manager_can_read_authorized_detail() {
    let repository = RecordingAdminCorrectionReadRepository {
        subordinates: Mutex::new(Vec::new()),
        can_manager_view: Mutex::new(true),
        list_filters: Mutex::new(None),
        list_result: Mutex::new(Vec::new()),
        detail_result: Mutex::new(Some(record_for("employee-1"))),
    };
    let use_case = GetAdminAttendanceCorrectionRequest::new(repository);

    let record = use_case
        .execute(GetAdminAttendanceCorrectionRequestQuery {
            requester_id: "manager-1".to_string(),
            requester_is_manager: true,
            requester_is_system_admin: false,
            request_id: "request-1".to_string(),
        })
        .await
        .expect("detail succeeds");

    assert_eq!(record.user_id, "employee-1");
}

#[tokio::test]
async fn manager_cannot_read_unauthorized_detail() {
    let repository = RecordingAdminCorrectionReadRepository {
        subordinates: Mutex::new(Vec::new()),
        can_manager_view: Mutex::new(false),
        list_filters: Mutex::new(None),
        list_result: Mutex::new(Vec::new()),
        detail_result: Mutex::new(Some(record_for("employee-1"))),
    };
    let use_case = GetAdminAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(GetAdminAttendanceCorrectionRequestQuery {
            requester_id: "manager-1".to_string(),
            requester_is_manager: true,
            requester_is_system_admin: false,
            request_id: "request-1".to_string(),
        })
        .await
        .expect_err("unauthorized manager detail is rejected");

    assert_eq!(
        error,
        AdminAttendanceCorrectionReadError::ManagerNotAuthorized
    );
}
