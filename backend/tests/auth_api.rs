fn is_comment_too_long(comment: &str) -> bool {
    comment.chars().count() > 500
}

#[test]
fn reject_comment_over_500_chars_without_db() {
    let long = "a".repeat(501);
    assert!(is_comment_too_long(&long));
    let short = "a".repeat(500);
    assert!(!is_comment_too_long(&short));
}
