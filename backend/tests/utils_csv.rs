use timekeeper_backend::utils::csv::append_csv_row;

#[test]
fn csv_escapes_double_quotes() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["say \"hello\"".to_string()]);
    assert_eq!(buffer, "\"say \"\"hello\"\"\"\n");
}

#[test]
fn csv_handles_newlines_in_fields() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["line1\nline2".to_string()]);
    assert_eq!(buffer, "\"line1\nline2\"\n");
}

#[test]
fn csv_handles_comma_in_fields() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["a, b, c".to_string()]);
    assert_eq!(buffer, "\"a, b, c\"\n");
}

#[test]
fn csv_formula_guard_with_equals() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["=cmd|' /C calc'!A0".to_string()]);
    assert_eq!(buffer, "\"'=cmd|' /C calc'!A0\"\n");
}

#[test]
fn csv_formula_guard_with_plus() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["+1+2".to_string()]);
    assert_eq!(buffer, "\"'+1+2\"\n");
}

#[test]
fn csv_formula_guard_with_at() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["@SUM(A1)".to_string()]);
    assert_eq!(buffer, "\"'@SUM(A1)\"\n");
}

#[test]
fn csv_no_guard_for_normal_text() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["normal text".to_string()]);
    assert_eq!(buffer, "\"normal text\"\n");
}

#[test]
fn csv_empty_field() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["".to_string()]);
    assert_eq!(buffer, "\"\"\n");
}

#[test]
fn csv_multiple_fields() {
    let mut buffer = String::new();
    append_csv_row(
        &mut buffer,
        &[
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ],
    );
    assert_eq!(buffer, "\"field1\",\"field2\",\"field3\"\n");
}

#[test]
fn csv_mixed_fields() {
    let mut buffer = String::new();
    append_csv_row(
        &mut buffer,
        &[
            "normal".to_string(),
            "=formula".to_string(),
            "quoted \"text\"".to_string(),
        ],
    );
    assert_eq!(buffer, "\"normal\",\"'=formula\",\"quoted \"\"text\"\"\"\n");
}

#[test]
fn csv_appends_multiple_rows() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["row1".to_string()]);
    append_csv_row(&mut buffer, &["row2".to_string()]);
    assert_eq!(buffer, "\"row1\"\n\"row2\"\n");
}

#[test]
fn csv_handles_unicode() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["日本語テスト".to_string()]);
    assert_eq!(buffer, "\"日本語テスト\"\n");
}

#[test]
fn csv_handles_tabs() {
    let mut buffer = String::new();
    append_csv_row(&mut buffer, &["col1\tcol2".to_string()]);
    assert_eq!(buffer, "\"col1\tcol2\"\n");
}
