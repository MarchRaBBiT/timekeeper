use timekeeper_contract::payroll_export::PayrollSnapshot;

pub const CSV_HEADER: &str = "employee_id,year,month,revision,worked_minutes,scheduled_minutes,\
statutory_within_minutes,statutory_excess_minutes,legal_holiday_minutes,night_minutes,\
absent_days,paid_leave_days,paid_leave_half_days,\
paid_leave_minutes,holiday_work_minutes,substitute_holiday_days,compensatory_leave_minutes";

pub fn render_csv(rows: &[PayrollSnapshot]) -> String {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| left.employee_id.cmp(&right.employee_id));
    let mut csv = format!("\u{feff}{CSV_HEADER}\r\n");
    for row in sorted {
        let employee_id = row.employee_id.replace('"', "\"\"");
        csv.push_str(&format!(
            "\"{employee_id}\",{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\r\n",
            row.year,
            row.month,
            row.revision,
            row.worked_minutes,
            row.scheduled_minutes,
            row.statutory_within_minutes,
            row.statutory_excess_minutes,
            row.legal_holiday_minutes,
            row.night_minutes,
            row.absent_days,
            row.paid_leave_days,
            row.paid_leave_half_days,
            row.paid_leave_minutes,
            row.holiday_work_minutes,
            row.substitute_holiday_days,
            row.compensatory_leave_minutes
        ));
    }
    csv
}
