//! Portable proof tests for fork issue #73 (hook/rewrite fixes).
//!
//! `rtk hook claude` is pure stdin -> stdout: it reads a Claude Code PreToolUse
//! JSON envelope on stdin and prints a rewrite decision as JSON on stdout. These
//! tests drive the real binary through that protocol and inspect the JSON, so
//! they are green on this fork and red on upstream rtk-ai/rtk (verified by
//! reading upstream's `src/hooks/hook_cmd.rs` — see commit messages c8d6e28 and
//! e93cde8 for the exact behavioral gap each test proves).
//!
//! Permission-rule loading (`src/hooks/permissions.rs`) reads both the project
//! root's `.claude/settings*.json` (found by walking up from the process's cwd)
//! and the real user's `~/.claude/settings*.json`. To keep the verdict
//! deterministic across CI and any developer's machine — regardless of what
//! permission rules that machine happens to have configured — every test runs
//! with `HOME`/`USERPROFILE` pointed at a fresh empty temp dir and `cwd` set to
//! that same temp dir (never inside a git checkout), so no rule file is ever
//! found and the verdict is always `Default`.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run `rtk hook claude` with `stdin` as input, isolated from real permission
/// config: fresh `HOME`/`USERPROFILE`, cwd outside any git repo.
fn run_claude_hook(stdin: &str) -> String {
    let home = tempfile::tempdir().expect("create isolated home tempdir");

    let mut child = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["hook", "claude"])
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rtk hook claude");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");

    let out = child.wait_with_output().expect("wait for rtk hook claude");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// c8d6e28 "fix(hook): accept current Claude tool input keys" — Claude Code's
/// current PreToolUse payload nests the tool call under `input`, not
/// `tool_input`. Upstream's `process_claude_payload` only ever reads
/// `/tool_input/command` (see upstream `src/hooks/hook_cmd.rs`, `process_claude_payload`):
/// a payload using `input` hits `None` there and produces PayloadAction::Ignore
/// — no stdout at all, so the command is never rewritten. This fork's
/// `claude_payload_input` checks `tool_input` then `input`, so the same
/// payload still gets rewritten and the sibling fields (timeout, description)
/// survive the round-trip.
#[test]
fn claude_hook_rewrites_current_input_key_shape() {
    let payload = serde_json::json!({
        "tool": "Bash",
        "input": {
            "command": "git status",
            "timeout": 30000,
            "description": "Check repo status"
        }
    })
    .to_string();

    let stdout = run_claude_hook(&payload);
    assert!(
        !stdout.is_empty(),
        "expected a rewrite decision on stdout for an `input`-shaped payload, got none \
         (upstream only reads `tool_input` and silently ignores this payload)"
    );

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    let updated = &v["hookSpecificOutput"]["updatedInput"];

    assert_eq!(updated["command"], "rtk git status");
    assert_eq!(updated["timeout"], 30000);
    assert_eq!(updated["description"], "Check repo status");
}

/// e93cde8 "fix(hook): emit ask decision for Claude rewrites" — when no
/// permission rule matches (the `Default` verdict, which is what every
/// unconfigured environment gets), the rewritten command must still make
/// Claude Code prompt the user rather than silently running. Upstream's
/// `process_claude_payload` only ever inserts `permissionDecision` for the
/// `allow` case (see upstream `src/hooks/hook_cmd.rs`) — for `ask`/`Default`
/// it emits `updatedInput` with the rewrite but omits `permissionDecision`
/// entirely, so Claude Code's own default (which upstream's own comment in
/// the pre-fix test described as "default-to-ask semantics") is the only
/// thing standing between an unattested rewrite and auto-run. This fork
/// explicitly writes `"permissionDecision": "ask"` for that case.
#[test]
fn claude_hook_emits_ask_decision_for_default_verdict() {
    let payload = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "git status" }
    })
    .to_string();

    let stdout = run_claude_hook(&payload);
    assert!(!stdout.is_empty(), "expected a rewrite decision on stdout");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    let hook = &v["hookSpecificOutput"];

    assert_eq!(
        hook["permissionDecision"], "ask",
        "Default-verdict rewrites must explicitly ask, not rely on an absent key \
         (upstream omits permissionDecision here entirely): got {hook}"
    );
    assert_eq!(hook["updatedInput"]["command"], "rtk git status");
}
