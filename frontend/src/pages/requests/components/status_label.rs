pub fn request_status_key(value: &str) -> Option<&'static str> {
    match value {
        "pending" => Some("pages.requests.status.pending"),
        "approved" => Some("pages.requests.status.approved"),
        "rejected" => Some("pages.requests.status.rejected"),
        "cancelled" => Some("pages.requests.status.cancelled"),
        _ => None,
    }
}

pub fn request_status_label(value: &str) -> String {
    request_status_key(value)
        .map(|key| rust_i18n::t!(key).to_string())
        .unwrap_or_else(|| value.to_string())
}

pub fn request_kind_key(value: crate::pages::requests::types::RequestKind) -> &'static str {
    match value {
        crate::pages::requests::types::RequestKind::Leave => "pages.requests.kind.leave",
        crate::pages::requests::types::RequestKind::Overtime => "pages.requests.kind.overtime",
        crate::pages::requests::types::RequestKind::AttendanceCorrection => {
            "pages.requests.kind.attendance_correction"
        }
    }
}

pub fn request_kind_label(value: crate::pages::requests::types::RequestKind) -> String {
    rust_i18n::t!(request_kind_key(value)).to_string()
}

pub fn request_kind_title_key(value: crate::pages::requests::types::RequestKind) -> &'static str {
    match value {
        crate::pages::requests::types::RequestKind::Leave => "pages.requests.kind.leave_request",
        crate::pages::requests::types::RequestKind::Overtime => {
            "pages.requests.kind.overtime_request"
        }
        crate::pages::requests::types::RequestKind::AttendanceCorrection => {
            "pages.requests.kind.attendance_correction_request"
        }
    }
}

pub fn request_kind_title(value: crate::pages::requests::types::RequestKind) -> String {
    rust_i18n::t!(request_kind_title_key(value)).to_string()
}

#[cfg(test)]
mod tests {
    use super::{request_kind_label, request_kind_title, request_status_label};
    use crate::pages::requests::types::RequestKind;
    use crate::test_support::helpers::set_test_locale;

    #[test]
    fn request_status_label_maps_known_values() {
        let _locale = set_test_locale("ja");
        assert_eq!(request_status_label("pending"), "承認待ち".to_string());
        assert_eq!(request_status_label("approved"), "承認済み".to_string());
        assert_eq!(request_status_label("rejected"), "却下".to_string());
        assert_eq!(request_status_label("cancelled"), "取消".to_string());
    }

    #[test]
    fn request_status_label_handles_unknown_values() {
        assert_eq!(request_status_label("unexpected"), "unexpected".to_string());
    }

    #[test]
    fn request_kind_label_maps_known_values() {
        let _locale = set_test_locale("en");
        assert_eq!(request_kind_label(RequestKind::Leave), "Leave");
        assert_eq!(request_kind_label(RequestKind::Overtime), "Overtime");
        assert_eq!(
            request_kind_title(RequestKind::AttendanceCorrection),
            "Attendance Correction Request"
        );
    }
}
