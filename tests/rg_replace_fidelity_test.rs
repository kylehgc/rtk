//! `rtk rg` must not let ripgrep's `-r`/`-R` (`--replace`) silently corrupt output.
//!
//! grep's `-r`/`-R` means "recursive" — ripgrep is already recursive by default and
//! reads the same bytes as `--replace <VALUE>` instead. An agent typing `rg -rn` out
//! of grep muscle memory gets every match silently rewritten to the literal string
//! "n", with no error. `strip_rg_replace` in `src/cmds/system/search.rs` strips
//! `-r`/`-R`/`--replace`/`--recursive` before the rg engine runs, but only for rg —
//! grep keeps `-r`/`-R` untouched, since there it really is recursive.
//!
//! A follow-up bug in the first fix: the letter-strip ran over every dash-prefixed
//! token in the flag list, including a value token that happens to start with `-r`
//! (e.g. a `--glob`/`-g` value like `-r*.rs`). That silently rewrote the value
//! itself (`-r*.rs` -> `-*.s`), changing which files got searched. The fix spares
//! any token that is the value of a preceding value-taking flag.
//!
//! These two commits exist nowhere in `upstream/develop`'s `search.rs`: it forwards
//! `-r`/`-R` to rg unchanged for every engine, so both bugs are just "the bug" there.
//! Green here / red there is the proof (see CONTEXT.md, "Proof suite").

use std::path::Path;
use std::process::Command;

fn rtk() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rtk"))
}

/// Real ripgrep, not the `rg` alias some environments shim over something else.
///
/// Two regimes, because "fail loudly, never skip" (issue #76) matters exactly
/// where a claim is being enforced. In the proof suite (`fork-proof.yml` sets
/// `RTK_PROOF_REQUIRE_RG=1` and installs ripgrep), a missing rg is a hard
/// failure — a silent skip there would record the claim as proven when nothing
/// ran. In ordinary `cargo test --all` (CI PR runs, bare dev checkouts), rg is
/// not guaranteed — GitHub runner images don't ship it — so we skip with a loud
/// message instead, same as this suite's cargo/git availability guards.
/// Returns false when the caller should return early.
fn require_rg() -> bool {
    let ok = Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        return true;
    }
    assert!(
        std::env::var_os("RTK_PROOF_REQUIRE_RG").is_none(),
        "rg (ripgrep) is not on PATH and RTK_PROOF_REQUIRE_RG is set. In the proof \
         suite a missing rg must fail, not skip — a silent skip would record the \
         claim as proven when nothing ran. Install ripgrep and re-run."
    );
    eprintln!("skipping: rg (ripgrep) not on PATH");
    false
}

fn write_file(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).expect("write fixture file");
}

fn rtk_rg_stdout(dir: &Path, args: &[&str]) -> String {
    let out = rtk()
        .arg("rg")
        .args(args)
        .arg(dir)
        .output()
        .expect("rtk rg must run");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// --- fix 1 (strip_rg_replace introduced): -r/-R must not reach rg as --replace ---

#[test]
fn rg_short_r_cluster_does_not_trigger_ripgrep_replace() {
    if !require_rg() {
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_file(dir.path(), "match.rs", "NEEDLE_TEXT here\n");

    // `-rn`: grep muscle memory for "recursive, with line numbers". Real ripgrep
    // parses `-rn` as `-r` with inline value "n" (--replace n), so an unfixed rtk
    // that forwards this literally gets ripgrep to rewrite every match to "n" —
    // confirmed against the real rg binary while writing this test:
    // `rg -rn NEEDLE_TEXT dir` prints `match.rs:n here`, not the real match.
    let stdout = rtk_rg_stdout(dir.path(), &["-rn", "NEEDLE_TEXT"]);

    assert!(
        stdout.contains("NEEDLE_TEXT here"),
        "the real matched line must survive `-rn` unmangled, got:\n{stdout}"
    );
    assert!(
        !stdout.contains(":n here"),
        "match text must not be rewritten to ripgrep's --replace value \"n\":\n{stdout}"
    );
}

// --- fix 2 (value-token carve-out): a value token starting with `-r` must survive ---

#[test]
fn rg_dash_prefixed_flag_value_survives_r_strip() {
    if !require_rg() {
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    // A real, dash-led filename (legal on every platform via direct fs::write) that
    // the glob `-r*.rs` is meant to match, plus two decoys: one that would only
    // match if the glob's value got letter-stripped into matching everything, and
    // one that must never match regardless.
    write_file(dir.path(), "-report.rs", "NEEDLE_TEXT here\n");
    write_file(dir.path(), "normal.rs", "NEEDLE_TEXT here\n");

    // `-rn` again (must still be stripped) plus `-g '-r*.rs'`: the glob's value is
    // a separate token that itself starts with `-r`. Confirmed against the real rg
    // binary: `rg -g '-r*.rs' NEEDLE_TEXT dir` matches only `-report.rs`; the
    // letter-stripped value `-*.s` matches nothing at all. If rtk's strip treats
    // this value token as its own flag cluster, the glob silently breaks and the
    // real match is lost.
    let stdout = rtk_rg_stdout(dir.path(), &["-rn", "-g", "-r*.rs", "NEEDLE_TEXT"]);

    assert!(
        stdout.contains("-report.rs") && stdout.contains("NEEDLE_TEXT here"),
        "the glob value `-r*.rs` must survive intact and match `-report.rs`, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("normal.rs"),
        "the glob must still exclude files it wasn't given for, got:\n{stdout}"
    );
    assert!(
        !stdout.contains(":n here"),
        "match text must not be rewritten to ripgrep's --replace value \"n\":\n{stdout}"
    );
}
