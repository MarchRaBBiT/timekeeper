use timekeeper_app::payroll_export::render_csv;
use timekeeper_contract::payroll_export::PayrollSnapshot;

fn row(id: &str) -> PayrollSnapshot {
    PayrollSnapshot {
        employee_id: id.into(),
        year: 2026,
        month: 7,
        revision: 1,
        worked_minutes: 9600,
        scheduled_minutes: 9000,
        statutory_within_minutes: 300,
        statutory_excess_minutes: 300,
        legal_holiday_minutes: 480,
        night_minutes: 60,
        absent_days: 1,
        paid_leave_days: 2,
        paid_leave_half_days: 1,
        paid_leave_minutes: 90,
        holiday_work_minutes: 480,
        substitute_holiday_days: 1,
        compensatory_leave_minutes: 240,
    }
}

#[test]
fn csv_is_bom_crlf_fixed_order_sorted_and_escaped() {
    let csv = render_csv(&[row("z"), row("a,\"quoted\"")]);
    assert!(csv.starts_with("\u{feff}employee_id,year,month,revision,worked_minutes"));
    assert!(!csv.replace("\r\n", "").contains('\n'));
    assert!(csv.find("\"a,\"\"quoted\"\"\"").expect("a row") < csv.find("\"z\"").expect("z row"));
    assert!(csv.ends_with("\r\n"));
}
