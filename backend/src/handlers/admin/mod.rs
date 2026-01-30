pub mod attendance;
pub mod audit_logs;
pub mod common;
pub mod export;
pub mod holidays;
pub mod requests;
pub mod sessions;
pub mod users;

pub use attendance::*;
pub use audit_logs::*;
// common is internal helpers, usually not re-exported fully, but let's see if docs.rs needs anything from it.
// docs.rs needs structs. The structs are in their respective modules now.
// We should re-export everything from the new modules to maintain backward compatibility for `use crate::handlers::admin::*;` if used.
pub use export::*;
pub use holidays::*;
pub use requests::*;
pub use sessions::*;
pub use users::*;

pub mod subject_requests;
pub use subject_requests::*;

#[cfg(any(test, feature = "test-utils"))]
#[allow(dead_code)]
mod query_validation {
    use super::*;
    use crate::models::holiday::AdminHolidayKind;
    use axum::http::StatusCode;
    use axum::Json;
    use chrono::NaiveDate;
    use serde_json::Value;

    const DEFAULT_PAGE: i64 = 1;
    const DEFAULT_PER_PAGE: i64 = 25;
    const MAX_PER_PAGE: i64 = 100;
    const MAX_PAGE: i64 = 1_000;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AdminHolidayQueryParams {
        pub page: i64,
        pub per_page: i64,
        pub kind: Option<AdminHolidayKind>,
        pub from: Option<NaiveDate>,
        pub to: Option<NaiveDate>,
    }

    pub fn validate_admin_holiday_query(
        q: AdminHolidayListQuery,
    ) -> Result<AdminHolidayQueryParams, (StatusCode, Json<Value>)> {
        let page = q.page.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
        let per_page = q
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);

        let kind = parse_type_filter(q.r#type.as_deref()).map_err(bad_request)?;
        let from = super::common::parse_optional_date(q.from.as_deref()).map_err(bad_request)?;
        let to = super::common::parse_optional_date(q.to.as_deref()).map_err(bad_request)?;

        if let (Some(from), Some(to)) = (from, to) {
            if from > to {
                return Err(bad_request("`from` must be before or equal to `to`"));
            }
        }

        Ok(AdminHolidayQueryParams {
            page,
            per_page,
            kind,
            from,
            to,
        })
    }

    fn parse_type_filter(raw: Option<&str>) -> Result<Option<AdminHolidayKind>, &'static str> {
        match raw {
            Some(value) if value.eq_ignore_ascii_case("all") => Ok(None),
            Some("public") => Ok(Some(AdminHolidayKind::Public)),
            Some("weekly") => Ok(Some(AdminHolidayKind::Weekly)),
            Some("exception") => Ok(Some(AdminHolidayKind::Exception)),
            Some(_) => Err("`type` must be one of public, weekly, exception, all"),
            None => Ok(None),
        }
    }

    fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": message })),
        )
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[allow(unused_imports)]
pub use query_validation::{
    validate_admin_holiday_query as test_validate_admin_holiday_query,
    AdminHolidayQueryParams as TestAdminHolidayQueryParams,
};
