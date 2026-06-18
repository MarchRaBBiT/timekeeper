use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::attendance::{
    ActiveBreakSummary, ListActiveBreaks, ListActiveBreaksError, ListActiveBreaksRepository,
};

#[derive(Default)]
struct RecordingActiveBreakRepository {
    active_breaks: Mutex<Vec<ActiveBreakSummary>>,
}

#[async_trait::async_trait]
impl ListActiveBreaksRepository for RecordingActiveBreakRepository {
    async fn list_active_breaks(&self) -> Result<Vec<ActiveBreakSummary>, ListActiveBreaksError> {
        Ok(self
            .active_breaks
            .lock()
            .expect("active breaks lock")
            .clone())
    }
}

fn break_start_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 12)
        .expect("date")
        .and_hms_opt(12, 0, 0)
        .expect("break start")
}

fn active_break() -> ActiveBreakSummary {
    ActiveBreakSummary {
        break_id: "break-1".to_string(),
        attendance_id: "attendance-1".to_string(),
        user_id: "user-1".to_string(),
        username: "employee".to_string(),
        full_name: Some("encrypted-name".to_string()),
        break_start_time: break_start_time(),
    }
}

#[tokio::test]
async fn lists_active_breaks_from_repository() {
    let repository = RecordingActiveBreakRepository::default();
    *repository.active_breaks.lock().expect("active breaks lock") = vec![active_break()];
    let use_case = ListActiveBreaks::new(repository);

    let active_breaks = use_case.execute().await.expect("list succeeds");

    assert_eq!(active_breaks.len(), 1);
    assert_eq!(active_breaks[0].break_id, "break-1");
    assert_eq!(
        active_breaks[0].full_name.as_deref(),
        Some("encrypted-name")
    );
}

#[tokio::test]
async fn returns_empty_list_when_no_breaks_are_active() {
    let use_case = ListActiveBreaks::new(RecordingActiveBreakRepository::default());

    let active_breaks = use_case.execute().await.expect("list succeeds");

    assert!(active_breaks.is_empty());
}
