use timekeeper_backend::handlers::admin::AdminRequestListPageInfo;

#[test]
fn pagination_info_without_db() {
    let page_info = AdminRequestListPageInfo {
        page: 2,
        per_page: 10,
    };
    assert_eq!(page_info.page, 2);
    assert_eq!(page_info.per_page, 10);
}
