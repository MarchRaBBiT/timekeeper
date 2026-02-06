use crate::api::{ApiError, CreateWeeklyHolidayRequest};
use chrono::NaiveDate;
use leptos::*;

#[derive(Clone, Copy)]
pub struct WeeklyHolidayFormState {
    weekday: RwSignal<String>,
    starts_on: RwSignal<String>,
    ends_on: RwSignal<String>,
}

impl WeeklyHolidayFormState {
    pub fn new(initial_weekday: &str, initial_starts_on: String) -> Self {
        Self {
            weekday: create_rw_signal(initial_weekday.to_string()),
            starts_on: create_rw_signal(initial_starts_on),
            ends_on: create_rw_signal(String::new()),
        }
    }

    pub fn weekday_signal(&self) -> RwSignal<String> {
        self.weekday
    }

    pub fn starts_on_signal(&self) -> RwSignal<String> {
        self.starts_on
    }

    pub fn ends_on_signal(&self) -> RwSignal<String> {
        self.ends_on
    }

    pub fn reset_starts_on(&self, next_start: NaiveDate) {
        self.starts_on
            .set(next_start.format("%Y-%m-%d").to_string());
    }

    pub fn reset_ends_on(&self) {
        self.ends_on.set(String::new());
    }

    pub fn to_payload(&self, min_start: NaiveDate) -> Result<CreateWeeklyHolidayRequest, ApiError> {
        let weekday_value: u8 =
            self.weekday.get().trim().parse::<u8>().map_err(|_| {
                ApiError::validation("曜日は 0 (日) 〜 6 (土) で入力してください。")
            })?;
        if weekday_value >= 7 {
            return Err(ApiError::validation(
                "曜日は 0 (日) 〜 6 (土) で入力してください。",
            ));
        }
        let start_raw = self.starts_on.get();
        if start_raw.trim().is_empty() {
            return Err(ApiError::validation("稼働開始日を入力してください。"));
        }
        let start_date = NaiveDate::parse_from_str(start_raw.trim(), "%Y-%m-%d").map_err(|_| {
            ApiError::validation("稼働開始日は YYYY-MM-DD 形式で入力してください。")
        })?;
        if start_date < min_start {
            return Err(ApiError::validation(&format!(
                "稼働開始日は {} 以降の日付を選択してください。",
                min_start.format("%Y-%m-%d")
            )));
        }

        let ends_on = {
            let raw = self.ends_on.get();
            if raw.trim().is_empty() {
                None
            } else {
                let parsed = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").map_err(|_| {
                    ApiError::validation("稼働終了日は YYYY-MM-DD 形式で入力してください。")
                })?;
                if parsed < start_date {
                    return Err(ApiError::validation(
                        "稼働終了日は開始日以降を指定してください。",
                    ));
                }
                Some(parsed)
            }
        };

        Ok(CreateWeeklyHolidayRequest {
            weekday: weekday_value,
            starts_on: start_date,
            ends_on,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestFilterSnapshot {
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Clone, Copy)]
pub struct RequestFilterState {
    status: RwSignal<String>,
    user_id: RwSignal<String>,
    page: RwSignal<u32>,
    per_page: u32,
}

impl RequestFilterState {
    pub fn new() -> Self {
        Self {
            status: create_rw_signal(String::new()),
            user_id: create_rw_signal(String::new()),
            page: create_rw_signal(1),
            per_page: 20,
        }
    }

    pub fn status_signal(&self) -> RwSignal<String> {
        self.status
    }

    pub fn user_id_signal(&self) -> RwSignal<String> {
        self.user_id
    }

    pub fn snapshot(&self) -> RequestFilterSnapshot {
        let status_value = self.status.get();
        let user_id_value = self.user_id.get();
        RequestFilterSnapshot {
            status: if status_value.is_empty() {
                None
            } else {
                Some(status_value)
            },
            user_id: if user_id_value.is_empty() {
                None
            } else {
                Some(user_id_value)
            },
            page: self.page.get(),
            per_page: self.per_page,
        }
    }

    pub fn reset_page(&self) {
        self.page.set(1);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectRequestFilterSnapshot {
    pub status: Option<String>,
    pub request_type: Option<String>,
    pub user_id: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Clone, Copy)]
pub struct SubjectRequestFilterState {
    status: RwSignal<String>,
    request_type: RwSignal<String>,
    user_id: RwSignal<String>,
    page: RwSignal<u32>,
    per_page: u32,
}

impl SubjectRequestFilterState {
    pub fn new() -> Self {
        Self {
            status: create_rw_signal(String::new()),
            request_type: create_rw_signal(String::new()),
            user_id: create_rw_signal(String::new()),
            page: create_rw_signal(1),
            per_page: 20,
        }
    }

    pub fn status_signal(&self) -> RwSignal<String> {
        self.status
    }

    pub fn request_type_signal(&self) -> RwSignal<String> {
        self.request_type
    }

    pub fn user_id_signal(&self) -> RwSignal<String> {
        self.user_id
    }

    pub fn snapshot(&self) -> SubjectRequestFilterSnapshot {
        let status_value = self.status.get();
        let request_type_value = self.request_type.get();
        let user_id_value = self.user_id.get();
        SubjectRequestFilterSnapshot {
            status: if status_value.is_empty() {
                None
            } else {
                Some(status_value)
            },
            request_type: if request_type_value.is_empty() {
                None
            } else {
                Some(request_type_value)
            },
            user_id: if user_id_value.is_empty() {
                None
            } else {
                Some(user_id_value)
            },
            page: self.page.get(),
            per_page: self.per_page,
        }
    }

    pub fn reset_page(&self) {
        self.page.set(1);
    }
}

pub fn next_allowed_weekly_start(today: NaiveDate, is_system_admin: bool) -> NaiveDate {
    if is_system_admin {
        today
    } else {
        today.succ_opt().unwrap_or(today)
    }
}

pub fn weekday_label(idx: i16) -> &'static str {
    match idx {
        0 => "日",
        1 => "月",
        2 => "火",
        3 => "水",
        4 => "木",
        5 => "金",
        6 => "土",
        _ => "-",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn non_system_admin_starts_tomorrow() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 1, 11).unwrap();
        assert_eq!(next_allowed_weekly_start(today, false), expected);
    }

    #[wasm_bindgen_test]
    fn weekly_form_validates_range() {
        let state = WeeklyHolidayFormState::new("0", "2025-01-15".into());
        state.ends_on_signal().set("2025-01-10".into());
        let min = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert!(state.to_payload(min).is_err());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use chrono::NaiveDate;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn weekly_form_payload_succeeds_with_valid_data() {
        with_runtime(|| {
            let state = WeeklyHolidayFormState::new("1", "2025-01-10".into());
            let min = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
            let payload = state.to_payload(min).unwrap();
            assert_eq!(payload.weekday, 1);
            assert_eq!(
                payload.starts_on,
                NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()
            );
            assert!(payload.ends_on.is_none());
        });
    }

    #[test]
    fn weekly_form_rejects_invalid_weekday() {
        with_runtime(|| {
            let state = WeeklyHolidayFormState::new("9", "2025-01-10".into());
            let min = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
            assert!(state.to_payload(min).is_err());
        });
    }

    #[test]
    fn request_filter_snapshot_includes_values() {
        with_runtime(|| {
            let state = RequestFilterState::new();
            state.status_signal().set("approved".into());
            state.user_id_signal().set("u1".into());
            let snapshot = state.snapshot();
            assert_eq!(snapshot.status.as_deref(), Some("approved"));
            assert_eq!(snapshot.user_id.as_deref(), Some("u1"));
        });
    }

    #[test]
    fn subject_request_filter_snapshot_defaults() {
        with_runtime(|| {
            let state = SubjectRequestFilterState::new();
            let snapshot = state.snapshot();
            assert!(snapshot.status.is_none());
            assert!(snapshot.request_type.is_none());
            assert!(snapshot.user_id.is_none());
            assert_eq!(snapshot.page, 1);
            assert_eq!(snapshot.per_page, 20);
        });
    }

    #[test]
    fn weekday_label_maps_values() {
        assert_eq!(weekday_label(0), "日");
        assert_eq!(weekday_label(6), "土");
        assert_eq!(weekday_label(9), "-");
    }
}
