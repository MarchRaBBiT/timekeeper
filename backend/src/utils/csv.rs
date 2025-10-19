fn needs_formula_guard(value: &str) -> bool {
    matches!(value.chars().next(), Some('=' | '+' | '-' | '@'))
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
