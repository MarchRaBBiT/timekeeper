use timekeeper_contract::payroll_export::{PayrollExportFailedUser, PayrollExportResponse};

#[test]
fn bulk_response_keeps_per_user_failures_with_csv() {
    let response = PayrollExportResponse {
        year: 2026,
        month: 7,
        exported: vec![],
        failed: vec![PayrollExportFailedUser {
            user_id: "u-1".into(),
            code: "monthly_not_closed".into(),
        }],
        csv: "\u{feff}employee_id\r\n".into(),
    };
    let json = serde_json::to_value(response).expect("serialize");
    assert_eq!(json["failed"][0]["code"], "monthly_not_closed");
    assert!(json["csv"].as_str().expect("csv").starts_with('\u{feff}'));
}
