//! `rtk git` must hand machine-readable formats back byte-for-byte.
//!
//! The unit tests in `src/cmds/git/git.rs` only cover the flag predicates —
//! whether an arg list *counts* as machine output. They cannot catch the actual
//! defect this guards, which lives in the print path: swapping `print!` back to
//! `println!("{}", stdout.trim())` still satisfies every predicate test while
//! silently dropping the trailing newline and breaking `while read` loops.
//!
//! So these run the real binary against real git and compare bytes.

use std::path::Path;
use std::process::Command;

/// Run a program in the repo root, returning raw stdout bytes.
fn run(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new(program)
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
        .ok()?;
    Some(out.stdout)
}

/// These compare against real `git` in this working tree. Skip rather than fail
/// where that isn't available, so the suite stays green off a git checkout.
fn git_available() -> bool {
    run("git", &["rev-parse", "--git-dir"]).is_some_and(|o| !o.is_empty())
}

#[test]
fn machine_readable_git_output_is_byte_identical_to_native() {
    if !git_available() {
        eprintln!("skipping: not a git checkout or git not on PATH");
        return;
    }

    // Each case must survive rtk untouched — same bytes, trailing newline included.
    let cases: &[&[&str]] = &[
        &["status", "--porcelain"],
        &["status", "--porcelain=v2"],
        &["status", "-z"],
        &["log", "--format=%H", "-3"],
        &["log", "--pretty=format:%H", "-3"],
        &["log", "-z", "--name-only", "-3"],
        &["diff", "--name-only", "HEAD~1..HEAD"],
        &["diff", "--name-status", "HEAD~1..HEAD"],
        &["diff", "--numstat", "HEAD~1..HEAD"],
        &["diff", "--raw", "HEAD~1..HEAD"],
    ];

    for args in cases {
        let Some(native) = run("git", args) else {
            continue;
        };
        let rtk = run(env!("CARGO_BIN_EXE_rtk"), &[&["git"], *args].concat())
            .expect("rtk binary must run");

        assert_eq!(
            String::from_utf8_lossy(&rtk),
            String::from_utf8_lossy(&native),
            "`rtk git {args:?}` diverged from native git ({} bytes vs {})",
            rtk.len(),
            native.len()
        );
    }
}

#[test]
fn human_facing_git_output_is_still_compacted() {
    if !git_available() {
        eprintln!("skipping: not a git checkout or git not on PATH");
        return;
    }

    // The counterweight: widening the machine-output guard until everything
    // passes through would make the byte-exactness test above pass trivially
    // while destroying the compaction rtk exists for.
    let native = run("git", &["status"]).expect("git status");
    let rtk = run(env!("CARGO_BIN_EXE_rtk"), &["git", "status"]).expect("rtk git status");

    assert!(
        rtk.len() < native.len(),
        "bare `git status` must still be compacted (rtk {} bytes vs native {})",
        rtk.len(),
        native.len()
    );

    // --no-color only suppresses ANSI; it must not buy a passthrough.
    let native_nc = run("git", &["diff", "--no-color", "HEAD~1..HEAD"]).expect("git diff");
    if !native_nc.is_empty() {
        let rtk_nc = run(
            env!("CARGO_BIN_EXE_rtk"),
            &["git", "diff", "--no-color", "HEAD~1..HEAD"],
        )
        .expect("rtk git diff");
        assert!(
            rtk_nc.len() < native_nc.len(),
            "`git diff --no-color` must stay compacted (rtk {} bytes vs native {})",
            rtk_nc.len(),
            native_nc.len()
        );
    }
}
