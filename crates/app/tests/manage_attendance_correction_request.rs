use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendanceCorrectionBreak, AttendanceCorrectionRecord, AttendanceCorrectionRequestStatus,
    AttendanceCorrectionSnapshot, CancelAttendanceCorrectionCommand,
    CancelAttendanceCorrectionRepository, CancelAttendanceCorrectionRequest,
    CreateAttendanceCorrectionError, UpdateAttendanceCorrectionCommand,
    UpdateAttendanceCorrectionRepository, UpdateAttendanceCorrectionRequest,
    UpdatedAttendanceCorrectionRequest,
};

#[derive(Default)]
struct RecordingCorrectionRepository {
    current: Mutex<Option<AttendanceCorrectionRecord>>,
    updated: Mutex<Option<UpdatedAttendanceCorrectionRequest>>,
    cancelled: Mutex<Option<(String, String)>>,
}

#[async_trait::async_trait]
impl UpdateAttendanceCorrectionRepository for RecordingCorrectionRepository {
    async fn find_attendance_correction_request_for_user(
        &self,
        _request_id: &str,
        _user_id: &str,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        self.current
            .lock()
            .expect("current lock")
            .clone()
            .ok_or(CreateAttendanceCorrectionError::RequestNotFound)
    }

    async fn update_pending_attendance_correction_request(
        &self,
        request: UpdatedAttendanceCorrectionRequest,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        *self.updated.lock().expect("updated lock") = Some(request.clone());
        let mut record = self
            .current
            .lock()
            .expect("current lock")
            .clone()
            .expect("current request");
        record.reason = request.reason;
        record.proposed_values = request.proposed_values;
        Ok(record)
    }
}

#[async_trait::async_trait]
impl CancelAttendanceCorrectionRepository for RecordingCorrectionRepository {
    async fn cancel_pending_attendance_correction_request(
        &self,
        request_id: &str,
        user_id: &str,
    ) -> Result<(), CreateAttendanceCorrectionError> {
        *self.cancelled.lock().expect("cancelled lock") =
            Some((request_id.to_string(), user_id.to_string()));
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

fn request_with_status(status: AttendanceCorrectionRequestStatus) -> AttendanceCorrectionRecord {
    AttendanceCorrectionRecord {
        id: "request-1".to_string(),
        user_id: "user-1".to_string(),
        attendance_id: "attendance-1".to_string(),
        date: date(),
        status,
        reason: "old reason".to_string(),
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
async fn updates_pending_request_with_new_proposed_values() {
    let repository = RecordingCorrectionRepository {
        current: Mutex::new(Some(request_with_status(
            AttendanceCorrectionRequestStatus::Pending,
        ))),
        updated: Mutex::new(None),
        cancelled: Mutex::new(None),
    };
    let use_case = UpdateAttendanceCorrectionRequest::new(repository);

    let record = use_case
        .execute(UpdateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            clock_in_time: None,
            clock_out_time: Some(time(20)),
            breaks: None,
            reason: "new reason".to_string(),
        })
        .await
        .expect("update succeeds");

    assert_eq!(record.reason, "new reason");
    assert_eq!(record.proposed_values.clock_out_time, Some(time(20)));
}

#[tokio::test]
async fn rejects_update_for_non_pending_request() {
    let repository = RecordingCorrectionRepository {
        current: Mutex::new(Some(request_with_status(
            AttendanceCorrectionRequestStatus::Approved,
        ))),
        updated: Mutex::new(None),
        cancelled: Mutex::new(None),
    };
    let use_case = UpdateAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(UpdateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            clock_in_time: None,
            clock_out_time: Some(time(20)),
            breaks: None,
            reason: "new reason".to_string(),
        })
        .await
        .expect_err("non-pending update is rejected");

    assert_eq!(error, CreateAttendanceCorrectionError::NotPendingUpdate);
}

#[tokio::test]
async fn rejects_update_without_any_changed_field() {
    let repository = RecordingCorrectionRepository {
        current: Mutex::new(Some(request_with_status(
            AttendanceCorrectionRequestStatus::Pending,
        ))),
        updated: Mutex::new(None),
        cancelled: Mutex::new(None),
    };
    let use_case = UpdateAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(UpdateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            clock_in_time: None,
            clock_out_time: Some(time(18)),
            breaks: None,
            reason: "new reason".to_string(),
        })
        .await
        .expect_err("unchanged update is rejected");

    assert_eq!(error, CreateAttendanceCorrectionError::NoChanges);
}

#[tokio::test]
async fn cancels_pending_request_by_id_and_user() {
    let repository = RecordingCorrectionRepository::default();
    let use_case = CancelAttendanceCorrectionRequest::new(repository);

    use_case
        .execute(CancelAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
        })
        .await
        .expect("cancel succeeds");
}
