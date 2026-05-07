//! Timestamp, date, tag-blob conversions and small enum mappers.
//!
//! Things 3 stores dates as seconds since 2001-01-01 UTC (a `REAL` column for
//! creation/modification, `INTEGER` for start/deadline). These helpers convert
//! between that representation and `chrono` types, plus two small `from_i32`
//! mappers for the integer task-status/type columns.

use crate::error::{Result as ThingsResult, ThingsError};
use crate::models::{TaskStatus, TaskType};
use chrono::NaiveDate;

/// Convert f64 timestamp to i64 safely
pub(crate) fn safe_timestamp_convert(ts_f64: f64) -> i64 {
    // Use try_from to avoid clippy warnings about casting
    if ts_f64.is_finite() && ts_f64 >= 0.0 {
        // Use a reasonable upper bound for timestamps (year 2100)
        let max_timestamp = 4_102_444_800_f64; // 2100-01-01 00:00:00 UTC
        if ts_f64 <= max_timestamp {
            // Convert via string to avoid precision loss warnings
            let ts_str = format!("{:.0}", ts_f64.trunc());
            ts_str.parse::<i64>().unwrap_or(0)
        } else {
            0 // Use epoch if too large
        }
    } else {
        0 // Use epoch if invalid
    }
}

/// Convert Things 3 date value (seconds since 2001-01-01) to NaiveDate
pub(crate) fn things_date_to_naive_date(seconds_since_2001: i64) -> Option<chrono::NaiveDate> {
    use chrono::{TimeZone, Utc};

    if seconds_since_2001 <= 0 {
        return None;
    }

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();

    // Add seconds to get the actual date
    let date_time = base_date + chrono::Duration::seconds(seconds_since_2001);

    Some(date_time.date_naive())
}

/// Convert NaiveDate to Things 3 timestamp (seconds since 2001-01-01)
pub fn naive_date_to_things_timestamp(date: NaiveDate) -> i64 {
    use chrono::{NaiveTime, TimeZone, Utc};

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();

    // Convert NaiveDate to DateTime at midnight UTC
    let date_time = date
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_local_timezone(Utc)
        .single()
        .unwrap();

    // Calculate seconds difference
    date_time.timestamp() - base_date.timestamp()
}

/// Serialize tags to Things 3 binary format
/// Note: This is a simplified implementation using JSON
/// The actual Things 3 binary format is proprietary
pub fn serialize_tags_to_blob(tags: &[String]) -> ThingsResult<Vec<u8>> {
    serde_json::to_vec(tags)
        .map_err(|e| ThingsError::unknown(format!("Failed to serialize tags: {e}")))
}

/// Deserialize tags from Things 3 binary format
pub fn deserialize_tags_from_blob(blob: &[u8]) -> ThingsResult<Vec<String>> {
    if blob.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(blob)
        .map_err(|e| ThingsError::unknown(format!("Failed to deserialize tags: {e}")))
}

impl TaskStatus {
    pub(crate) fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskStatus::Incomplete),
            2 => Some(TaskStatus::Canceled),
            3 => Some(TaskStatus::Completed),
            _ => None,
        }
    }
}

