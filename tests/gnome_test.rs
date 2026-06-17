use memguard::desktop::gnome::parse_eval_result;

#[test]
fn test_parse_eval_result_success() {
    assert_eq!(parse_eval_result("true, 12345").unwrap(), 12345);
}

#[test]
fn test_parse_eval_result_failure() {
    assert!(parse_eval_result("false, error").is_err());
}

#[test]
fn test_parse_eval_result_malformed() {
    assert!(parse_eval_result("12345").is_err());
}
