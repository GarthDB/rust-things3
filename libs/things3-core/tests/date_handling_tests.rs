//! Comprehensive tests for date handling and validation
//!
//! This test suite validates the date conversion, validation, and edge case handling
//! throughout the Things 3 integration.

use chrono::{Datelike, NaiveDate};
use things3_core::database::{
    add_days, format_date_for_display, is_date_in_future, is_date_in_past,
    is_valid_things_timestamp, parse_date_from_string, safe_naive_date_to_things_timestamp,
    safe_things_date_to_naive_date, validate_date_range, DateConversionError, DateValidationError,
};
use things3_core::ThingsError;

#[cfg(feature = "test-utils")]
use things3_core::test_utils::{create_test_database_and_connect, TaskRequestBuilder};

// ===========================
// Conversion Tests (8 tests)
// ===========================

#[test]
fn test_things_date_conversion_basic() {
    // Test conversion of a normal date
    let timestamp = 86400 * 365; // Approximately 1 year after epoch
    let date = safe_things_date_to_naive_date(timestamp).unwrap();

    // Should be roughly early 2002
    assert_eq!(date.year(), 2002);
}

#[test]
fn test_things_date_conversion_epoch() {
    // Epoch (2001-01-01) should convert correctly
    let date = safe_things_date_to_naive_date(0).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2001, 1, 1).unwrap());
}

#[test]
fn test_things_date_conversion_negative() {
    // Small negative timestamps (within reasonable range) should fail
    let result = safe_things_date_to_naive_date(-86400 * 400); // ~1 year+ before epoch
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DateConversionError::InvalidTimestamp(_))
    ));
}

#[test]
fn test_things_date_conversion_zero() {
    // Zero timestamp should convert to epoch
    let date = safe_things_date_to_naive_date(0).unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2001, 1, 1).unwrap());
}

#[test]
fn test_things_date_conversion_far_future() {
    // Year 2150 (way beyond reasonable range)
    let far_future_timestamp = 86400 * 365 * 150; // ~150 years after epoch
    let result = safe_things_date_to_naive_date(far_future_timestamp);
    assert!(result.is_err());
}

#[test]
fn test_things_date_conversion_overflow() {
    // Test with value that would cause overflow
    let result = safe_things_date_to_naive_date(i64::MAX);
    assert!(result.is_err());
}

#[test]
fn test_naive_date_to_things_before_epoch() {
    // Date before 2001-01-01 should fail
    let date = NaiveDate::from_ymd_opt(2000, 12, 31).unwrap();
    let result = safe_naive_date_to_things_timestamp(date);
    assert!(result.is_err());
    assert!(matches!(result, Err(DateConversionError::BeforeEpoch(_))));
}

#[test]
fn test_round_trip_conversion() {
    // Test multiple dates round-trip correctly
    let test_dates = vec![
        NaiveDate::from_ymd_opt(2001, 1, 1).unwrap(),   // Epoch
        NaiveDate::from_ymd_opt(2010, 6, 15).unwrap(),  // Mid-year
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), // Recent date
        NaiveDate::from_ymd_opt(2050, 1, 1).unwrap(),   // Far future (but valid)
    ];

    for original_date in test_dates {
        let timestamp = safe_naive_date_to_things_timestamp(original_date).unwrap();
        let converted_date = safe_things_date_to_naive_date(timestamp).unwrap();
        assert_eq!(
            original_date, converted_date,
            "Round-trip conversion failed for {original_date}"
        );
    }
}

// ================================
// Validation Tests (8 tests)
// ================================

#[test]
fn test_validate_deadline_after_start() {
    // Valid case: deadline after start date
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    assert!(validate_date_range(Some(start), Some(deadline)).is_ok());
}

#[test]
fn test_validate_deadline_before_start() {
    // Invalid case: deadline before start date
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let result = validate_date_range(Some(start), Some(deadline));
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DateValidationError::DeadlineBeforeStartDate { .. })
    ));
}

#[test]
fn test_validate_same_date() {
    // Edge case: start date and deadline are the same
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    assert!(validate_date_range(Some(date), Some(date)).is_ok());
}

