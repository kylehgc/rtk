# Conflicted Sync — 2026-08-12 (branch `sync/stream-lossy-utf8`)

Sync of `upstream/develop` (head `1989899`, the merge of upstream PR #2997
"fix/utf8-line-drop-in-stream-filters") into fork `develop`. One conflict, in
`src/core/stream.rs`. Resolved on a topic branch via fork PR, per the Conflicted
Sync policy (CONTEXT.md).

## Fork work removed

- The fork's earlier **pre-adoption of upstream #2997** (`c35968f`, Chris Brown /
  albatrossflyon-coder): the original `read_lines_lossy` implementation using a
  manual `read_until` loop in `std::iter::from_fn`.
- Fork amendment `bfce39e` (kylehgc, "log stream read errors instead of silently
  stopping"): the `eprintln!` on read error added on top of that adoption.

## Why upstream won

Upstream merged the same PR #2997, but with a review-feedback commit (`ae5d1ae`)
the fork's early adoption predated:

- Implementation reworked to `BufReader::split(b'\n').filter_map(...)` — simpler
  and equivalent (split strips `\n`; `\r` popped explicitly).
- The fork amendment's error logging is incorporated verbatim
  (`eprintln!("rtk: stream read error: {}", e)`), so `bfce39e` is fully subsumed.
- Adds a `FailingReader` regression test (`test_read_lines_lossy_stops_on_io_error_without_panicking`)
  the fork didn't have.

Both conflict hunks resolved to upstream's side verbatim. The fork's versions were
duplicates, not additive.

## Fork work surviving (verified additive)

- `decode_process_output` in `exec_capture` / `exec_capture_stdin` (adoption
  `13cf995`, guy oron — Windows console code page decoding). Verified absent from
  `upstream/develop:src/core/utils.rs`; this is now the only fork delta in
  `src/core/stream.rs`.

## Fork tests updated

None. Upstream's tests adopted verbatim; all pre-existing fork tests passed
unchanged.

Quality gate: fmt + clippy clean, `cargo test --all` 2704 passed / 0 failed.
