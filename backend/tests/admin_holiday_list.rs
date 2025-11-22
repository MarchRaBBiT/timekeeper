use chrono::{DateTime, NaiveDate, Utc};
use timekeeper_backend::handlers::admin::{
    AdminHolidayKind, AdminHolidayListItem, AdminHolidayListQuery, AdminHolidayListResponse,
};

fn build_item(id: &str, kind: AdminHolidayKind, date: NaiveDate) -> AdminHolidayListItem {
    AdminHolidayListItem {
        id: id.to_string(),
        kind,
        applies_from: date,
        applies_to: None,
        date: Some(date),
        weekday: None,
        starts_on: None,
        ends_on: None,
        name: Some(id.to_string()),
        description: None,
        user_id: None,
        reason: None,
        created_by: None,
        created_at: DateTime::<Utc>::from(Utc::now()),
        is_override: None,
    }
}

fn mock_list_holidays(
    items: Vec<AdminHolidayListItem>,
    query: AdminHolidayListQuery,
) -> AdminHolidayListResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(25).clamp(1, 100);

    let type_filter = query.r#type.as_deref().map(|s| s.to_ascii_lowercase());
    let from = query
        .from
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to = query
        .to
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let mut filtered: Vec<AdminHolidayListItem> = items
        .into_iter()
        .filter(|item| match type_filter.as_deref() {
            Some("public") => item.kind == AdminHolidayKind::Public,
            Some("weekly") => item.kind == AdminHolidayKind::Weekly,
            Some("exception") => item.kind == AdminHolidayKind::Exception,
            _ => true,
        })
        .filter(|item| from.map(|f| item.applies_from >= f).unwrap_or(true))
        .filter(|item| to.map(|t| item.applies_from <= t).unwrap_or(true))
        .collect();

    filtered.sort_by_key(|item| item.applies_from);

    let total = filtered.len() as i64;
    let offset = ((page - 1) * per_page) as usize;
    let items = filtered
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect();

    AdminHolidayListResponse {
        page,
        per_page,
        total,
        items,
    }
}

#[test]
fn admin_holiday_list_filters_by_type() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let items = vec![
        build_item("public", AdminHolidayKind::Public, date),
        build_item("weekly", AdminHolidayKind::Weekly, date),
        build_item("exception", AdminHolidayKind::Exception, date),
    ];

    let query = AdminHolidayListQuery {
        page: Some(1),
        per_page: Some(20),
        r#type: Some("public".into()),
        from: None,
        to: None,
    };

    let response = mock_list_holidays(items, query);

    assert_eq!(response.total, 1);
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].kind, AdminHolidayKind::Public);
    assert_eq!(response.items[0].date, Some(date));
}

#[test]
fn admin_holiday_list_supports_pagination_and_range() {
    let dates = [
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 5).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
    ];
    let items: Vec<_> = dates
        .iter()
        .enumerate()
        .map(|(idx, d)| build_item(&format!("Holiday {}", idx), AdminHolidayKind::Public, *d))
        .collect();

    let query = AdminHolidayListQuery {
        page: Some(2),
        per_page: Some(1),
        r#type: Some("all".into()),
        from: Some("2025-03-01".into()),
        to: Some("2025-03-06".into()),
    };

    let response = mock_list_holidays(items, query);

    assert_eq!(response.total, 2);
    assert_eq!(response.page, 2);
    assert_eq!(response.per_page, 1);
    assert_eq!(response.items.len(), 1);
    assert!(response
        .items
        .iter()
        .all(|item| item.kind == AdminHolidayKind::Public));
    assert!(response.items.iter().all(|item| item.applies_from
        >= NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()
        && item.applies_from <= NaiveDate::from_ymd_opt(2025, 3, 6).unwrap()));
}
