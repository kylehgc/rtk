//! Fork issue #74 proof tests — `rtk git log` fidelity claims.
//!
//! Both fixes guard the same failure shape: upstream's commit-summary filter
//! caps each commit's body at 3 lines, and drops what does not fit. That is
//! fine for a commit message body, but silently eats *diff content* the user
//! explicitly asked for. Portable: std + `tempfile` only — `tempfile` is a
//! regular (non-dev) dependency in both the fork's and upstream's Cargo.toml,
//! so any checkout has it. Follows the scratch-repo + explicit user.name/
//! user.email isolation pattern from `tests/git_machine_output_test.rs` and
//! `tests/guard_integration_test.rs`.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn git_in_dir(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git_in_dir(dir.path(), &["init", "-q", "-b", "main"]);
    git_in_dir(dir.path(), &["config", "user.email", "t@t.t"]);
    git_in_dir(dir.path(), &["config", "user.name", "t"]);
    git_in_dir(dir.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
    dir
}

fn rtk_stdout_in_dir(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .env("LC_ALL", "C")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn rtk");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// #74 / d88c1f4 ("preserve patch output from log commands").
///
/// `-p` must return the real diff, not the 3-line-capped commit summary.
/// The fixture commit adds 8 lines to a new file: the diff has 5 header
/// lines (`diff --git`, `new file mode`, `index`, `---`, `+++`) before the
/// `@@` hunk marker even starts, so a body cap of 3 would discard the hunk
/// header and every added line. Healed 2026-09-01: upstream `0baf07e` /
/// `ca89767` route `-p` through raw passthrough too, so this is now a
/// regression guard that passes on both sides, not a fork claim.
#[test]
fn git_log_patch_preserves_diff_hunks() {
    let dir = init_git_repo();

    let mut content = String::new();
    for i in 1..=8 {
        content.push_str(&format!("PROOF_PATCH_LINE_{i:02}\n"));
    }
    std::fs::write(dir.path().join("proof_patch.txt"), content).expect("write fixture");
    git_in_dir(dir.path(), &["add", "proof_patch.txt"]);
    git_in_dir(
        dir.path(),
        &["commit", "-q", "-m", "add proof patch fixture"],
    );

    let out = rtk_stdout_in_dir(dir.path(), &["git", "log", "-p", "-1"]);

    assert!(
        out.contains("@@ -0,0 +1,8 @@"),
        "hunk header missing from patch output:\n{out}"
    );
    for i in 1..=8 {
        let marker = format!("PROOF_PATCH_LINE_{i:02}");
        assert!(
            out.contains(&marker),
            "diff content line {marker} dropped from patch output:\n{out}"
        );
    }
}

/// #74 / 791359c ("keep every commit in git log --stat output").
///
/// Every commit `git log --stat` returns must survive RTK's filter, diffstat
/// and all. Each fixture commit touches 3 files, so its diffstat is 4 lines
/// (3 file lines + the "N files changed" summary) — one more than the
/// filter's 3-line body cap, and enough to fully evict a commit header when
/// the compact path parses it: a trailing `---END---` marker leaves this
/// commit's diffstat heading the *next* commit's block, so the block's first
/// (diffstat) line gets misread as that next commit's header, and the real
/// header — now demoted to an ordinary body line — gets pushed out once the
/// remaining 3 diffstat lines alone fill the cap. Healed 2026-09-01: upstream
/// `ca89767` sends `--stat` (and every other diff-shaped flag) through raw
/// passthrough, so the compact parser never sees a diffstat and this test
/// passes on both sides. Kept as a regression guard for that routing.
#[test]
fn git_log_stat_keeps_every_commit() {
    let dir = init_git_repo();

    for i in 1..=15 {
        for file in ["a.txt", "b.txt", "c.txt"] {
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.path().join(file))
                .expect("open fixture file");
            writeln!(f, "{file}_{i:02}").expect("append fixture line");
        }
        git_in_dir(dir.path(), &["add", "a.txt", "b.txt", "c.txt"]);
        git_in_dir(
            dir.path(),
            &["commit", "-q", "-m", &format!("PROOF_STAT_{i:02}")],
        );
    }

    // No -N: `--stat` takes the raw passthrough path, which applies no
    // commit limit, so all 15 commits are expected back.
    let out = rtk_stdout_in_dir(dir.path(), &["git", "log", "--stat"]);

    for i in 6..=15 {
        let marker = format!("PROOF_STAT_{i:02}");
        assert!(
            out.contains(&marker),
            "commit {marker} dropped from `git log --stat` output:\n{out}"
        );
    }
}
