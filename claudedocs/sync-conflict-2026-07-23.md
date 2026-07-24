# Conflicted Sync — 2026-07-23 (merge `e32bb38`)

Sync of `upstream/develop` (through upstream PRs for safe pipeline rewriting and
piped grep/rg streaming, head commit `1c5a23c`) into fork `develop`. One conflict, in
`src/discover/registry.rs`. Resolved and pushed directly to `develop` — this predates the
Conflicted Sync PR policy (CONTEXT.md) and is what motivated it.

## Fork work removed

- `PIPE_INCOMPATIBLE` const + `is_pipe_incompatible()` in `src/discover/registry.rs`:
  a blanket skip that left `find`/`fd`/`grep`/`rg`/`ls`/`head`/`tail`/`cat` segments
  unrewritten anywhere in a pipeline, so rtk's reshaped output (summary headers, elision
  markers, size columns) never fed a pipe consumer (#439 lineage).

## Why upstream won

Upstream's redesign solves the same problem structurally and more precisely:

- Pipelines now keep **all producer stages raw** — only the final stage is ever rewritten
  (`analyze_pipeline` / `rewrite_pipeline_final_stage`), so filtered output can never feed
  another pipe stage, for any command, not just the eight listed.
- The final stage is rewritten only for commands flagged `pipeline_final_safe`, and
  `grep`/`rg` as final stage now stream piped stdin faithfully (upstream's
  "fix(search): stream piped grep and rg output").
- `|&` is tokenized as a single pipe operator and such pipelines stay raw; pipeline-final
  `wc` stays raw.

The fork's guard was a subset of this behavior; keeping both would have been dead code.

## Fork tests updated

- `test_rewrite_head_pipe_skipped` → renamed `test_rewrite_head_pipe_producer_stays_raw`:
  `head -20 src/main.rs | grep use` now rewrites to `head -20 src/main.rs | rtk grep use`
  (producer still raw — the original concern — final grep safely rewritten).
- All other fork pipe tests (grep/wc raw, sudo/env prefixes, absolute paths, `&&`
  segments) passed unchanged under the upstream mechanism.

Quality gate: fmt + clippy clean, `cargo test --all` 2512 passed / 0 failed.
