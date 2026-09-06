# Conflicted Sync — 2026-09-05 (branch `sync/upstream-2026-09-05`)

Sync of `upstream/develop` (head `e53ec1c`, tag `dev-0.49.0-rc.407`; 37 commits, 31 non-merge,
since the 2026-09-03 sync) into fork `develop`. Two conflicts: `.gitattributes` and
`scripts/benchmark.sh`. Resolved on a topic branch via fork PR, per the Conflicted Sync policy
(CONTEXT.md): upstream wins on everything it covers, only proven-additive fork code survives,
and surviving code takes upstream's current shape.

Incoming upstream work that collided with, or now constrains, fork code:

- **`a083e05`** (alvins82) "fix(ci): keep hook payload line endings stable" — adds
  `hooks/**/*.ts -text` to `.gitattributes`.
- **`d952a6b`** (Adrien Eppling, PR #3863) "fix(benchmark): skip find's disclosure note and tee
  pointer when counting names" — upstream's own fix for the bug fork PR #158 fixed.
- **golangci-lint on Go 1.27**: `4db7b15` installs golangci-lint v2 in CI instead of the
  abandoned v1 line, `0731055` gives the Go benchmark fixture something for the linters to find,
  `c730255` reads a v-prefixed golangci-lint version, `67007fe` swaps the synthetic
  `golangci_v2_json.txt` fixture for captured `golangci_v2_*_raw.json` ones (all KuSh).
- **Pi extension hash allowlist** (alvins82, in the OMP lineage): `KNOWN_PI_PLUGIN_HASHES` in
  `src/hooks/init.rs` (`ad51059`) plus `test_all_git_pi_plugin_revisions_are_allowlisted`
  (`157d5c2`), which walks
  `git rev-list HEAD --full-history -- hooks/pi/rtk.ts` and requires every revision's hash to be
  allowlisted. This did not conflict; it failed the gate.

The rest auto-merged: Oh My Pi (OMP) support and its hardening rounds (alvins82), the lexer
`coalesce_words` / `shell_split` rebuild and the lone-CR fixes (KuSh), sudo passthrough in the
rewriter (patrick), discover docs. No change to `Cargo.toml`, `build.rs`, or `Cargo.lock` in the
range.

## `.gitattributes`

Upstream's block taken verbatim (`hooks/**/*.ts -text` with its comment). The fork's
`*.sh text eol=lf` rule (`a45d0d9`) survives: `git show upstream/develop:.gitattributes | grep sh`
is blank, so it is additive. Both sides were additions at the same spot; the file is now
upstream's plus the fork's rule, its two comment lines and a blank separator.

## `scripts/benchmark.sh`

**Upstream's file verbatim.** Two fork changes are gone, both duplicates:

- `e2e11b7` (fork PR #158, upstream #3861) tightened the `count_find_names` /
  `count_find_total` skip patterns to `^\.\.\. \([0-9]+\+? filtered\)$` and
  `^\[see remaining: tail -n \+[0-9]+ .*\]$`. Upstream merged Adrien Eppling's `d952a6b`
  for the same lines with the looser `^\.\.\. \(` / `^\[see remaining: ` forms. Same bug,
  upstream's fix; the fork's goes. Added to `HEALED_SHAS` in `scripts/fork-delta.sh`.
- `1c0437d` (fork PR #157) put `2>&1` on the rtk side of the four Go rows so a failing
  golangci-lint v1 on Go 1.27 read as a failure instead of "filter returned empty output". It
  auto-merged, and `git checkout --theirs` dropped it deliberately: upstream solved the same
  symptom its own way (golangci-lint v2 in CI, a fixture with findings, the filter reading the
  v-prefixed version). `fix(cicd)` is an excluded delta scope, so no `HEALED_SHAS` entry.

`.github/workflows/ci.yml` auto-merged correctly: the fork's push trigger, semgrep-baseline
fallback and `cargo install cargo-audit --locked` are all still there (Fork Infrastructure,
upstream has none of them); upstream's `golangci-lint/v2` install and `fetch-depth: 0` on the
test job are taken.

## `hooks/pi/rtk.ts` and the hash allowlist

The fork's Pi extension is upstream's plus the `--` terminator before the rewritten command
(`8253401`, fork PR #36, which extended upstream #2475's Claude-hook fix to the Hermes, OpenCode
and Pi callers). It auto-merged. Proof it is still additive, on real binaries:

```
$ rtk rewrite --help          # what upstream's ["rewrite", cmd] sends for a "--help" command
Rewrite a raw command to its RTK equivalent (single source of truth for hooks)
...                            exit 0  — the help text would become the rewritten command
$ rtk rewrite -- --help
                               exit 1  — no rewrite, command passes through
```

`allow_hyphen_values` on `Rewrite` does not stop clap from answering `--help`, so the fork's
change stays. The cost is new this sync: upstream's allowlist recognises only upstream blobs,
and `is_known_stock_pi_plugin` gates `rtk init --agent pi --uninstall` (and the OMP
uninstall/status paths). A Pi extension installed by any fork release hashes to a value
upstream never saw.

**Fork-authored amendment** (`src/hooks/init.rs`, 3 entries appended to
`KNOWN_PI_PLUGIN_HASHES`, commented as fork revisions):

| hash | content | fork revisions |
|---|---|---|
| `dfe5632d…` | upstream `eb56dd08` + `--` | `8253401` through the 2026-08 syncs |
| `46f41389…` | upstream `3eb16108` + `--` | `ee9e9d9` (2026-08-29 sync) through `develop` |
| `8a496cff…` | upstream `5e80e811` + `--` | this merge |

Bases verified by diff: `git diff f6a5451 8253401 -- hooks/pi/rtk.ts` is the `--` hunk alone.
Every other revision the history test walks hashes to an upstream entry. The list stays
append-only, as upstream's comment requires.

## Repro, real binaries

Before = release build of the merged tree without the amendment; after = with it. Local scope
(`./.pi/extensions/rtk.ts`), `RTK_DB_PATH` scratch. "stock" is `develop`'s `hooks/pi/rtk.ts`,
i.e. what a fork release installs; "modified" is the same file plus a `// user modification`
line.

```
$ rtk init --agent pi --uninstall            # stock
before: rtk: Pi extension at .pi\extensions\rtk.ts contains RTK content that does not match
        the stock extension. Remove the file manually.                    exit 1, file kept
after:  RTK uninstalled (Pi):
          - Pi extension: .pi\extensions\rtk.ts
        Restart pi to apply changes.                                      exit 0, file removed

$ rtk init --agent pi --uninstall            # modified
before: (refused, as above)                                               exit 1, file kept
after:  (refused, as above)                                               exit 1, file kept
```

The first gate run is the unit-level "before": `test_known_pi_plugin_hashes_are_sha256` failed
with `current Pi extension hash 8a496cff… is missing`, and
`test_all_git_pi_plugin_revisions_are_allowlisted` with
`Pi extension revision 0728c555… has unallowlisted hash dfe5632d…`.

## Review round

Independent read-only reviewer pass over the staged resolution against this writeup, eleven
verification items. All passed: no markers or mojibake beyond upstream's own test literals and
a genuine fixture; `.gitattributes` and `ci.yml` are upstream's plus the documented fork hunks
only; `benchmark.sh` blob-identical to upstream; `hooks/pi/rtk.ts` differs by the `--` hunk
alone; the `init.rs` diff against upstream is byte-for-byte the fork's pre-existing
PowerShell-matcher divergence plus the allowlist addition, nothing lost; all three fork hashes
recomputed independently, and every one of the 62 revisions the history test walks (fork and
upstream) hashes to an allowlisted entry; upstream's `Rewrite` variant carries no
`disable_help_flag` or `--` handling, so the terminator is not redundant. No findings. Two
nits taken: the allowlist constant is `ad51059`, the history test `157d5c2`; the
`.gitattributes` delta is four lines counting the blank separator. Noted, not changed: the
`7db14a3` `HEALED_SHAS` note ("upstream merged its PR #1350") names the issue, not the closed
PR #2475 — the fix reached upstream under a different SHA, which is what the patch-id layer
records.

## Quality gate

x64 host toolchain (`scripts/win-dev-env.ps1`): `cargo fmt --all` clean · `cargo clippy
--all-targets` 0 warnings · `cargo test --all` 3288 unit tests passed, 0 failed, 8 ignored; all
integration suites green, including upstream's new `omp_init_test`. `git diff --cached --check`
clean.

## Fork Delta

`scripts/fork-delta.sh` regenerated: 60 fixes (was 59 on the page; `e2e11b7` healed, and the two
`#159` stderr commits that the page had not yet picked up are now listed). New `HEALED_SHAS`
entry for `e2e11b7`.

## Deliberately not done

- **Upstream #3861** (fork-authored, open) carries the now-superseded benchmark-find half next
  to the still-valid `cargo-audit --locked` half. Trimming it to the audit change and replying
  is #3861 work, a separate ask.
- **The `--` terminator has no upstream PR.** Upstream #2475 is closed unmerged, and `8253401`
  is fork-authored, so this is an Original fix under CONTEXT.md and it is not in the #84
  ledger. Every future upstream revision of `hooks/pi/rtk.ts` will add one hash here until
  upstream carries the terminator itself. Flagged, not started.
- The `rules.rs` overrides recorded in the 2026-09-01 writeup are unchanged.
