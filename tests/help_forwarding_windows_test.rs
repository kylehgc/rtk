//! Windows twin of the `--help` cases in `stderr_only_failure_test.rs` (which is
//! `#![cfg(unix)]`): a forwarded `--help` on a captured tool is shown as the
//! tool prints it, through a `.cmd` shim found via PATH.
#![cfg(windows)]
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn fake_cmd_tool(dir: &Path, name: &str, body: &str) {
    fs::write(
        dir.join(format!("{name}.cmd")),
        format!("@echo off\r\n{body}\r\n"),
    )
    .expect("write fake tool");
}

fn rtk_with(dir: &Path, args: &[&str]) -> Output {
    let path = format!(
        "{};{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(args)
        .env("PATH", path)
        .env("RTK_DB_PATH", dir.join("rtk-test.db"))
        .output()
        .expect("run rtk")
}

#[test]
fn forwarded_help_on_a_captured_tool_is_shown_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_cmd_tool(
        dir.path(),
        "wget",
        "echo Usage: wget [OPTION]... [URL]...\r\necho Try --help for more options.\r\nexit /b 0",
    );
    let out = rtk_with(dir.path(), &["wget", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("Usage: wget [OPTION]"),
        "usage must reach stdout verbatim, got {stdout:?}"
    );
    assert!(
        !stdout.to_lowercase().contains("fail") && !stdout.contains("rtk"),
        "no filter summary under the usage, got {stdout:?}"
    );
}

#[test]
fn forwarded_help_exits_with_the_tools_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    fake_cmd_tool(dir.path(), "wget", "echo usage: wget\r\nexit /b 129");
    let out = rtk_with(dir.path(), &["wget", "--help"]);
    assert_eq!(out.status.code(), Some(129));
}
