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

mod common;

/// Run `cmd` in the repo root, returning raw stdout bytes.
fn run_cmd(mut cmd: Command, args: &[&str]) -> Option<Vec<u8>> {
    let out = cmd
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
        .ok()?;
    Some(out.stdout)
}

/// Native git, isolated from the developer's configuration the same way the
/// rtk child's git is, so both sides of each comparison read the same config.
fn run(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut cmd = Command::new(program);
    common::isolate_git(&mut cmd);
    run_cmd(cmd, args)
}

/// The rtk binary, with its data redirected away from the developer's own.
fn run_rtk(args: &[&str]) -> Option<Vec<u8>> {
    run_cmd(common::rtk_command(), args)
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
        let rtk = run_rtk(&[&["git"], *args].concat()).expect("rtk binary must run");

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
fn human_facing_git_output_is_still_filtered() {
    if !git_available() {
        eprintln!("skipping: not a git checkout or git not on PATH");
        return;
    }

    // The counterweight: widening the machine-output guard until everything
    // passes through would make the byte-exactness test above pass trivially
    // while destroying the filtering rtk exists for.
    //
    // The assertion is "not byte-identical", not "fewer bytes". rtk's compact
    // diff adds per-file headers and annotations, so on a large diff its output
    // is legitimately *longer* than raw git — a smaller-than check passes on a
    // one-file commit and fails on a merge, which is repo state, not behaviour.
    let native = run("git", &["status"]).expect("git status");
    let rtk = run_rtk(&["git", "status"]).expect("rtk git status");
    assert_ne!(
        rtk, native,
        "bare `git status` must still be filtered, not passed through"
    );

    // --no-color only suppresses ANSI; it must not buy a passthrough.
    let native_nc = run("git", &["diff", "--no-color", "HEAD~1..HEAD"]).expect("git diff");
    if !native_nc.is_empty() {
        let rtk_nc = run_rtk(&["git", "diff", "--no-color", "HEAD~1..HEAD"]).expect("rtk git diff");
        assert_ne!(
            rtk_nc, native_nc,
            "`git diff --no-color` must stay filtered, not passed through"
        );
    }
}
