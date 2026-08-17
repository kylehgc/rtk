# Search Strategy — RTK Codebase Navigation

Priority order: **Grep** (known symbols/strings) → **Glob** (find modules by name) → **Read** (only after locating the right file) → Explore agent (broad research, last resort). Never use Bash `find`/`grep`/`rg` for search — use the dedicated tools.

## Where things live (non-obvious)

- `src/main.rs` — Clap `Commands` enum + routing; start here for any command.
- Command filters: `src/cmds/<ecosystem>/<cmd>_cmd.rs` (plus `git/git.rs`, `rust/runner.rs`, `cloud/container.rs`). `ls src/cmds` for the ecosystem list; each folder has a README.
- Shared helpers: `src/core/utils.rs` (`strip_ansi`, `truncate`, `execute_command`) — check before reimplementing.
- Tracking/metrics: `src/core/tracking.rs`; DB path from `tracking.database_path` in config, overridden by the `RTK_DB_PATH` env var.
- Config: `src/core/config.rs`; file at `~/.config/rtk/config.toml`; `rtk init` lives in `src/hooks/init.rs`.
- TOML filter DSL: engine in `src/core/toml_filter.rs`; filter files in `~/.config/rtk/filters/` (global) or `.rtk/filters/` (project); shipped defaults in `src/filters/`.

## Dependency rules

- Before adding a crate, Grep `Cargo.toml` — it may already be there.
- Async is forbidden: if `tokio|async-std|futures|async fn` matches anything new, that's a bug.
