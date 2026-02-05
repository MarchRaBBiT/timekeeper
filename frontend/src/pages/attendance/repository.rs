use crate::api::{ApiClient, ApiError, BreakRecordResponse, HolidayCalendarEntry};

pub trait AttendanceApi {
    async fn get_monthly_holidays(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<HolidayCalendarEntry>, ApiError>;
    async fn get_breaks_by_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakRecordResponse>, ApiError>;
}

impl AttendanceApi for ApiClient {
    async fn get_monthly_holidays(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
        self.get_monthly_holidays(year, month).await
    }

    async fn get_breaks_by_attendance(
        &self,
        attendance_id: &str,
    ) -> Result<Vec<BreakRecordResponse>, ApiError> {
        self.get_breaks_by_attendance(attendance_id).await
    }
}

pub async fn fetch_monthly_holidays(
    api: &impl AttendanceApi,
    year: i32,
    month: u32,
) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
    api.get_monthly_holidays(year, month).await
}

pub async fn fetch_breaks_by_attendance(
    api: &impl AttendanceApi,
    attendance_id: &str,
) -> Result<Vec<BreakRecordResponse>, ApiError> {
    api.get_breaks_by_attendance(attendance_id).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeApi {
        holidays: Vec<HolidayCalendarEntry>,
        breaks: Vec<BreakRecordResponse>,
        holiday_calls: RefCell<Vec<(i32, u32)>>,
        breaks_calls: RefCell<Vec<String>>,
    }

    impl FakeApi {
        fn new() -> Self {
            let holiday = HolidayCalendarEntry {
                date: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                reason: "public holiday".into(),
            };
            let break_record = BreakRecordResponse {
                id: "br-1".into(),
                attendance_id: "att-1".into(),
                break_start_time: chrono::NaiveDate::from_ymd_opt(2025, 1, 2)
                    .unwrap()
                    .and_hms_opt(12, 0, 0)
                    .unwrap(),
                break_end_time: None,
                duration_minutes: None,
            };

            Self {
                holidays: vec![holiday],
                breaks: vec![break_record],
                holiday_calls: RefCell::new(Vec::new()),
                breaks_calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl AttendanceApi for FakeApi {
        async fn get_monthly_holidays(
            &self,
            year: i32,
            month: u32,
        ) -> Result<Vec<HolidayCalendarEntry>, ApiError> {
            self.holiday_calls.borrow_mut().push((year, month));
            Ok(self.holidays.clone())
        }

        async fn get_breaks_by_attendance(
            &self,
            attendance_id: &str,
        ) -> Result<Vec<BreakRecordResponse>, ApiError> {
            self.breaks_calls
                .borrow_mut()
                .push(attendance_id.to_string());
            Ok(self.breaks.clone())
        }
    }

    #[tokio::test]
    async fn attendance_repository_calls_api() {
        let api = FakeApi::new();

        let holidays = fetch_monthly_holidays(&api, 2025, 1).await.unwrap();
        assert_eq!(holidays.len(), 1);
        assert_eq!(holidays[0].reason, "public holiday");
        let breaks = fetch_breaks_by_attendance(&api, "att-1").await.unwrap();
        assert_eq!(breaks.len(), 1);
        assert_eq!(breaks[0].attendance_id, "att-1");

        assert_eq!(api.holiday_calls.borrow().as_slice(), &[(2025, 1)]);
        assert_eq!(api.breaks_calls.borrow().as_slice(), &["att-1".to_string()]);
    }
}