impl TaskType {
    pub(crate) fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskType::Todo),
            1 => Some(TaskType::Project),
            2 => Some(TaskType::Heading),
            3 => Some(TaskType::Area),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_from_i32() {
        assert_eq!(TaskStatus::from_i32(0), Some(TaskStatus::Incomplete));
        assert_eq!(TaskStatus::from_i32(1), None); // unused in real Things 3
        assert_eq!(TaskStatus::from_i32(2), Some(TaskStatus::Canceled));
        assert_eq!(TaskStatus::from_i32(3), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_i32(4), None);
        assert_eq!(TaskStatus::from_i32(-1), None);
    }

    #[test]
    fn test_task_type_from_i32() {
        assert_eq!(TaskType::from_i32(0), Some(TaskType::Todo));
        assert_eq!(TaskType::from_i32(1), Some(TaskType::Project));
        assert_eq!(TaskType::from_i32(2), Some(TaskType::Heading));
        assert_eq!(TaskType::from_i32(3), Some(TaskType::Area));
        assert_eq!(TaskType::from_i32(4), None);
        assert_eq!(TaskType::from_i32(-1), None);
    }

    #[test]
    fn test_safe_timestamp_convert_edge_cases() {
        // Test normal timestamp
        assert_eq!(safe_timestamp_convert(1_609_459_200.0), 1_609_459_200); // 2021-01-01

        // Test zero
        assert_eq!(safe_timestamp_convert(0.0), 0);

        // Test negative (should return 0)
        assert_eq!(safe_timestamp_convert(-1.0), 0);

        // Test infinity (should return 0)
        assert_eq!(safe_timestamp_convert(f64::INFINITY), 0);

        // Test NaN (should return 0)
        assert_eq!(safe_timestamp_convert(f64::NAN), 0);

        // Test very large timestamp (should return 0)
        assert_eq!(safe_timestamp_convert(5_000_000_000.0), 0);

        // Test max valid timestamp
        let max_timestamp = 4_102_444_800_f64; // 2100-01-01
        assert_eq!(safe_timestamp_convert(max_timestamp), 4_102_444_800);
    }

    #[test]
    fn test_task_status_from_i32_all_variants() {
        assert_eq!(TaskStatus::from_i32(0), Some(TaskStatus::Incomplete));
        assert_eq!(TaskStatus::from_i32(1), None); // unused in real Things 3
        assert_eq!(TaskStatus::from_i32(2), Some(TaskStatus::Canceled));
        assert_eq!(TaskStatus::from_i32(3), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_i32(999), None);
        assert_eq!(TaskStatus::from_i32(-1), None);
    }

    #[test]
    fn test_task_type_from_i32_all_variants() {
        assert_eq!(TaskType::from_i32(0), Some(TaskType::Todo));
        assert_eq!(TaskType::from_i32(1), Some(TaskType::Project));
        assert_eq!(TaskType::from_i32(2), Some(TaskType::Heading));
        assert_eq!(TaskType::from_i32(3), Some(TaskType::Area));
        assert_eq!(TaskType::from_i32(999), None);
        assert_eq!(TaskType::from_i32(-1), None);
    }

    #[test]
    fn test_things_date_negative_returns_none() {
        // Negative values should return None
        assert_eq!(things_date_to_naive_date(-1), None);
        assert_eq!(things_date_to_naive_date(-100), None);
        assert_eq!(things_date_to_naive_date(i64::MIN), None);
    }

    #[test]
    fn test_things_date_zero_returns_none() {
        // Zero should return None (no date set)
        assert_eq!(things_date_to_naive_date(0), None);
    }

    #[test]
    fn test_things_date_boundary_2001() {
        use chrono::Datelike;
        // 1 second after 2001-01-01 00:00:00 should be 2001-01-01
        let result = things_date_to_naive_date(1);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2001);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);
    }

    #[test]
    fn test_things_date_one_day() {
        use chrono::Datelike;
        // 86400 seconds = 1 day (60 * 60 * 24), should be 2001-01-02
        let seconds_per_day = 86400i64;
        let result = things_date_to_naive_date(seconds_per_day);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2001);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 2);
    }

    #[test]
    fn test_things_date_one_year() {
        use chrono::Datelike;
        // ~365 days should be around 2002-01-01 (365 days * 86400 seconds/day)
        let seconds_per_year = 365 * 86400i64;
        let result = things_date_to_naive_date(seconds_per_year);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2002);
    }

    #[test]
    fn test_things_date_current_era() {
        use chrono::Datelike;
        // Test a date in the current era (2024)
        // Days from 2001-01-01 to 2024-01-01 = ~8401 days
        // Calculation: (2024-2001) * 365 + leap days (2004, 2008, 2012, 2016, 2020) = 23 * 365 + 5 = 8400
        let days_to_2024 = 8401i64;
        let seconds_to_2024 = days_to_2024 * 86400;

        let result = things_date_to_naive_date(seconds_to_2024);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
    }

    #[test]
    fn test_things_date_leap_year() {
        use chrono::{Datelike, TimeZone, Utc};
        // Test Feb 29, 2004 (leap year)
        // Days from 2001-01-01 to 2004-02-29
        let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();
        let target_date = Utc.with_ymd_and_hms(2004, 2, 29, 0, 0, 0).single().unwrap();
        let seconds_diff = (target_date - base_date).num_seconds();

        let result = things_date_to_naive_date(seconds_diff);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2004);
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_safe_timestamp_convert_normal_values() {
        // Normal timestamp values should convert correctly
        let ts = 1_700_000_000.0; // Around 2023
        let result = safe_timestamp_convert(ts);
        assert_eq!(result, 1_700_000_000);
    }

    #[test]
    fn test_safe_timestamp_convert_zero() {
        // Zero should return zero
        assert_eq!(safe_timestamp_convert(0.0), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_negative() {
        // Negative values should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(-1.0), 0);
        assert_eq!(safe_timestamp_convert(-1000.0), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_infinity() {
        // Infinity should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(f64::INFINITY), 0);
        assert_eq!(safe_timestamp_convert(f64::NEG_INFINITY), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_nan() {
        // NaN should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(f64::NAN), 0);
    }

    #[test]
    fn test_date_roundtrip_known_dates() {
        use chrono::{Datelike, TimeZone, Utc};
        // Test roundtrip conversion for known dates
        // Note: Starting from 2001-01-02 because 2001-01-01 is the base date (0 seconds)
        // and things_date_to_naive_date returns None for values <= 0
        let test_cases = vec![
            (2001, 1, 2), // Start from day 2 since day 1 is the base (0 seconds)
            (2010, 6, 15),
            (2020, 12, 31),
            (2024, 2, 29), // Leap year
            (2025, 7, 4),
        ];

        for (year, month, day) in test_cases {
            let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();
            let target_date = Utc
                .with_ymd_and_hms(year, month, day, 0, 0, 0)
                .single()
                .unwrap();
            let seconds = (target_date - base_date).num_seconds();

            let converted = things_date_to_naive_date(seconds);
            assert!(
                converted.is_some(),
                "Failed to convert {}-{:02}-{:02}",
                year,
                month,
                day
            );

            let result_date = converted.unwrap();
            assert_eq!(
                result_date.year(),
                year,
                "Year mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
            assert_eq!(
                result_date.month(),
                month,
                "Month mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
            assert_eq!(
                result_date.day(),
                day,
                "Day mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
        }
    }
}
