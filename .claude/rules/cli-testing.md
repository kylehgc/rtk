# CLI Testing Strategy

Testing rules for RTK CLI tool development.

## Unit Testing (🔴 Critical — all filter and output-format changes)

Plain `#[cfg(test)] mod tests` colocated in the same file as the filter; assert directly against expected output with `assert_eq!`/`assert!`.

### Fixture strategy

Two patterns coexist, pick based on what the filter needs:

1. **Inline literal strings** (most common for `src/cmds/**` unit tests) — a small representative string built in the test body. Good for quick coverage of a specific format/edge case.
2. **Real captured fixtures via `include_str!`** — when the raw output is large or format-sensitive enough that inline strings would be unreadable or drift from reality. `src/cmds/jvm/mvn_cmd.rs` is the reference example (23+ fixtures). Fixtures live in `tests/fixtures/` and are REAL captured command output (e.g. `mvn test > tests/fixtures/mvn_test_example_raw.txt`), never synthetic data.

Every new filter covers the common case plus at least one edge case (empty input, error output). Also worth covering when relevant: malformed input (best-effort output or passthrough — never panic), unicode, ANSI codes (strip or preserve, don't break).

## Token Accuracy (🔴 Critical — all filter implementations)

Every filter MUST verify its savings claim with a test comparing `count_tokens` (`split_whitespace().count()`) of input vs filtered output.

- **≥60% savings is the single enforced floor and a release blocker.** There is no per-filter threshold table.
- Don't assert specific per-command percentages unless verified against that filter's own fixtures — invented numbers rot immediately.
- The `count_tokens` helper is duplicated per test module — there is no shared `tests/common/mod.rs`.

## Cross-Platform (🔴 Critical — shell escaping / command execution changes)

RTK must work on macOS (zsh), Linux (bash), Windows (PowerShell); quoting and path separators differ. Use `#[cfg(target_os = ...)]` for platform-dependent assertions — never test only the current platform. Test macOS + Linux locally; trust CI for Windows.

## Integration Tests (🟡 Important)

Top-level `tests/*.rs` files (not colocated with `src/`), exercising cross-cutting behavior; several draw on `tests/fixtures/`. Real-process tests are `#[ignore]`-tagged, need the installed binary (`cargo install --path .`), and run with `cargo test --ignored` — always before a release, and after filter or hook changes.

## Performance (🟡 Important)

| Metric | Target | Verification |
|--------|--------|--------------|
| Startup time | <10ms | `hyperfine 'rtk <cmd>' --warmup 3` |
| Memory usage | <5MB | `/usr/bin/time -l` (macOS) / `-v` (Linux) |
| Binary size | <5MB | `ls -lh target/release/rtk` |

Benchmark before/after any performance-relevant change (raw binary vs `target/release/rtk`); investigate if startup grows >2ms.

## Checklist (adding/modifying a filter)

- [ ] Unit test in the filter's `#[cfg(test)]` block (inline string, or `include_str!` fixture once output gets large/format-sensitive)
- [ ] Token accuracy test (≥60% savings) with a locally-defined `count_tokens`
- [ ] Cross-platform escaping test if applicable
- [ ] `cargo test --all` passes; `cargo test --ignored` before release
- [ ] `hyperfine` startup check (<10ms)
