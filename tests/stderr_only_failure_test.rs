//! Stdout-only filters parse structured stdout, but a tool's diagnostics often go to
//! stderr. Dropping that stream leaves a failing command looking silent; inventing a
//! stdout message in its place is just as wrong. These pin both halves.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

/// Writes an executable `name` in `dir` that runs `body`.
fn fake_tool(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{}\n", body)).expect("write fake tool");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod fake tool");
}

/// Runs rtk with `dir` first on PATH, so the fake tool shadows any real one.
fn rtk_with(dir: &Path, args: &[&str]) -> Output {
    let path = format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(args)
        .env("PATH", path)
        // Keep tracking off the developer's real database.
        .env("RTK_DB_PATH", dir.join("rtk-test.db"))
        .output()
        .expect("run rtk")
}

/// A golangci-lint that answers `--version`, then fails the way a config or build
/// error does: everything on stderr, stdout empty, non-zero exit.
fn fake_golangci(dir: &Path, failure: &str) {
    fake_tool(
        dir,
        "golangci-lint",
        &format!(
            r#"if [ "$1" = "--version" ]; then
  echo "golangci-lint has version 1.64.8"
  exit 0
fi
{failure}"#
        ),
    );
}

#[test]
fn golangci_lint_stderr_only_failure_reaches_the_user() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_golangci(
        dir.path(),
        r#"echo 'level=error msg="context loading failed: failed to load packages"' >&2
exit 3"#,
    );

    let out = rtk_with(dir.path(), &["golangci-lint", "run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        stderr.contains("context loading failed"),
        "the tool's own error must reach the user; got stderr: {stderr}"
    );
    assert_eq!(out.status.code(), Some(3), "exit code must propagate");
}

/// The command said nothing on stdout, so rtk must not claim a clean run there. A
/// filter's "I got nothing" placeholder is not a diagnostic — the real one is on
/// stderr. This fork's prettier filter reads that stderr report itself (fork PR
/// #117, upstream issue #2878) and summarises it on stdout, so the pin here is that the
/// report reaches the user and no success is invented, not which stream carries it.
#[test]
fn no_invented_success_when_the_tool_reported_on_stderr() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "prettier",
        r#"echo "Checking formatting..." >&2
echo "[warn] src/a.js" >&2
echo "Code style issues found in the above file(s)." >&2
exit 1"#,
    );

    let out = rtk_with(dir.path(), &["prettier", "--check", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("src/a.js"),
        "prettier's report must reach the user; got stdout: {stdout} stderr: {stderr}"
    );
    assert!(
        !stdout.contains("No errors") && !stdout.contains("formatted"),
        "a failing check must not be reported as success; got stdout: {stdout}"
    );
    assert_eq!(out.status.code(), Some(1), "exit code must propagate");
}

/// stderr goes out on its own stream, so it must not also be replayed on stdout.
#[test]
fn stderr_is_not_duplicated_onto_stdout() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "ruff",
        r#"echo "stdout line"
echo "stderr diagnostic" >&2
exit 1"#,
    );

    let out = rtk_with(dir.path(), &["ruff", "check", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !stdout.contains("stderr diagnostic"),
        "stderr must not be replayed on stdout; got stdout: {stdout}"
    );
    assert!(
        stderr.contains("stderr diagnostic"),
        "stderr must still reach the user; got stderr: {stderr}"
    );
}

/// A command that genuinely says nothing must stay silent.
#[test]
fn silent_command_stays_silent() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(dir.path(), "ruff", "exit 0");

    let out = rtk_with(dir.path(), &["ruff", "check", "."]);

    assert!(
        String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "a silent command must not gain output"
    );
    assert!(String::from_utf8_lossy(&out.stderr).trim().is_empty());
    assert_eq!(out.status.code(), Some(0));
}

/// Not a golangci-lint quirk: every stdout-only filter forwards stderr.
#[test]
fn stderr_forwarding_is_not_golangci_specific() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "ruff",
        r#"echo "ruff failed hard" >&2
exit 2"#,
    );

    let out = rtk_with(dir.path(), &["ruff", "check", "."]);

    assert!(
        String::from_utf8_lossy(&out.stderr).contains("ruff failed hard"),
        "stderr must reach the user for any stdout-only filter"
    );
    assert_eq!(out.status.code(), Some(2), "exit code must propagate");
}

/// golangci-lint exits 1 when it merely found lint issues. RTK reports them and
/// returns 0, so the linter running successfully does not fail a build.
#[test]
fn lint_issues_are_summarised_and_exit_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let issues = r#"{"Issues":[{"FromLinter":"errcheck","Text":"unchecked","Pos":{"Filename":"main.go","Line":1,"Column":1},"SourceLines":["x()"],"Severity":""}]}"#;
    fake_golangci(dir.path(), &format!("echo '{issues}'\nexit 1"));

    let out = rtk_with(dir.path(), &["golangci-lint", "run"]);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("1 issues in 1 files") && stdout.contains("errcheck"),
        "expected an issue summary; got: {stdout}"
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "exit 1 means issues found, which RTK reports without failing"
    );
}

