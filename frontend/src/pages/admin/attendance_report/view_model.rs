use timekeeper_contract::admin_attendance_report::AdminAttendanceReportQuery;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttendanceReportFilters {
    pub year: i32,
    pub month: u32,
    pub department_id: Option<String>,
    pub page: i64,
    pub per_page: i64,
}

impl AttendanceReportFilters {
    pub fn to_query(&self) -> AdminAttendanceReportQuery {
        AdminAttendanceReportQuery {
            year: self.year,
            month: self.month,
            department_id: self
                .department_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            page: self.page.max(1),
            per_page: self.per_page.clamp(1, 100),
        }
    }

    pub fn next_page(&self, total: i64) -> Self {
        let has_next = self.page.saturating_mul(self.per_page) < total;
        Self {
            page: if has_next { self.page + 1 } else { self.page },
            ..self.clone()
        }
    }

    pub fn previous_page(&self) -> Self {
        Self {
            page: (self.page - 1).max(1),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_normalize_boundaries_and_pagination() {
        let filters = AttendanceReportFilters {
            year: 2026,
            month: 7,
            department_id: Some("  ".into()),
            page: 0,
            per_page: 999,
        };
        let query = filters.to_query();
        assert_eq!(query.department_id, None);
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 100);
        assert_eq!(filters.next_page(1_000).page, 1);
    }
}
