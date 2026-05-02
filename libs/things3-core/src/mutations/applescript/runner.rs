//! `osascript` runner.
//!
//! Spawns `osascript -` and feeds the script via stdin. Piping through stdin
//! sidesteps every shell-escaping vector — the script bytes are not
//! interpreted by any shell.

use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{Result, ThingsError};

/// Spawn `osascript`, pipe `script` via stdin, and return the captured stdout.
///
/// On non-zero exit, errors are mapped to [`ThingsError::AppleScript`] with
/// messages tailored to the failure mode (TCC denied, Things 3 not running,
/// osascript missing, generic).
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134 (Phase B).
pub(crate) async fn run_script(script: &str) -> Result<String> {
    let mut child = Command::new("osascript")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            ThingsError::applescript(format!(
                "osascript not available — AppleScriptBackend is macOS-only ({e})"
            ))
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ThingsError::applescript("osascript stdin not piped"))?;
    stdin
        .write_all(script.as_bytes())
        .await
        .map_err(|e| ThingsError::applescript(format!("failed to write script: {e}")))?;
    stdin
        .shutdown()
        .await
        .map_err(|e| ThingsError::applescript(format!("failed to close stdin: {e}")))?;
    // Explicitly drop stdin to close the write end of the pipe. shutdown() marks
    // the stream done but does not close the fd on Unix pipes — osascript hangs
    // waiting for more input until the write end is closed.
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| ThingsError::applescript(format!("osascript wait failed: {e}")))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(map_failure(&stderr))
}

fn map_failure(stderr: &str) -> ThingsError {
    let lowered = stderr.to_lowercase();
    if stderr.contains("-1743") || lowered.contains("not authori") {
        return ThingsError::applescript(
            "macOS Automation permission denied. Grant access in \
             System Settings → Privacy & Security → Automation, then retry.",
        );
    }
    if stderr.contains("-600") || stderr.contains("Application isn't running") {
        return ThingsError::applescript("Things 3 is not running. Launch Things 3 and retry.");
    }
    if stderr.contains("-10810") || stderr.contains("NSWorkspaceNotFound") {
        return ThingsError::applescript("Things 3 is not installed at the expected location.");
    }
    ThingsError::applescript(format!("osascript failed: {}", stderr.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_failure_tcc_denied() {
        let err = map_failure("execution error: Not authorized to send Apple events. (-1743)");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("Automation permission denied"));
                assert!(message.contains("System Settings"));
            }
            _ => panic!("expected AppleScript error"),
        }
    }

    #[test]
    fn map_failure_not_running() {
        let err =
            map_failure("execution error: Things3 got an error: Application isn't running. (-600)");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("Things 3 is not running"));
            }
            _ => panic!("expected AppleScript error"),
        }
    }

    #[test]
    fn map_failure_not_installed() {
        let err = map_failure("execution error: NSWorkspaceNotFound (-10810)");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("not installed"));
            }
            _ => panic!("expected AppleScript error"),
        }
    }

    #[test]
    fn map_failure_generic() {
        let err = map_failure("syntax error: bad keyword");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("osascript failed"));
                assert!(message.contains("syntax error"));
            }
            _ => panic!("expected AppleScript error"),
        }
    }

    #[test]
    fn map_failure_trims_whitespace() {
        let err = map_failure("  some error\n");
        match err {
            ThingsError::AppleScript { message } => {
                assert_eq!(message, "osascript failed: some error");
            }
            _ => panic!("expected AppleScript error"),
        }
    }

    #[tokio::test]
    async fn run_script_returns_stdout_for_arithmetic() {
        let out = run_script("return 1 + 1")
            .await
            .expect("osascript should run");
        assert_eq!(out.trim(), "2");
    }

    #[tokio::test]
    async fn run_script_maps_runtime_error() {
        // Force AppleScript to raise an error number we don't special-case.
        let err = run_script("error \"deliberate failure\" number 99")
            .await
            .expect_err("script should fail");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("osascript failed"));
                assert!(message.contains("deliberate failure"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn run_script_handles_string_return() {
        let out = run_script("return \"hello\"")
            .await
            .expect("osascript should run");
        assert!(out.contains("hello"));
    }

    #[tokio::test]
    async fn run_script_escaped_string_round_trips() {
        use crate::mutations::applescript::escape::as_applescript_string;

        let title = "Buy \"organic\" milk\nand \\bread";
        let escaped = as_applescript_string(title);
        let script = format!("return {escaped}");
        let out = run_script(&script).await.expect("osascript should run");
        // Inner double-quote didn't break the string literal — "organic" is present
        assert!(out.contains("organic"), "output was: {out:?}");
        // Text after embedded newline survived
        assert!(out.contains("bread"), "output was: {out:?}");
        // Backslash round-tripped: \\ in source → single \ in output
        assert!(out.contains('\\'), "output was: {out:?}");
    }
}
