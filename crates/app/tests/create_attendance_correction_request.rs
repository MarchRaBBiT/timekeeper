use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use timekeeper_app::attendance::{
    AttendanceCorrectionBreak, AttendanceCorrectionRecord, AttendanceCorrectionRequestStatus,
    AttendanceCorrectionSnapshot, CorrectionAttendance, CreateAttendanceCorrectionCommand,
    CreateAttendanceCorrectionError, CreateAttendanceCorrectionRepository,
    CreateAttendanceCorrectionRequest, NewAttendanceCorrectionRequest,
};

#[derive(Default)]
struct RecordingCorrectionRepository {
    attendance: Mutex<Option<CorrectionAttendance>>,
    breaks: Mutex<Vec<AttendanceCorrectionBreak>>,
    created: Mutex<Option<NewAttendanceCorrectionRequest>>,
}

#[async_trait::async_trait]
impl CreateAttendanceCorrectionRepository for RecordingCorrectionRepository {
    async fn find_attendance_by_user_and_date(
        &self,
        _user_id: &str,
        _date: NaiveDate,
    ) -> Result<Option<CorrectionAttendance>, CreateAttendanceCorrectionError> {
        Ok(self.attendance.lock().expect("attendance lock").clone())
    }

    async fn breaks_for_attendance(
        &self,
        _attendance_id: &str,
    ) -> Result<Vec<AttendanceCorrectionBreak>, CreateAttendanceCorrectionError> {
        Ok(self.breaks.lock().expect("breaks lock").clone())
    }

    async fn create_attendance_correction_request(
        &self,
        request: NewAttendanceCorrectionRequest,
    ) -> Result<AttendanceCorrectionRecord, CreateAttendanceCorrectionError> {
        *self.created.lock().expect("created lock") = Some(request.clone());
        Ok(AttendanceCorrectionRecord {
            id: request.id,
            user_id: request.user_id,
            attendance_id: request.attendance_id,
            date: request.date,
            status: AttendanceCorrectionRequestStatus::Pending,
            reason: request.reason,
            original_snapshot: request.original_snapshot,
            proposed_values: request.proposed_values,
            decision_comment: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            created_at: recorded_at(),
            updated_at: recorded_at(),
        })
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

fn repository_with_attendance() -> RecordingCorrectionRepository {
    RecordingCorrectionRepository {
        attendance: Mutex::new(Some(CorrectionAttendance {
            attendance_id: "attendance-1".to_string(),
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: Some(time(9)),
            clock_out_time: Some(time(18)),
        })),
        breaks: Mutex::new(vec![AttendanceCorrectionBreak {
            break_start_time: time(12),
            break_end_time: Some(time(13)),
        }]),
        created: Mutex::new(None),
    }
}

#[tokio::test]
async fn creates_request_with_original_snapshot_and_proposed_overrides() {
    let repository = repository_with_attendance();
    let use_case = CreateAttendanceCorrectionRequest::new(repository);

    let record = use_case
        .execute(CreateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: None,
            clock_out_time: Some(time(19)),
            breaks: None,
            reason: "forgot to clock out".to_string(),
        })
        .await
        .expect("create succeeds");

    assert_eq!(record.id, "request-1");
    assert_eq!(record.status, AttendanceCorrectionRequestStatus::Pending);
    assert_eq!(
        record.original_snapshot,
        AttendanceCorrectionSnapshot {
            clock_in_time: Some(time(9)),
            clock_out_time: Some(time(18)),
            breaks: vec![AttendanceCorrectionBreak {
                break_start_time: time(12),
                break_end_time: Some(time(13)),
            }],
        }
    );
    assert_eq!(record.proposed_values.clock_out_time, Some(time(19)));
    assert_eq!(
        record.proposed_values.breaks,
        record.original_snapshot.breaks
    );
}

#[tokio::test]
async fn rejects_request_without_any_changed_field() {
    let repository = repository_with_attendance();
    let use_case = CreateAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(CreateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: None,
            clock_out_time: None,
            breaks: None,
            reason: "same values".to_string(),
        })
        .await
        .expect_err("unchanged request is rejected");

    assert_eq!(error, CreateAttendanceCorrectionError::NoChanges);
}

#[tokio::test]
async fn rejects_break_that_ends_after_clock_out() {
    let repository = repository_with_attendance();
    let use_case = CreateAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(CreateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: None,
            clock_out_time: Some(time(18)),
            breaks: Some(vec![AttendanceCorrectionBreak {
                break_start_time: time(17),
                break_end_time: Some(time(19)),
            }]),
            reason: "bad break".to_string(),
        })
        .await
        .expect_err("invalid break is rejected");

    assert_eq!(
        error,
        CreateAttendanceCorrectionError::BreakEndAfterClockOut
    );
}

#[tokio::test]
async fn rejects_blank_reason_before_repository_write() {
    let repository = repository_with_attendance();
    let use_case = CreateAttendanceCorrectionRequest::new(repository);

    let error = use_case
        .execute(CreateAttendanceCorrectionCommand {
            request_id: "request-1".to_string(),
            user_id: "user-1".to_string(),
            date: date(),
            clock_in_time: None,
            clock_out_time: Some(time(19)),
            breaks: None,
            reason: "   ".to_string(),
        })
        .await
        .expect_err("blank reason is rejected");

    assert_eq!(error, CreateAttendanceCorrectionError::ReasonRequired);
}
