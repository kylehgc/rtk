# Conflicted Sync — 2026-08-01 (branch `sync/upstream-2026-08-01`)

Sync of `upstream/develop` (security hardening: owner-only data dirs/files —
history db, tee logs, audit log, warn marker; head commit `e0ffd40`) into fork
`develop`. Two conflicts. First sync routed through a topic branch + fork PR per
the Conflicted Sync policy (CONTEXT.md).

## Conflicts and resolutions

### `src/hooks/hook_check.rs` — warn-marker write

- Fork HEAD: `touch_warn_marker(&marker)` — fork-only helper writing a **non-empty**
  payload (`b"1"`), because on Windows writing 0 bytes to an already-empty file may
  not update the mtime, breaking the once-per-day warn rate limit.
- Upstream: inlined `create_private_dir(marker.parent()?)` + `fs::write(&marker, b"")`.
- Verified fork-only: `git show upstream/develop:src/hooks/hook_check.rs` has no
  `touch_warn_marker`. Upstream's change is orthogonal (dir permissions), and its
  empty-payload write would reintroduce the Windows mtime bug the helper fixes.
- **Resolution**: kept the fork helper (genuinely additive), migrated its
  `fs::create_dir_all` to upstream's `create_private_dir` so the marker's parent dir
  gets upstream's owner-only hardening — additive code adopts upstream's current style.

### `src/core/utils.rs` — test module tail

- Both sides appended new tests at the same location: fork's Windows encoding tests
  (`decode_process_output` / `codepage_to_encoding` — verified absent upstream) vs
  upstream's `create_private_dir` / `restrict_file` permission tests.
- **Resolution**: kept both blocks, upstream's after the fork's. No code overlap.

## Fork work removed

None. No fork code was superseded by this sync.

## Fork tests updated

None. Fork's `touch_warn_marker` tests (non-empty payload, mtime refresh) still
encode the intent and pass unchanged.

Quality gate: fmt + clippy clean, `cargo test --all` 2586 run / 2578 passed /
8 ignored / 0 failed (x64 host toolchain, Windows).
