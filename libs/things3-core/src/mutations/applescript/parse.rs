//! Parse `osascript` stdout into typed values.
//!
//! Things 3 AppleScript references look like
//! `to do id "ABCDEF-..." of application "Things3"`. When a script returns
//! `id of newTask`, osascript prints just the UUID string. This module
//! handles both shapes defensively so a future script change that returns the
//! full reference does not break callers.

use crate::error::{Result, ThingsError};
use crate::models::{BulkOperationResult, ThingsId};

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
        // We intentionally do NOT validate the format here: Things 3 native IDs
        // (21–22-char base62) are not UUIDs, and new ID shapes may appear in future
        // Things releases. run_script only calls this on a successful osascript exit,
        // so corrupt/diagnostic output on a failed exit never reaches here. Strict
        // format validation happens at the MCP boundary via ThingsId::from_str.
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

/// Parse the output of a [`super::script::bulk_wrap`]-built bulk script into a
/// [`BulkOperationResult`].
///
/// Expected output shapes:
/// - `"OK <count>"` — all items succeeded
/// - `"OK <count>\nitem <idx>: <msg>\nitem <idx>: <msg>\n..."` — partial failure
///
/// `total` is the requested item count; used to build a clear message when
/// failures occur.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn parse_bulk_result(stdout: &str, total: usize) -> Result<BulkOperationResult> {
    let trimmed = stdout.trim();
    let mut lines = trimmed.lines();
    let header = lines.next().unwrap_or("");

    let processed: usize = header
        .strip_prefix("OK ")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| {
            ThingsError::applescript(format!(
                "bulk script returned unexpected output: {trimmed:?}"
            ))
        })?;

    // Clamp against `total` to keep the count coherent if a script-generation
    // bug ever reports more processed items than were requested.
    let processed = processed.min(total);

    let errors: Vec<String> = lines.map(|l| l.trim().to_string()).collect();
    let success = errors.is_empty();
    let message = if success {
        format!("Successfully processed {processed} item(s)")
    } else {
        format!(
            "Processed {processed}/{total}; errors: {}",
            errors.join("; ")
        )
    };

    Ok(BulkOperationResult {
        success,
        processed_count: processed,
        message,
    })
}

/// Parse the output of the atomic [`super::script::bulk_create_tasks_script`].
///
/// Expected output shapes:
/// - `"OK <count>"` — all tasks created successfully
/// - `"ROLLBACK: <msg>"` — creation failed; the script already deleted any
///   partial creates, so the caller should surface the error as-is
#[allow(dead_code)] // Used by AppleScriptBackend, added in #157.
pub(crate) fn parse_atomic_bulk_create_result(stdout: &str) -> Result<BulkOperationResult> {
    let trimmed = stdout.trim();
    if let Some(msg) = trimmed.strip_prefix("ROLLBACK: ") {
        return Err(ThingsError::applescript(format!(
            "bulk_create_tasks rolled back after partial failure: {msg}"
        )));
    }
    let processed: usize = trimmed
        .strip_prefix("OK ")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| {
            ThingsError::applescript(format!(
                "bulk_create_tasks returned unexpected output: {trimmed:?}"
            ))
        })?;
    Ok(BulkOperationResult {
        success: true,
        processed_count: processed,
        message: format!("Successfully created {processed} task(s)"),
    })
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

    /// Bare output with no quotes is trusted verbatim — no format validation.
    /// This is intentional: Things native IDs (base62, 21–22 chars) are not UUIDs.
    /// Validation happens at the MCP boundary via `ThingsId::from_str`, not here.
    #[test]
    fn accepts_bare_non_uuid_string_intentionally() {
        // "not a uuid at all" has no quotes and is non-empty, so it parses.
        let id = extract_id("not a uuid at all").unwrap();
        assert_eq!(id.as_str(), "not a uuid at all");
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

    #[test]
    fn parse_bulk_all_success() {
        let res = parse_bulk_result("OK 5\n", 5).unwrap();
        assert!(res.success);
        assert_eq!(res.processed_count, 5);
        assert!(res.message.contains("Successfully processed 5"));
    }

    #[test]
    fn parse_bulk_partial_failure() {
        let stdout = "OK 3\nitem 1: not found\nitem 4: invalid";
        let res = parse_bulk_result(stdout, 5).unwrap();
        assert!(!res.success);
        assert_eq!(res.processed_count, 3);
        assert!(res.message.contains("3/5"));
        assert!(res.message.contains("item 1: not found"));
        assert!(res.message.contains("item 4: invalid"));
    }

    #[test]
    fn parse_bulk_zero_items() {
        let res = parse_bulk_result("OK 0\n", 0).unwrap();
        assert!(res.success);
        assert_eq!(res.processed_count, 0);
    }

    #[test]
    fn parse_bulk_clamps_processed_to_total() {
        // Defensive: if a future script-generation bug ever reports more
        // processed items than were requested, the parser must not return a
        // count that exceeds `total`.
        let res = parse_bulk_result("OK 10\n", 5).unwrap();
        assert!(res.success);
        assert_eq!(res.processed_count, 5);
    }

    #[test]
    fn parse_bulk_rejects_garbage_header() {
        let err = parse_bulk_result("garbage", 1).unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("unexpected output"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[test]
    fn parse_atomic_bulk_create_success() {
        let res = parse_atomic_bulk_create_result("OK 3").unwrap();
        assert!(res.success);
        assert_eq!(res.processed_count, 3);
        assert!(res.message.contains("Successfully created 3"));
    }

    #[test]
    fn parse_atomic_bulk_create_rollback_returns_err() {
        let err =
            parse_atomic_bulk_create_result("ROLLBACK: project id \"bad-uuid\" doesn't exist")
                .unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("rolled back"));
                assert!(message.contains("bad-uuid"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[test]
    fn parse_atomic_bulk_create_rejects_garbage() {
        let err = parse_atomic_bulk_create_result("garbage").unwrap_err();
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("unexpected output"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }
}