#[test]
fn test_validate_only_start_date() {
    // Only start date provided (no deadline)
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    assert!(validate_date_range(Some(start), None).is_ok());
}

#[test]
fn test_validate_only_deadline() {
    // Only deadline provided (no start date)
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    assert!(validate_date_range(None, Some(deadline)).is_ok());
}

#[test]
fn test_validate_no_dates() {
    // Both dates are None
    assert!(validate_date_range(None, None).is_ok());
}

#[test]
#[cfg(feature = "test-utils")]
fn test_validate_task_dates_via_builder() {
    // Test validation through TaskRequestBuilder
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

    let request = TaskRequestBuilder::new()
        .title("Test task")
        .start_date(start)
        .deadline(deadline)
        .build();

    // Should be able to validate
    assert!(validate_date_range(request.start_date, request.deadline).is_ok());
}

#[test]
#[cfg(feature = "test-utils")]
fn test_validate_task_dates_invalid_via_builder() {
    // Test validation catches invalid dates
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let request = TaskRequestBuilder::new()
        .title("Test task")
        .start_date(start)
        .deadline(deadline)
        .build();

    // Should fail validation
    assert!(validate_date_range(request.start_date, request.deadline).is_err());
}

// ================================
// Edge Case Tests (6 tests)
// ================================

#[test]
fn test_handle_null_dates_gracefully() {
    // Test that None dates are handled gracefully
    assert!(validate_date_range(None, None).is_ok());

    let formatted = format_date_for_display(None);
    assert_eq!(formatted, "None");
}

#[test]
fn test_handle_invalid_timestamp_gracefully() {
    // Test that invalid timestamps produce clear errors
    let invalid_timestamps = vec![i64::MAX, i64::MIN, -1000000000, 10000000000];

    for timestamp in invalid_timestamps {
        let result = safe_things_date_to_naive_date(timestamp);
        assert!(result.is_err(), "Timestamp {timestamp} should be invalid");
    }
}

#[test]
fn test_date_formatting_consistency() {
    // Test that date formatting is consistent
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let formatted = format_date_for_display(Some(date));
    assert_eq!(formatted, "2024-06-15");

    // Test None case
    let formatted_none = format_date_for_display(None);
    assert_eq!(formatted_none, "None");
}

#[test]
fn test_date_parsing_multiple_formats() {
    // Test ISO 8601 format
    let date = parse_date_from_string("2024-06-15").unwrap();
    assert_eq!(date, NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());

    // Test invalid formats
    assert!(parse_date_from_string("invalid").is_err());
    assert!(parse_date_from_string("2024-13-01").is_err()); // Invalid month
    assert!(parse_date_from_string("2024-06-32").is_err()); // Invalid day
    assert!(parse_date_from_string("15/06/2024").is_err()); // Wrong format
}

#[test]
fn test_date_arithmetic_overflow() {
    // Test that date arithmetic handles overflow
    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Add reasonable days
    let new_date = add_days(date, 10).unwrap();
    assert_eq!(new_date, NaiveDate::from_ymd_opt(2024, 1, 11).unwrap());

    // Subtract days
    let earlier_date = add_days(date, -10).unwrap();
    assert_eq!(earlier_date, NaiveDate::from_ymd_opt(2023, 12, 22).unwrap());

    // Try to add a large number of days that will overflow NaiveDate's valid range
    // (NaiveDate supports years from -262144 to 262143, so large numbers should overflow)
    let far_date = NaiveDate::from_ymd_opt(260000, 1, 1).unwrap();
    let result = add_days(far_date, 365 * 3000); // Add 3000 years to a far-future date
    assert!(result.is_err());
}

#[test]
fn test_date_comparison_edge_cases() {
    let today = chrono::Utc::now().date_naive();

    // Past date
    let yesterday = today - chrono::Duration::days(1);
    assert!(is_date_in_past(yesterday));
    assert!(!is_date_in_future(yesterday));

    // Future date
    let tomorrow = today + chrono::Duration::days(1);
    assert!(is_date_in_future(tomorrow));
    assert!(!is_date_in_past(tomorrow));

    // Today should be neither past nor future
    assert!(!is_date_in_past(today));
    assert!(!is_date_in_future(today));
}