/// A git that answers `-h`/`--help` before `--` with its usage on stdout and
/// exit 129, like the real one, and otherwise echoes its argv and exits 0.
const FAKE_GIT: &str = "for a in \"$@\"; do [ \"$a\" = \"--\" ] && break; case \"$a\" in -h|--help) echo \"usage: git $1 [<options>]\"; exit 129;; esac; done; echo \"$@\"; exit 0";

/// git answers `-h` with its usage on **stdout** and exit 129. Once `rtk git
/// log -h` / `rtk git status -h` forward the flag (they used to be clap's), the
/// call must reach the user as git's usage — not as a filter's reading of it.
#[test]
fn forwarded_help_keeps_gits_stdout_usage() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(dir.path(), "git", FAKE_GIT);
    for sub in ["log", "status"] {
        let out = rtk_with(dir.path(), &["git", sub, "-h"]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(129), "git {sub} -h exit");
        assert!(
            stdout.contains(&format!("usage: git {sub}")),
            "git {sub} -h usage must reach stdout, got {stdout:?}"
        );
    }
}

/// A tool's usage is not filter input. `rtk wget --help` used to reach the
/// wget filter, which read the usage text as a failed download; the capture
/// helper now shows it as the tool printed it and exits with the tool's code.
#[test]
fn forwarded_help_on_a_captured_tool_is_shown_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "wget",
        "echo \"Usage: wget [OPTION]... [URL]...\"; echo \"Try --help for more options.\"; exit 0",
    );
    let out = rtk_with(dir.path(), &["wget", "http://x/f", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("Usage: wget [OPTION]"),
        "usage must reach stdout verbatim, got {stdout:?}"
    );
    assert!(
        !stdout.contains(" ok |") && !stdout.to_lowercase().contains("fail"),
        "usage read as a completed download, got {stdout:?}"
    );
}

/// A tool run through `runner::run` in filtered mode: its usage must not be
/// read as a normal (empty) run. `rtk cargo build --help` used to come back as
/// "cargo build (0 crates compiled)" under cargo's own usage text.
#[test]
fn forwarded_help_on_a_filtered_tool_is_shown_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "cargo",
        "echo \"Usage: cargo build [OPTIONS]\"; echo \"  --release  Build artifacts in release mode\"; exit 0",
    );
    let out = rtk_with(dir.path(), &["cargo", "build", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("Usage: cargo build"),
        "usage must reach stdout verbatim, got {stdout:?}"
    );
    assert!(
        !stdout.contains("crates compiled"),
        "filter summary leaked under the usage, got {stdout:?}"
    );
}

/// After `--`, `-h` is a pathspec. clap strips the `--` before the git filter
/// sees the args, so the help guard must read them with it restored.
#[test]
fn dash_h_after_double_dash_stays_a_pathspec() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(dir.path(), "git", FAKE_GIT);
    let out = rtk_with(dir.path(), &["git", "log", "--", "-h"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "git log -- -h exit, got {stdout:?}"
    );
    assert!(
        !stdout.contains("usage:"),
        "`-- -h` is a pathspec, not a usage request, got {stdout:?}"
    );
}

/// `-h` is a usage request for a tool that does not define it otherwise:
/// `rtk pytest -h` must show pytest's usage, not the filter's reading of it
/// ("Pytest: No tests collected").
#[test]
fn dash_h_where_the_tool_means_help_is_shown_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "pytest",
        "echo \"usage: pytest [options] [file_or_dir] [file_or_dir] [...]\"; exit 0",
    );
    let out = rtk_with(dir.path(), &["pytest", "-h"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("usage: pytest"),
        "pytest's usage must reach stdout verbatim, got {stdout:?}"
    );
    assert!(
        !stdout.contains("No tests collected"),
        "usage read as an empty run, got {stdout:?}"
    );
}

/// `-h` is the tool's own flag where the tool defines it: `psql -h host` is
/// a connection option, so the call stays filtered instead of being shown
/// verbatim as if it were a usage request.
#[test]
fn dash_h_where_the_tool_defines_it_stays_filtered() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "psql",
        "printf ' id | name\n----+------\n  1 | a\n(1 row)\n'; exit 0",
    );
    let out = rtk_with(dir.path(), &["psql", "-h", "localhost", "-c", "select 1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("1\ta"),
        "table must be filtered to tab-separated rows, got {stdout:?}"
    );
    assert!(
        !stdout.contains("(1 row)") && !stdout.contains("----+"),
        "`-h host` was treated as a usage request and shown verbatim, got {stdout:?}"
    );
}

/// A Node tool that is not on PATH runs through its package runner, and rtk
/// puts its own `--` between the two (`npx --no-install -- playwright …`).
/// The help guard must read past that `--`: `rtk playwright test --help` is
/// still a usage request, not a test run for the filter to model.
#[test]
fn forwarded_help_through_the_package_runner_is_shown_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_tool(
        dir.path(),
        "npx",
        "echo \"Usage: playwright test [options] [test-filter...]\"; exit 0",
    );
    let out = rtk_with(dir.path(), &["playwright", "test", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("Usage: playwright test"),
        "usage must reach stdout verbatim, got {stdout:?}"
    );
    assert!(
        stderr.trim().is_empty(),
        "usage was handed to the filter, got stderr {stderr:?}"
    );
}
