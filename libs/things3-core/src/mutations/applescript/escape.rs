//! AppleScript string-literal escaping.
//!
//! Every untrusted string (task title, notes, tag name, …) flowing into a
//! script must pass through [`as_applescript_string`]. Without escaping, a
//! title containing `"` ends the literal early — at best a syntax error, at
//! worst arbitrary AppleScript injection.
//!
//! The returned value includes the surrounding `"` characters so callers can
//! splice it directly into a script template:
//!
//! ```ignore
//! let script = format!(
//!     "make new to do with properties {{name:{}}}",
//!     escape::as_applescript_string(title),
//! );
//! ```

/// Escape `s` as an AppleScript string literal, with surrounding quotes.
///
/// The escape sequences `\\`, `\"`, `\n`, `\r`, `\t` are recognised by
/// AppleScript on every macOS version Things 3 supports.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134 (Phase B).
#[must_use]
pub(crate) fn as_applescript_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::as_applescript_string;

    #[test]
    fn empty() {
        assert_eq!(as_applescript_string(""), "\"\"");
    }

    #[test]
    fn plain_ascii() {
        assert_eq!(as_applescript_string("Hello world"), "\"Hello world\"");
    }

    #[test]
    fn double_quote() {
        assert_eq!(as_applescript_string("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn backslash() {
        assert_eq!(as_applescript_string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn newline() {
        assert_eq!(as_applescript_string("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn carriage_return() {
        assert_eq!(as_applescript_string("a\rb"), "\"a\\rb\"");
    }

    #[test]
    fn tab() {
        assert_eq!(as_applescript_string("a\tb"), "\"a\\tb\"");
    }

    #[test]
    fn mixed_whitespace() {
        assert_eq!(
            as_applescript_string("line1\n\tindented \"quoted\"\\\r"),
            "\"line1\\n\\tindented \\\"quoted\\\"\\\\\\r\""
        );
    }

    #[test]
    fn unicode_passes_through() {
        // Cyrillic, CJK, emoji — all valid inside an AppleScript string literal
        assert_eq!(as_applescript_string("café"), "\"café\"");
        assert_eq!(as_applescript_string("日本語"), "\"日本語\"");
        assert_eq!(as_applescript_string("🚀"), "\"🚀\"");
    }

    #[test]
    fn unicode_line_separators_pass_through() {
        // U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR have no
        // dedicated AppleScript escape; they're valid inside string literals
        // so we leave them as-is.
        assert_eq!(as_applescript_string("a\u{2028}b"), "\"a\u{2028}b\"");
        assert_eq!(as_applescript_string("a\u{2029}b"), "\"a\u{2029}b\"");
    }

    #[test]
    fn injection_attempt_is_neutralised() {
        // The classic shell-style injection vector: a quote that breaks the
        // string, then arbitrary AppleScript, then a continuation. After
        // escaping, every special character is inside the string literal —
        // no execution outside it is possible.
        let payload = r#""); do shell script "rm -rf /"; (""#;
        let escaped = as_applescript_string(payload);
        // Output is one balanced literal — every interior `"` is escaped.
        assert!(escaped.starts_with('"') && escaped.ends_with('"'));
        // Count of unescaped quotes (preceded by even number of backslashes)
        // is exactly 2 — the outer pair.
        let body = &escaped[1..escaped.len() - 1];
        for (i, ch) in body.char_indices() {
            if ch == '"' {
                // Must be preceded by an odd number of backslashes,
                // i.e. escaped.
                let preceding_backslashes =
                    body[..i].chars().rev().take_while(|c| *c == '\\').count();
                assert!(
                    preceding_backslashes % 2 == 1,
                    "unescaped quote at byte {i} in {escaped:?}"
                );
            }
        }
    }

    #[test]
    fn long_runs_of_specials() {
        assert_eq!(as_applescript_string("\\\\\\\""), "\"\\\\\\\\\\\\\\\"\"");
    }

    #[test]
    fn round_trip_via_pure_concat() {
        // A practical test: build a synthetic script, count escaped quotes,
        // confirm balance.
        let title = "Buy \"organic\" milk\nand \\bread\t";
        let snippet = format!(
            "make new to do with properties {{name:{}, notes:{}}}",
            as_applescript_string(title),
            as_applescript_string("multi\nline\nnote"),
        );
        // Quotes in the snippet, excluding escaped ones, should be exactly
        // 4 — the bounding pair around each of the two literals.
        let mut unescaped_quotes = 0;
        let mut prev = '\0';
        for ch in snippet.chars() {
            if ch == '"' && prev != '\\' {
                unescaped_quotes += 1;
            }
            prev = ch;
        }
        assert_eq!(unescaped_quotes, 4, "snippet was: {snippet:?}");
    }
}
