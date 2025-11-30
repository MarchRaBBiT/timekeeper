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

#[cfg(test)]
mod tests {
    use super::*;

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
}
