use jira_mcp::error::AppError;

#[test]
fn validation_errors_map_to_validation_message() {
    let mapped = AppError::validation("bad input").to_mcp();

    assert_eq!(mapped.message, "validation_error");
}

#[test]
fn unsafe_operation_errors_map_to_expected_message() {
    let mapped = AppError::unsafe_operation("dangerous action").to_mcp();

    assert_eq!(mapped.message, "unsafe_operation");
}
