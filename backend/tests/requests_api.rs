use timekeeper_backend::handlers::admin::{AdminRequestListPageInfo, paginate_requests};
use timekeeper_backend::handlers::admin::RequestListQuery;

#[test]
fn pagination_info_without_db() {
    let page_info = AdminRequestListPageInfo {
        page: 2,
        per_page: 10,
    };
    assert_eq!(page_info.page, 2);
    assert_eq!(page_info.per_page, 10);

    let query = RequestListQuery {
        status: None,
        r#type: None,
        user_id: None,
        from: None,
        to: None,
        page: Some(3),
        per_page: Some(15),
    };
    let (page, per_page, offset) = paginate_requests(&query).expect("pagination ok");
    assert_eq!(page, 3);
    assert_eq!(per_page, 15);
    assert_eq!(offset, 30);
}
