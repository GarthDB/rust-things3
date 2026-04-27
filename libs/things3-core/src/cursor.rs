//! Opaque keyset pagination cursors.
//!
//! [`Cursor`] is a base64-encoded JSON payload identifying the last task
//! returned in a page. [`Page`] bundles a slice of items with the optional
//! cursor for the next page. Both are returned by
//! [`crate::query::TaskQueryBuilder::execute_paged`].
//!
//! Cursors anchor on `(created, uuid)`:
//! - `created` is immutable (a task's creation date never changes), so a
//!   cursor remains valid even if the underlying task is edited between
//!   page fetches.
//! - `uuid` is a deterministic tiebreaker.
//!
//! The encoded form is URL-safe base64 (no padding) so cursors can travel
//! through HTTP query strings without further escaping.
//!
//! Requires the `batch-operations` feature flag.

#![cfg(feature = "batch-operations")]

use std::fmt;
use std::str::FromStr;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, ThingsError};

/// Opaque pagination token. Constructed by
/// [`crate::query::TaskQueryBuilder::execute_paged`] and round-tripped
/// through [`Display`]/[`FromStr`].
///
/// Callers should treat the wrapped string as opaque — the encoding is an
/// implementation detail that may change without breaking the public API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor(String);

impl Cursor {
    /// Borrow the encoded string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Encode a payload into a cursor.
    ///
    /// Only invoked from `execute_paged`, which requires both `advanced-queries`
    /// and `batch-operations`. Under `batch-operations` alone the function is
    /// unused — allow dead_code rather than tightening the gate on the type.
    #[cfg_attr(not(feature = "advanced-queries"), allow(dead_code))]
    pub(crate) fn encode(payload: &CursorPayload) -> Result<Self> {
        let bytes = serde_json::to_vec(payload)?;
        Ok(Self(URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// Decode the cursor back into its payload.
    pub(crate) fn decode(&self) -> Result<CursorPayload> {
        let bytes = URL_SAFE_NO_PAD
            .decode(self.0.as_bytes())
            .map_err(|e| ThingsError::InvalidCursor(format!("base64 decode failed: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ThingsError::InvalidCursor(format!("payload parse failed: {e}")))
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Cursor {
    type Err = ThingsError;

    fn from_str(s: &str) -> Result<Self> {
        let cursor = Self(s.to_string());
        // Validate decoding eagerly so bad input is rejected at the API boundary.
        cursor.decode()?;
        Ok(cursor)
    }
}

/// Internal cursor payload: the anchor for "what comes after."
///
/// Field names are deliberately one letter to keep the encoded cursor short.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CursorPayload {
    /// `created` of the last-returned task.
    pub(crate) c: DateTime<Utc>,
    /// `uuid` of the last-returned task.
    pub(crate) u: Uuid,
}

/// A page of results plus an optional cursor for the next page.
///
/// `next_cursor` is `None` when the page is the last one (i.e. the page
/// returned fewer items than the configured page size).
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> CursorPayload {
        CursorPayload {
            c: DateTime::parse_from_rfc3339("2026-04-27T12:34:56Z")
                .unwrap()
                .with_timezone(&Utc),
            u: Uuid::parse_str("12345678-1234-5678-1234-567812345678").unwrap(),
        }
    }

    #[test]
    fn test_cursor_encode_decode_roundtrip() {
        let p = payload();
        let cursor = Cursor::encode(&p).unwrap();
        let decoded = cursor.decode().unwrap();
        assert_eq!(decoded, p);
    }

    #[test]
    fn test_cursor_display_fromstr_roundtrip() {
        let p = payload();
        let cursor = Cursor::encode(&p).unwrap();
        let s = cursor.to_string();
        let parsed = Cursor::from_str(&s).unwrap();
        assert_eq!(parsed, cursor);
    }

    #[test]
    fn test_cursor_rejects_invalid_base64() {
        let err = Cursor::from_str("!!!not base64!!!").unwrap_err();
        match err {
            ThingsError::InvalidCursor(msg) => assert!(msg.contains("base64")),
            other => panic!("expected InvalidCursor, got {other:?}"),
        }
    }

    #[test]
    fn test_cursor_rejects_invalid_json() {
        let bogus = URL_SAFE_NO_PAD.encode(b"not json");
        let err = Cursor::from_str(&bogus).unwrap_err();
        match err {
            ThingsError::InvalidCursor(msg) => assert!(msg.contains("payload parse failed")),
            other => panic!("expected InvalidCursor, got {other:?}"),
        }
    }

    #[test]
    fn test_cursor_rejects_missing_fields() {
        let bogus = URL_SAFE_NO_PAD.encode(b"{}");
        assert!(matches!(
            Cursor::from_str(&bogus),
            Err(ThingsError::InvalidCursor(_))
        ));
    }

    #[test]
    fn test_cursor_url_safe_encoding() {
        // URL-safe base64 uses '-' and '_' instead of '+' and '/' and omits padding.
        let p = payload();
        let cursor = Cursor::encode(&p).unwrap();
        let s = cursor.as_str();
        assert!(!s.contains('+'), "cursor should not contain '+': {s}");
        assert!(!s.contains('/'), "cursor should not contain '/': {s}");
        assert!(
            !s.contains('='),
            "cursor should not contain '=' padding: {s}"
        );
    }

    #[test]
    fn test_cursor_serde_through_json() {
        let p = payload();
        let cursor = Cursor::encode(&p).unwrap();
        let json = serde_json::to_string(&cursor).unwrap();
        let parsed: Cursor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cursor);
    }

    #[test]
    fn test_page_construction() {
        let page: Page<i32> = Page {
            items: vec![1, 2, 3],
            next_cursor: None,
        };
        assert_eq!(page.items.len(), 3);
        assert!(page.next_cursor.is_none());
    }
}
