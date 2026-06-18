fn needs_formula_guard(value: &str) -> bool {
    value
        .chars()
        .find(|c| !c.is_whitespace())
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
}

fn escape_cell(value: &str) -> String {
    let mut sanitized = value.replace('"', "\"\"");
    if needs_formula_guard(&sanitized) {
        sanitized.insert(0, '\'');
    }
    format!("\"{}\"", sanitized)
}

pub fn append_csv_row(buffer: &mut String, fields: &[String]) {
    for (idx, field) in fields.iter().enumerate() {
        if idx > 0 {
            buffer.push(',');
        }
        buffer.push_str(&escape_cell(field));
    }
    buffer.push('\n');
}

pub struct AdminAttendanceExportCsvRow {
    pub username: String,
    pub full_name: String,
    pub date: String,
    pub clock_in: String,
    pub clock_out: String,
    pub total_hours: String,
    pub status: String,
}

pub fn render_admin_attendance_export_csv(rows: &[AdminAttendanceExportCsvRow]) -> String {
    let mut csv_data = String::new();
    append_csv_row(&mut csv_data, &attendance_export_headers());

    for row in rows {
        append_csv_row(
            &mut csv_data,
            &[
                row.username.clone(),
                row.full_name.clone(),
                row.date.clone(),
                row.clock_in.clone(),
                row.clock_out.clone(),
                row.total_hours.clone(),
                row.status.clone(),
            ],
        );
    }

    csv_data
}

pub fn render_user_attendance_export_csv(
    rows: &[timekeeper_app::attendance::UserAttendanceExportRow],
) -> String {
    let mut csv_data = String::new();
    append_csv_row(&mut csv_data, &attendance_export_headers());

    for row in rows {
        append_csv_row(
            &mut csv_data,
            &[
                row.username.clone(),
                row.full_name.clone(),
                row.date.format("%Y-%m-%d").to_string(),
                row.clock_in_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                row.clock_out_time
                    .map(|time| time.format("%H:%M:%S").to_string())
                    .unwrap_or_default(),
                row.total_work_hours
                    .map(|hours| format!("{hours:.2}"))
                    .unwrap_or_else(|| "0.00".to_string()),
                row.status.clone(),
            ],
        );
    }

    csv_data
}

fn attendance_export_headers() -> Vec<String> {
    vec![
        "Username".to_string(),
        "Full Name".to_string(),
        "Date".to_string(),
        "Clock In".to_string(),
        "Clock Out".to_string(),
        "Total Hours".to_string(),
        "Status".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use timekeeper_app::attendance::UserAttendanceExportRow;

    #[test]
    fn escapes_formulas_and_quotes() {
        let mut buffer = String::new();
        append_csv_row(
            &mut buffer,
            &["=SUM(A1)".to_string(), "\"quoted\"".to_string()],
        );

        assert_eq!(buffer, "\"'=SUM(A1)\",\"\"\"quoted\"\"\"\n");
    }

    #[test]
    fn guards_formula_after_leading_whitespace() {
        let mut buffer = String::new();
        append_csv_row(&mut buffer, &["  -1".to_string()]);

        assert_eq!(buffer, "\"'  -1\"\n");
    }

    #[test]
    fn renders_user_attendance_export_csv() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 13).expect("date");
        let row = UserAttendanceExportRow {
            username: "employee".to_string(),
            full_name: "Test User".to_string(),
            date,
            clock_in_time: Some(date.and_hms_opt(9, 0, 0).expect("clock in")),
            clock_out_time: Some(date.and_hms_opt(18, 0, 0).expect("clock out")),
            total_work_hours: Some(8.0),
            status: "present".to_string(),
        };

        let csv = render_user_attendance_export_csv(&[row]);

        assert!(csv.contains("\"Username\",\"Full Name\",\"Date\""));
        assert!(csv.contains("\"employee\",\"Test User\",\"2026-06-13\""));
        assert!(csv.contains("\"09:00:00\",\"18:00:00\",\"8.00\",\"present\""));
    }

    #[test]
    fn renders_admin_attendance_export_csv() {
        let row = AdminAttendanceExportCsvRow {
            username: "employee".to_string(),
            full_name: "Masked".to_string(),
            date: "2026-06-13".to_string(),
            clock_in: "09:00:00".to_string(),
            clock_out: "18:00:00".to_string(),
            total_hours: "8.00".to_string(),
            status: "present".to_string(),
        };

        let csv = render_admin_attendance_export_csv(&[row]);

        assert!(csv.contains("\"Username\",\"Full Name\",\"Date\""));
        assert!(csv.contains("\"employee\",\"Masked\",\"2026-06-13\""));
        assert!(csv.contains("\"09:00:00\",\"18:00:00\",\"8.00\",\"present\""));
    }
}
