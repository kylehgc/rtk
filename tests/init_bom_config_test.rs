//! `rtk init -g --auto-patch` must tolerate a UTF-8 BOM in a hand-edited
//! settings.json. Notepad and PowerShell 5.1's `Out-File -Encoding utf8`
//! both prepend one by default. Upstream's JSON reader in
//! `patch_settings_json_command` rejects a BOM'd file outright ("expected
//! value at line 1 column 1"), aborting the hook install and blaming the
//! user's otherwise-valid JSON.
//!
//! Isolated via `CLAUDE_CONFIG_DIR`, which `resolve_claude_dir()` honors as
//! an override ahead of `$HOME`/`%USERPROFILE%` — this never touches the
//! developer's real `~/.claude`. `--dry-run` keeps every other `init` side
//! effect (RTK.md, CLAUDE.md, the global filters.toml template, legacy-hook
//! migration) a printed message instead of a write, and is applied *after*
//! the BOM'd file is read and parsed, so it doesn't skip the code path under
//! test — it just keeps the test from writing anywhere at all, including
//! paths `CLAUDE_CONFIG_DIR` can't reach (`dirs::config_dir()`,
//! `dirs::home_dir()`, both real and unoverridable via env on Windows).

use std::process::Command;

#[test]
fn init_dry_run_tolerates_bom_prefixed_settings_json() {
    let claude_dir = tempfile::tempdir().expect("tempdir");
    let settings_path = claude_dir.path().join("settings.json");

    // BOM (U+FEFF) + a pre-existing user key that must survive being
    // reported back, exactly what Notepad / PowerShell 5.1 write.
    std::fs::write(&settings_path, "\u{feff}{\"foo\": 1}").expect("write settings.json fixture");

    let out = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["init", "-g", "--auto-patch", "--dry-run"])
        .current_dir(claude_dir.path())
        .env("CLAUDE_CONFIG_DIR", claude_dir.path())
        .env("RTK_TELEMETRY_DISABLED", "1")
        .output()
        .expect("spawn rtk init");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "rtk init -g --auto-patch --dry-run must not abort on a BOM-prefixed \
         settings.json\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stderr.contains("Failed to parse"),
        "the BOM must not be reported as a JSON parse failure: {stderr}"
    );
    assert!(
        stdout.contains("would patch settings.json"),
        "dry-run must reach the patch step (proving the BOM'd file parsed), \
         not bail out early: {stdout}"
    );

    // dry-run must not have touched the fixture on disk.
    let after = std::fs::read_to_string(&settings_path).expect("read back settings.json");
    assert_eq!(
        after, "\u{feff}{\"foo\": 1}",
        "dry-run must leave the settings.json fixture untouched"
    );
}
