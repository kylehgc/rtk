//! Reduction claim: `rtk tsc` must compress `tsc --noEmit --pretty` output
//! (ANSI colors, code frames, carets) at least as well as it compresses the
//! plain `file(line,col): error TSxxxx` format it already handled.
//!
//! Upstream now parses the pretty format itself (`fix(tsc): handle pretty
//! diagnostics`, 9d1c60a, adopted in the 2026-08-29 sync), so the parsing gap
//! this file was written against is closed. What stays fork-only is the
//! coverage: upstream asserts no token-savings floor for this filter, and has
//! no end-to-end test driving the built binary. This keeps both.
//!
//! `filter_tsc_output` lives in `src/cmds/js/tsc_cmd.rs`, unreachable from a
//! portable test directly. But `rtk pipe --filter tsc` (`src/cmds/system/
//! pipe_cmd.rs`) routes stdin through that exact function without needing a
//! real `tsc` install, so this drives the real captured fixture
//! (`tests/fixtures/tsc_pretty_errors_raw.txt`, real `tsc --noEmit --pretty`
//! output, added in 842ffeb) through the binary end to end.

use std::io::Write;
use std::process::{Command, Stdio};

const FIXTURE: &str = include_str!("fixtures/tsc_pretty_errors_raw.txt");

/// rtk's own token estimator (`core::tracking::estimate_tokens`, ~4 chars/token,
/// the metric `rtk gain` reports). Reimplemented here since portable tests may
/// not `use rtk::...`; whitespace tokenization would undercount the ANSI escape
/// codes this filter strips, understating the real savings.
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f64 / 4.0).ceil() as usize
}

#[test]
fn tsc_pipe_filter_compresses_real_pretty_diagnostics() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["pipe", "--filter", "tsc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rtk pipe --filter tsc");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(FIXTURE.as_bytes())
        .expect("write fixture to stdin");

    let out = child.wait_with_output().expect("wait rtk pipe");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "rtk pipe --filter tsc failed\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("TS2322"),
        "diagnostic code must survive pretty parsing: {stdout}"
    );
    assert!(
        stdout.contains("src/api/userService.ts"),
        "file path must be extracted from the pretty format: {stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "ANSI escapes must be stripped, not passed through: {stdout}"
    );

    let savings =
        100.0 - (estimate_tokens(&stdout) as f64 / estimate_tokens(FIXTURE) as f64 * 100.0);
    assert!(
        savings >= 60.0,
        "expected >=60% token savings on real pretty tsc output, got {:.1}% \
         (in={} tokens, out={} tokens)",
        savings,
        estimate_tokens(FIXTURE),
        estimate_tokens(&stdout)
    );
}
