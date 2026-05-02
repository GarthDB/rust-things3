//! Parse `osascript` stdout into typed values.
//!
//! Things 3 AppleScript references look like
//! `to do id "ABCDEF-..." of application "Things3"`. When a script returns
//! `id of newTask`, osascript prints just the UUID string. This module
//! handles both shapes defensively so a future script change that returns the
//! full reference does not break callers.

use uuid::Uuid;

use crate::error::{Result, ThingsError};

/// Extract a `Uuid` from an osascript stdout buffer.
///
/// Accepts either:
/// - a bare UUID string (the result of `return id of someTask`)
/// - a Things-style reference like `to do id "<uuid>" of application "Things3"`
///   (a defensive fallback so we cope if a future script returns the reference
///   instead of the bare id)
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn extract_id(stdout: &str) -> Result<Uuid> {
    let trimmed = stdout.trim();

    if let Ok(uuid) = Uuid::parse_str(trimmed) {
        return Ok(uuid);
    }

    if let Some(start) = trimmed.find("id \"") {
        let after = &trimmed[start + 4..];
        if let Some(end) = after.find('"') {
            let candidate = &after[..end];
            if let Ok(uuid) = Uuid::parse_str(candidate) {
                return Ok(uuid);
            }
        }
    }

    Err(ThingsError::applescript(format!(
        "could not extract UUID from osascript output: {trimmed:?}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_UUID: &str = "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e";

    #[test]
    fn extracts_bare_uuid() {
        let uuid = extract_id(SAMPLE_UUID).unwrap();
        assert_eq!(uuid.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_bare_uuid_with_trailing_newline() {
        // Real osascript stdout always ends with a newline.
        let stdout = format!("{SAMPLE_UUID}\n");
        let uuid = extract_id(&stdout).unwrap();
        assert_eq!(uuid.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_bare_uuid_with_surrounding_whitespace() {
        let stdout = format!("  {SAMPLE_UUID}  \n");
        let uuid = extract_id(&stdout).unwrap();
        assert_eq!(uuid.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_uuid_from_things_reference() {
        let stdout = format!("to do id \"{SAMPLE_UUID}\" of application \"Things3\"\n");
        let uuid = extract_id(&stdout).unwrap();
        assert_eq!(uuid.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_first_uuid_from_multiple_references() {
        // If a script accidentally returns multiple references, take the first.
        let second = "11111111-2222-3333-4444-555555555555";
        let stdout = format!(
            "to do id \"{SAMPLE_UUID}\" of application \"Things3\", \
             to do id \"{second}\" of application \"Things3\""
        );
        let uuid = extract_id(&stdout).unwrap();
        assert_eq!(uuid.to_string(), SAMPLE_UUID);
    }

    #[test]
    fn rejects_empty_input() {
        let err = extract_id("").unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("could not extract UUID"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[test]
    fn rejects_garbage() {
        let err = extract_id("not a uuid at all").unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("could not extract UUID"));
                assert!(message.contains("not a uuid at all"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[test]
    fn rejects_id_pattern_with_invalid_uuid() {
        let stdout = "to do id \"not-actually-a-uuid\" of application \"Things3\"";
        let err = extract_id(stdout).unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("could not extract UUID"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }
}
