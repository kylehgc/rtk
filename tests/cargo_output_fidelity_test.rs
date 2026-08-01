//! Fork issue #75: proof tests for `rtk cargo test`'s compiler-warning handling.
//!
//! Upstream's `CargoTestHandler` / `filter_cargo_test` (`src/cmds/rust/cargo_cmd.rs`)
//! drop every compile-phase `warning:` block on a passing `cargo test` run outright —
//! `should_skip`/`is_block_start` never recognize a `warning:` line, and the unmatched
//! line falls through `BlockStreamFilter::feed_line`'s default (drop). An agent reading
//! "N passed" concludes the build is clean even when rustc warned. Same story in the
//! buffered `filter_cargo_test`, used by `rtk pipe --filter cargo-test`: no warning
//! capture at all, so a run with no test-result line and no compile error falls straight
//! to a bare last-5-raw-lines tail with no idea a warning block even exists.
//!
//! This fork's ff13986 fixes the fidelity gap (streams/captures the warning block
//! instead of dropping it); f954b61 fixes a duplication bug in the buffered fallback
//! that ff13986 introduced (the raw-tail fallback could restate lines the captured
//! warnings section already printed). Both are exercised here against a real,
//! dependency-free fixture crate and the real `rtk` binary — never `use rtk::...`.
//!
//! Dropped from this suite: 6235d4b ("keep compile errors visible when warnings are
//! captured"). Verified by hand against both binaries (fork and upstream/develop) on a
//! crate with an unused-variable warning in one function and a type-mismatch compile
//! error in another: both report the error and the warning exactly once, just via
//! different code paths (fork streams the warning live and skips it in the buffered
//! fallback; upstream never streams anything live and renders both from a single
//! buffered pass). The bug 6235d4b fixes was introduced and resolved entirely within
//! this fork's own history — upstream's baseline never had it, so there is no portable
//! command-output difference left to prove.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const FIXTURE_CARGO_TOML: &str = r#"[package]
name = "rtkfixture"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rtkfixture"
path = "src/main.rs"
"#;

// One bin, one deliberate `unused_variable` warning, one passing test. Zero
// dependencies so the nested cargo invocation below builds offline.
const FIXTURE_MAIN: &str = r#"fn warns() -> i32 {
    let unused_value = 42;
    7
}

fn main() {
    println!("{}", warns());
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_passes() {
        assert_eq!(2 + 2, 4);
    }
}
"#;

fn write_fixture(dir: &Path) {
    fs::write(dir.join("Cargo.toml"), FIXTURE_CARGO_TOML).expect("write fixture Cargo.toml");
    let src = dir.join("src");
    fs::create_dir_all(&src).expect("create fixture src dir");
    fs::write(src.join("main.rs"), FIXTURE_MAIN).expect("write fixture main.rs");
}

/// Skip rather than fail where cargo isn't available, so the suite stays green off a
/// bare checkout (mirrors `tests/git_machine_output_test.rs`'s git-availability guard).
fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// FIDELITY: a passing `rtk cargo test` must still surface the compiler warning rustc
/// emitted, not just a clean "N passed" summary.
#[test]
fn cargo_test_preserves_compiler_warnings_on_passing_run() {
    if !cargo_available() {
        eprintln!("skipping: cargo not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("create fixture tempdir");
    write_fixture(dir.path());

    let out = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["cargo", "test"])
        .current_dir(dir.path())
        .output()
        .expect("run rtk cargo test");
    // The live-streamed warning block is written to whichever fd fed it (cargo's own
    // build diagnostics go to stderr); the compact pass/warning-count summary is
    // written wherever the last raw line came from (test-harness output on stdout).
    // Check both, combined, exactly as a terminal watching this run would see it.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        combined.contains("unused_value"),
        "fork must preserve the unused-variable warning detail on a passing \
         `cargo test`, got:\n{combined}"
    );
    assert!(
        combined.contains("compiler warning"),
        "fork must annotate the pass summary with a compiler-warning count, got:\n{combined}"
    );
    assert!(
        combined.contains("passed"),
        "the underlying test result must still be reported, got:\n{combined}"
    );
}

/// REDUCTION: `rtk pipe --filter cargo-test` on raw `cargo test --no-run` output (a
/// warning, no test-result line, no compile error — exactly the shape with no
/// confident-detection branch to fall into) must show the warning's full detail exactly
/// once, not restate lines the captured-warnings section already printed via the
/// raw-tail fallback below it.
#[test]
fn piped_cargo_test_filter_does_not_restate_captured_warnings() {
    if !cargo_available() {
        eprintln!("skipping: cargo not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("create fixture tempdir");
    write_fixture(dir.path());

    let raw_out = Command::new("cargo")
        .args(["test", "--no-run"])
        .current_dir(dir.path())
        .output()
        .expect("run cargo test --no-run");
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&raw_out.stdout),
        String::from_utf8_lossy(&raw_out.stderr)
    );
    assert!(
        raw.contains("unused_value"),
        "fixture must actually warn for this test to mean anything, got raw:\n{raw}"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["pipe", "--filter", "cargo-test"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn rtk pipe --filter cargo-test");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(raw.as_bytes())
        .expect("write raw cargo output to rtk pipe stdin");
    let out = child
        .wait_with_output()
        .expect("wait for rtk pipe --filter cargo-test");
    let filtered = String::from_utf8_lossy(&out.stdout);

    // Measured: the note/help lines that trail the warning block are exactly the ones
    // the pre-f954b61 raw-tail fallback would restate (they fall in its last-5-lines
    // window). Assert the duplication marker appears at most once.
    let dup_marker = "= note: `#[warn(unused_variables)]`";
    let occurrences = filtered.matches(dup_marker).count();
    assert_eq!(
        occurrences, 1,
        "warning detail must appear exactly once (not restated by the raw-tail \
         fallback), got {occurrences} occurrences in:\n{filtered}"
    );

    // Combined with dedup: the full block (not just the trailing note/help lines a bare
    // raw-tail fallback would happen to catch) must survive — this is what upstream's
    // undifferentiated last-5-lines fallback drops instead of restates.
    assert!(
        filtered.contains("unused_value"),
        "filtered output must retain the full warning block, got:\n{filtered}"
    );

    assert!(
        filtered.len() < raw.len(),
        "filtered output ({} bytes) should be smaller than raw cargo output ({} bytes)",
        filtered.len(),
        raw.len()
    );
}
