pub fn request_status_label(value: &str) -> String {
    match value {
        "pending" => "承認待ち".to_string(),
        "approved" => "承認済み".to_string(),
        "rejected" => "却下".to_string(),
        "cancelled" => "取消".to_string(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::request_status_label;

    #[test]
    fn request_status_label_maps_known_values() {
        assert_eq!(request_status_label("pending"), "承認待ち".to_string());
        assert_eq!(request_status_label("approved"), "承認済み".to_string());
        assert_eq!(request_status_label("rejected"), "却下".to_string());
        assert_eq!(request_status_label("cancelled"), "取消".to_string());
    }

    #[test]
    fn request_status_label_handles_unknown_values() {
        assert_eq!(request_status_label("unexpected"), "unexpected".to_string());
    }
}
