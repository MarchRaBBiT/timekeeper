use chrono::{Datelike, Duration, Months, NaiveDate};
use leptos::*;

#[derive(Clone)]
pub struct AttendanceFormState {
    from: RwSignal<String>,
    to: RwSignal<String>,
}

impl AttendanceFormState {
    pub fn new() -> Self {
        Self {
            from: create_rw_signal(String::new()),
            to: create_rw_signal(String::new()),
        }
    }

    pub fn start_date_signal(&self) -> RwSignal<String> {
        self.from
    }

    pub fn end_date_signal(&self) -> RwSignal<String> {
        self.to
    }

    pub fn set_range(&self, from: NaiveDate, to: NaiveDate) {
        self.from.set(from.format("%Y-%m-%d").to_string());
        self.to.set(to.format("%Y-%m-%d").to_string());
    }

    pub fn to_payload(&self) -> Result<(Option<NaiveDate>, Option<NaiveDate>), String> {
        let from_val = self.from.get();
        let to_val = self.to.get();
        let from = parse_date_input(&from_val, "開始日は YYYY-MM-DD 形式で入力してください。")?;
        let to = parse_date_input(&to_val, "終了日は YYYY-MM-DD 形式で入力してください。")?;
        if let (Some(f), Some(t)) = (from, to) {
            if f > t {
                return Err("開始日は終了日以前の日付を指定してください。".into());
            }
        }
        Ok((from, to))
    }
}

fn parse_date_input(value: &str, error_message: &str) -> Result<Option<NaiveDate>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map(Some)
        .map_err(|_| error_message.into())
}

pub fn month_bounds(today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?;
    let next_month = first.checked_add_months(Months::new(1))?;
    let last = next_month.checked_sub_signed(Duration::days(1))?;
    Some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn form_state_to_payload_accepts_blank() {
        let state = AttendanceFormState::new();
        let result = state.to_payload().unwrap();
        assert!(result.0.is_none());
        assert!(result.1.is_none());
    }

    #[wasm_bindgen_test]
    fn form_state_validates_order() {
        let state = AttendanceFormState::new();
        state.start_date_signal().set("2025-02-10".into());
        state.end_date_signal().set("2025-02-01".into());
        assert!(state.to_payload().is_err());
    }

    #[wasm_bindgen_test]
    fn month_bounds_returns_expected_range() {
        let date = NaiveDate::from_ymd_opt(2025, 2, 18).unwrap();
        let (start, end) = month_bounds(date).unwrap();
        assert_eq!(start.day(), 1);
        assert_eq!(end.day(), 28);
    }
}