// ================================
// Integration Tests (4 tests)
// ================================

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn test_create_task_with_invalid_dates() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create task with deadline before start date
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let request = TaskRequestBuilder::new()
        .title("Invalid dates task")
        .start_date(start)
        .deadline(deadline)
        .build();

    // This should fail validation
    let result = db.create_task(request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::DateValidation(_))));
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn test_update_task_deadline_before_start() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a valid task first
    let request = TaskRequestBuilder::new().title("Test task").build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Try to update with invalid dates
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let update_request = things3_core::models::UpdateTaskRequest {
        uuid: task_uuid,
        title: None,
        notes: None,
        start_date: Some(start),
        deadline: Some(deadline),
        status: None,
        tags: None,
        project_uuid: None,
        area_uuid: None,
    };

    // This should fail validation
    let result = db.update_task(update_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::DateValidation(_))));
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn test_create_task_with_valid_dates() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create task with valid dates
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

    let request = TaskRequestBuilder::new()
        .title("Valid dates task")
        .start_date(start)
        .deadline(deadline)
        .build();

    // This should succeed
    let result = db.create_task(request).await;
    assert!(
        result.is_ok(),
        "Task creation should succeed with valid dates"
    );

    let task_uuid = result.unwrap();
    let task = db
        .get_task_by_uuid(&task_uuid)
        .await
        .unwrap()
        .expect("Task should exist");

    assert_eq!(task.start_date, Some(start));
    assert_eq!(task.deadline, Some(deadline));
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn test_update_task_dates_successfully() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a task without dates
    let request = TaskRequestBuilder::new().title("Test task").build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Update with valid dates
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

    let update_request = things3_core::models::UpdateTaskRequest {
        uuid: task_uuid,
        title: None,
        notes: None,
        start_date: Some(start),
        deadline: Some(deadline),
        status: None,
        tags: None,
        project_uuid: None,
        area_uuid: None,
    };

    // This should succeed
    let result = db.update_task(update_request).await;
    assert!(
        result.is_ok(),
        "Task update should succeed with valid dates"
    );

    // Verify dates were updated
    let task = db
        .get_task_by_uuid(&task_uuid)
        .await
        .unwrap()
        .expect("Task should exist");
    assert_eq!(task.start_date, Some(start));
    assert_eq!(task.deadline, Some(deadline));
}

// ================================
// Timestamp Validation Tests
// ================================

#[test]
fn test_is_valid_things_timestamp_valid() {
    // Valid timestamps
    assert!(is_valid_things_timestamp(0)); // Epoch
    assert!(is_valid_things_timestamp(86400)); // 1 day after
    assert!(is_valid_things_timestamp(86400 * 365)); // 1 year after
    assert!(is_valid_things_timestamp(86400 * 365 * 20)); // 20 years after
}

#[test]
fn test_is_valid_things_timestamp_invalid() {
    // Invalid - too far in past
    assert!(!is_valid_things_timestamp(-86400 * 365 * 5)); // 5 years before epoch

    // Invalid - too far in future
    assert!(!is_valid_things_timestamp(86400 * 365 * 150)); // 150 years after epoch
}

// ================================
// Error Message Tests
// ================================

#[test]
fn test_date_conversion_error_messages() {
    let date_before_epoch = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let result = safe_naive_date_to_things_timestamp(date_before_epoch);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("before"));
    assert!(error.to_string().contains("2001"));

    let date_too_far = NaiveDate::from_ymd_opt(2150, 1, 1).unwrap();
    let result = safe_naive_date_to_things_timestamp(date_too_far);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("future"));
}

#[test]
fn test_date_validation_error_messages() {
    let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let result = validate_date_range(Some(start), Some(deadline));
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Deadline"));
    assert!(error.to_string().contains("cannot be before"));
    assert!(error.to_string().contains("start date"));
}
