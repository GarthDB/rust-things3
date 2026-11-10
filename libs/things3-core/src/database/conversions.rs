//! Timestamp, date, tag-blob conversions and small enum mappers.
#![allow(deprecated)]
//!
//! Things 3 stores creation/modification as seconds (a `REAL` column) and
//! start/deadline as packed ints (`year<<16 | month<<12 | day<<7`). These
//! helpers convert between those and `chrono` types, plus two small `from_i32`
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

/// Decode a Things 3 packed date (`year<<16 | month<<12 | day<<7`) to a NaiveDate.
///
/// Returns `None` for NULL-like/invalid values and for the year-4001 sentinel
/// Things uses for "no date" on repeating templates.
pub(crate) fn things_date_to_naive_date(packed: i64) -> Option<chrono::NaiveDate> {
    if packed <= 0 {
        return None;
    }
    let year = i32::try_from(packed >> 16).ok()?;
    if year >= 4000 {
        return None;
    }
    let month = u32::try_from((packed >> 12) & 0xF).ok()?;
    let day = u32::try_from((packed >> 7) & 0x1F).ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Encode a NaiveDate as a Things 3 packed date (`year<<16 | month<<12 | day<<7`)
pub fn naive_date_to_things_timestamp(date: NaiveDate) -> i64 {
    use chrono::Datelike;

    (i64::from(date.year()) << 16) | (i64::from(date.month()) << 12) | (i64::from(date.day()) << 7)
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
    fn test_things_date_invalid_returns_none() {
        assert_eq!(things_date_to_naive_date(-1), None);
        assert_eq!(things_date_to_naive_date(i64::MIN), None);
        assert_eq!(things_date_to_naive_date(0), None);
        // month 0 / day 0
        assert_eq!(things_date_to_naive_date(2026 << 16), None);
        // year-4001 sentinel seen on repeating templates
        assert_eq!(things_date_to_naive_date(262_213_760), None);
    }

    #[test]
    fn test_things_date_known_values() {
        assert_eq!(
            things_date_to_naive_date(132_822_272),
            NaiveDate::from_ymd_opt(2026, 11, 10)
        );
        assert_eq!(
            naive_date_to_things_timestamp(NaiveDate::from_ymd_opt(2026, 11, 10).unwrap()),
            132_822_272
        );
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
        for (y, m, d) in [(2001, 1, 1), (2010, 6, 15), (2024, 2, 29), (2025, 7, 4)] {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            assert_eq!(
                things_date_to_naive_date(naive_date_to_things_timestamp(date)),
                Some(date)
            );
        }
    }
}
