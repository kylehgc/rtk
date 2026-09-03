#![cfg(unix)]
//! A tool's stderr must reach the user whatever the exit code. Filters that
//! compress stdout run with `RunOptions::stdout_only`, which keeps stderr out of
//! the filter input — so unless the runner emits it, the diagnostics are dropped
//! and the exit code becomes the only surviving signal (upstream #3029), or, on
//! a success exit with empty stdout, nothing at all survives (upstream #3772).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const DIAGNOSTIC: &str = "shim-diagnostic: dependency is missing";

/// A fake `ruff` that fails the way real tools do: message on stderr, nothing on
/// stdout, nonzero exit. `rtk ruff` filters stdout only, which is the path under
/// test; `resolved_command` resolves via PATH, so the shim takes precedence.
fn shim(exit_code: i32) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create shim tempdir");
    let bin = dir.path().join("ruff");
    fs::write(
        &bin,
        format!("#!/bin/sh\necho '{DIAGNOSTIC}' >&2\nexit {exit_code}\n"),
    )
    .expect("write shim script");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod shim");
    dir
}

fn rtk_ruff(shim_dir: &tempfile::TempDir, db: &tempfile::TempDir) -> std::process::Output {
    let path = format!(
        "{}:{}",
        shim_dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["ruff", "check", "."])
        .env("PATH", path)
        // Keep tracking off the developer's real database.
        .env("RTK_DB_PATH", db.path().join("t.db"))
        .output()
        .expect("rtk")
}

#[test]
fn failing_tool_stderr_reaches_the_user() {
    let db = tempfile::tempdir().expect("create db tempdir");
    let out = rtk_ruff(&shim(1), &db);

    assert_eq!(out.status.code(), Some(1), "exit code must propagate");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(DIAGNOSTIC),
        "diagnostic swallowed — stderr was {:?}, stdout was {:?}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn successful_tool_stderr_reaches_the_user() {
    // A success exit carries diagnostics too — a deprecation notice, a missing
    // config, prettier's whole report — and with stdout empty the never-worse
    // guard used to collapse the run to silence on both streams.
    let db = tempfile::tempdir().expect("create db tempdir");
    let out = rtk_ruff(&shim(0), &db);

    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(DIAGNOSTIC),
        "diagnostic swallowed on a success exit — stderr was {:?}, stdout was {:?}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "the tool said nothing on stdout, so neither may rtk"
    );
}
