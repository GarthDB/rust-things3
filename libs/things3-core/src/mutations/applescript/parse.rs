//! Parse `osascript` stdout into typed values.
//!
//! Things 3 AppleScript references look like
//! `to do id "ABCDEF-..." of application "Things3"`. When a script returns
//! `id of newTask`, osascript prints just the UUID string. This module
//! handles both shapes defensively so a future script change that returns the
//! full reference does not break callers.

use crate::error::{Result, ThingsError};
use crate::models::ThingsId;

/// Extract a [`ThingsId`] from an osascript stdout buffer.
///
/// Accepts either:
/// - a bare ID string (the result of `return id of someTask`) — Things 3
///   returns its native base62-style IDs (21–22 chars) or RFC-4122 UUIDs
/// - a Things-style reference like `to do id "<id>" of application "Things3"`
///   (a defensive fallback so we cope if a future script returns the reference
///   instead of the bare id)
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn extract_id(stdout: &str) -> Result<ThingsId> {
    let trimmed = stdout.trim();

    if !trimmed.is_empty() && !trimmed.contains('"') {
        // Bare ID — trust it (Things 3 controls the output format).
        return Ok(ThingsId::from_trusted(trimmed.to_string()));
    }

    if let Some(start) = trimmed.find("id \"") {
        let after = &trimmed[start + 4..];
        if let Some(end) = after.find('"') {
            let candidate = &after[..end];
            if !candidate.is_empty() {
                return Ok(ThingsId::from_trusted(candidate.to_string()));
            }
        }
    }

    Err(ThingsError::applescript(format!(
        "could not extract ID from osascript output: {trimmed:?}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_UUID: &str = "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e";
    const SAMPLE_THINGS_ID: &str = "R4t2G8Q63aGZq4epMHNeCr";

    #[test]
    fn extracts_bare_uuid() {
        let id = extract_id(SAMPLE_UUID).unwrap();
        assert_eq!(id.as_str(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_bare_things_id() {
        // Things 3 native base62-style IDs (21–22 chars).
        let id = extract_id(SAMPLE_THINGS_ID).unwrap();
        assert_eq!(id.as_str(), SAMPLE_THINGS_ID);
    }

    #[test]
    fn extracts_bare_uuid_with_trailing_newline() {
        // Real osascript stdout always ends with a newline.
        let stdout = format!("{SAMPLE_UUID}\n");
        let id = extract_id(&stdout).unwrap();
        assert_eq!(id.as_str(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_bare_uuid_with_surrounding_whitespace() {
        let stdout = format!("  {SAMPLE_UUID}  \n");
        let id = extract_id(&stdout).unwrap();
        assert_eq!(id.as_str(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_uuid_from_things_reference() {
        let stdout = format!("to do id \"{SAMPLE_UUID}\" of application \"Things3\"\n");
        let id = extract_id(&stdout).unwrap();
        assert_eq!(id.as_str(), SAMPLE_UUID);
    }

    #[test]
    fn extracts_first_uuid_from_multiple_references() {
        // If a script accidentally returns multiple references, take the first.
        let second = "11111111-2222-3333-4444-555555555555";
        let stdout = format!(
            "to do id \"{SAMPLE_UUID}\" of application \"Things3\", \
             to do id \"{second}\" of application \"Things3\""
        );
        let id = extract_id(&stdout).unwrap();
        assert_eq!(id.as_str(), SAMPLE_UUID);
    }

    #[test]
    fn rejects_empty_input() {
        let err = extract_id("").unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("could not extract ID"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[test]
    fn extracts_things_id_from_reference() {
        // Things 3 native IDs in reference format.
        let stdout = format!("to do id \"{SAMPLE_THINGS_ID}\" of application \"Things3\"\n");
        let id = extract_id(&stdout).unwrap();
        assert_eq!(id.as_str(), SAMPLE_THINGS_ID);
    }
}
