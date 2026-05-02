//! AppleScript-based mutation backend.
//!
//! Drives Things 3 via `osascript` — CulturedCode's [documented Mac-only
//! scripting API](https://culturedcode.com/things/support/articles/4562654/).
//! Replaces direct-SQLite writes (which CulturedCode warns can corrupt the
//! user's database — see [the safety
//! article](https://culturedcode.com/things/support/articles/5510170/)) for
//! every mutation operation rust-things3 exposes.
//!
//! ## Layout
//!
//! - [`escape`] — pure string-literal escaping; the script-injection guard
//! - [`runner`] — `osascript` process spawn + error mapping
//! - The `AppleScriptBackend` struct and `MutationBackend` impl land in #134
//!   (Phase B); this module is gated `#[cfg(target_os = "macos")]` and
//!   currently only exposes the foundation pieces.

pub(crate) mod escape;
pub(crate) mod runner;
