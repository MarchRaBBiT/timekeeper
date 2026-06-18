use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    ApproveAttendanceCorrectionCommand, ApprovedAttendanceCorrectionRequest,
    AttendanceCorrectionBreak, AttendanceCorrectionDecisionError, AttendanceCorrectionRecord,
    AttendanceCorrectionRequestStatus, AttendanceCorrectionSnapshot,
    DecideAttendanceCorrectionRepository, RejectAttendanceCorrectionCommand,
    RejectAttendanceCorrectionRequest, RejectedAttendanceCorrectionRequest,
};

#[derive(Default)]
struct RecordingDecisionRepository {
    request: Mutex<Option<AttendanceCorrectionRecord>>,
    manager_can_approve: Mutex<bool>,
    approved: Mutex<Option<ApprovedAttendanceCorrectionRequest>>,
    rejected: Mutex<Option<RejectedAttendanceCorrectionRequest>>,
}

#[async_trait::async_trait]
impl DecideAttendanceCorrectionRepository for RecordingDecisionRepository {
    async fn find_attendance_correction_request(
        &self,
        _request_id: &str,
    ) -> Result<AttendanceCorrectionRecord, AttendanceCorrectionDecisionError> {
        self.request
            .lock()
            .expect("request lock")
            .clone()
            .ok_or(AttendanceCorrectionDecisionError::RequestNotFound)
    }

    async fn can_manager_approve(
        &self,
        _manager_id: &str,
        _applicant_id: &str,
    ) -> Result<bool, AttendanceCorrectionDecisionError> {
        Ok(*self.manager_can_approve.lock().expect("approval lock"))
    }

    async fn approve_attendance_correction_request(
        &self,
        request: ApprovedAttendanceCorrectionRequest,
    ) -> Result<(), AttendanceCorrectionDecisionError> {
        *self.approved.lock().expect("approved lock") = Some(request);
        Ok(())
    }

    async fn reject_attendance_correction_request(
        &self,
        request: RejectedAttendanceCorrectionRequest,
    ) -> Result<(), AttendanceCorrectionDecisionError> {
        *self.rejected.lock().expect("rejected lock") = Some(request);
        Ok(())
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

fn pending_request_for(user_id: &str) -> AttendanceCorrectionRecord {
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

#[tokio::test]
async fn manager_can_approve_subordinate_request() {
    let repository = RecordingDecisionRepository {
        request: Mutex::new(Some(pending_request_for("employee-1"))),
        manager_can_approve: Mutex::new(true),
        approved: Mutex::new(None),
        rejected: Mutex::new(None),
    };
    let use_case = timekeeper_app::attendance::ApproveAttendanceCorrectionRequest::new(repository);

    use_case
        .execute(ApproveAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            approver_id: "manager-1".to_string(),
            approver_is_manager: true,
            approver_is_system_admin: false,
            comment: "approved".to_string(),
        })
        .await
        .expect("approval succeeds");
}

#[tokio::test]
async fn rejects_self_approval() {
    let repository = RecordingDecisionRepository {
        request: Mutex::new(Some(pending_request_for("manager-1"))),
        manager_can_approve: Mutex::new(true),
        approved: Mutex::new(None),
        rejected: Mutex::new(None),
    };
    let use_case = timekeeper_app::attendance::ApproveAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(ApproveAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            approver_id: "manager-1".to_string(),
            approver_is_manager: true,
            approver_is_system_admin: false,
            comment: "approved".to_string(),
        })
        .await
        .expect_err("self approval is rejected");

    assert_eq!(error, AttendanceCorrectionDecisionError::SelfDecision);
}

#[tokio::test]
async fn rejects_manager_without_subordinate_authorization() {
    let repository = RecordingDecisionRepository {
        request: Mutex::new(Some(pending_request_for("employee-1"))),
        manager_can_approve: Mutex::new(false),
        approved: Mutex::new(None),
        rejected: Mutex::new(None),
    };
    let use_case = timekeeper_app::attendance::ApproveAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(ApproveAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            approver_id: "manager-1".to_string(),
            approver_is_manager: true,
            approver_is_system_admin: false,
            comment: "approved".to_string(),
        })
        .await
        .expect_err("unauthorized manager is rejected");

    assert_eq!(
        error,
        AttendanceCorrectionDecisionError::ManagerNotAuthorized
    );
}

#[tokio::test]
async fn rejects_blank_decision_comment() {
    let repository = RecordingDecisionRepository {
        request: Mutex::new(Some(pending_request_for("employee-1"))),
        manager_can_approve: Mutex::new(true),
        approved: Mutex::new(None),
        rejected: Mutex::new(None),
    };
    let use_case = timekeeper_app::attendance::ApproveAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(ApproveAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            approver_id: "manager-1".to_string(),
            approver_is_manager: true,
            approver_is_system_admin: false,
            comment: "   ".to_string(),
        })
        .await
        .expect_err("blank comment is rejected");

    assert_eq!(error, AttendanceCorrectionDecisionError::CommentRequired);
}

#[tokio::test]
async fn system_admin_can_reject_without_subordinate_check() {
    let repository = RecordingDecisionRepository {
        request: Mutex::new(Some(pending_request_for("employee-1"))),
        manager_can_approve: Mutex::new(false),
        approved: Mutex::new(None),
        rejected: Mutex::new(None),
    };
    let use_case = RejectAttendanceCorrectionRequest::new(repository);

    use_case
        .execute(RejectAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            approver_id: "admin-1".to_string(),
            approver_is_manager: false,
            approver_is_system_admin: true,
            comment: "rejected".to_string(),
        })
        .await
        .expect("system admin reject succeeds");
}
