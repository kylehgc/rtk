# Rust Patterns — RTK Development Rules

RTK-specific Rust idioms and constraints. Applied to all code in this repository.

## Non-Negotiable RTK Rules

These override general Rust conventions:

1. **No async** — Zero `tokio`, `async-std`, `futures`. Single-threaded by design. Async adds 5-10ms startup.
2. **No `unwrap()` in production** — Use anyhow's `.context("description")?` everywhere. Tests: use `expect("reason")`.
3. **Lazy regex** — Fixed patterns reused across calls go in `LazyLock<Regex>` (recompiling per call kills performance); runtime-dependent patterns stay local. `.unwrap()` in a `LazyLock` initializer is an established RTK pattern — a bad regex literal is a programming error caught at first use.
4. **Fallback pattern** — If a filter fails, execute the raw command unchanged. Never block the user; never silently swallow errors (`Err(_) => {}` means the user gets no output).
5. **Exit code propagation** — `std::process::exit(code)` if the underlying command fails; returning early without it makes CI/CD think the command succeeded.

## Fallback pattern (mandatory for all filters)

```rust
pub fn run(args: MyArgs) -> Result<()> {
    let output = execute_command("mycmd", &args.to_cmd_args())
        .context("Failed to execute mycmd")?;

    let filtered = filter_output(&output.stdout)
        .unwrap_or_else(|e| {
            eprintln!("rtk: filter warning: {}", e);
            output.stdout.clone()  // Passthrough on failure
        });

    tracking::record("mycmd", &output.stdout, &filtered)?;
    print!("{}", filtered);

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    Ok(())
}
```

## Module Structure

Every `*_cmd.rs` follows this order: imports → args struct → lazy regexes → public `pub fn run(args) -> Result<()>` entry point → private filter functions → `#[cfg(test)] mod tests` (always present, with a local `count_tokens` helper and savings tests).

## Anti-Patterns (RTK-Specific)

| Pattern | Problem | Fix |
|---------|---------|-----|
| Fixed `Regex::new()` in hot function | Recompiles every call | `LazyLock<Regex>` |
| `unwrap()` in production | Panic breaks user workflow | `.context()?` |
| `tokio::main` or `async fn` | +5-10ms startup | Blocking I/O only |
| Silent match `Err(_) => {}` | User gets no output | Log warning + fallback |
| `println!` in filter path | Debug artifact in output | Remove or `eprintln!` |
| Returning early without exit code | CI/CD thinks command succeeded | `std::process::exit(code)` |
| `clone()` of large strings | Extra allocation in hot path | Borrow with `&str` |
