# Gotchas

Known pitfalls and edge cases. Referenced from AGENTS.md.

## Hashcat Output Routing

- Hashcat `event_log_warning` and `event_log_advice` go to **stdout**, not stderr -- only `event_log_error` goes to stderr. Hash parse errors are warnings, so they appear on stdout.
- `--status-json` only structures the periodic status display -- errors/warnings remain plain text regardless of output mode.
- v7.x changed per-hash error prefix from `Hashfile '...'` to `Hash parsing error in hashfile: '...'` -- patterns must match both.
- Machine-readable error format (`<file>:<line>:<hash>:<error>`) uses colons as delimiters, but many hash types also contain colons. Regex uses non-greedy `(.+?)` for the file path capture.

## Clippy / Linting

- `unsafe_code` is **forbidden** -- no `unsafe` blocks allowed anywhere.
- `unwrap_used` is **denied** -- use `?` with `anyhow`/`thiserror`, or handle the error explicitly.
- `panic` is **denied** -- use `bail!` or return `Err` instead.
- `expect_used` is **warned** -- prefer `context()` from anyhow.
- `warnings = "deny"` makes ALL warnings fatal, including `dead_code` and `unfulfilled_lint_expectations` -- `#[expect(dead_code)]` on items that are transitively reachable will itself error.
- `as_conversions` is **warned** -- use `.try_into().unwrap_or(fallback)` or `.into()` instead of `as` casts.
- `pattern_type_mismatch` is **warned** -- use `match *err` when matching on `&T` references, not `match err`.
- `match_same_arms` is **warned** -- merge match arms with identical bodies using `|`.
- `missing_const_for_fn` is **warned** -- add `const` to pure functions that qualify.
- `redundant_closure` is **warned** -- use `.when(is_retryable)` not `.when(|e| is_retryable(e))`.
- `expect_used`, `unwrap_used`, `panic`, `indexing_slicing` are **warned/denied** in production but allowed in `#[cfg(test)]` modules and integration tests via module-level `#[allow(...)]`.

## File Downloads

- Resources (wordlists, masklists, rulelists) can exceed 100 GB. All download pipelines stream to disk -- never buffer fully in memory.
- Use atomic writes (temp file + `fs::rename`) for crash safety on large downloads.

## Cross-Platform

- Hashcat session file location varies by OS: `~/.local/share/hashcat/sessions/` (Linux), `~/.hashcat/sessions/` (legacy), or next to the binary (Windows).
- `os.Symlink` equivalents require elevated privileges on Windows -- test helpers should skip symlink tests on Windows.

## Pre-commit Hooks

- `rustfmt`, `mdformat`, and `cargo-sort` hooks auto-fix files on first run, causing the commit to fail. Re-stage modified files and retry.
- PostToolUse hooks (cargo fmt, clippy) may restructure code after edits -- always re-read modified files before making dependent changes to avoid working against stale assumptions.
- `cargo-sort` hook rev may auto-update in `.pre-commit-config.yaml` -- stage the updated config alongside your commit.
- `cargo-machete` detects unused dependencies -- remove them before committing.

## Dependency Version Constraints

- `sysinfo` 0.36+ requires Rust 1.86+, 0.38+ requires 1.88+ -- pin to `0.35` while `rust-version = "1.85"`.
- `reqwest` 0.13 renamed feature `rustls-tls` to `rustls` -- use `rustls` in Cargo.toml features.
- `sha2` 0.11 changed `finalize()` return type -- output no longer implements `LowerHex`. Format bytes manually with `write!(acc, "{byte:02x}")`.

## Retry Classification Testing

- `reqwest::Error` has no public constructors -- test `is_retryable` with real network calls (e.g., `192.0.2.1` for timeout/connect errors) inside a `tokio::runtime::Builder::new_current_thread()` block.
- Only `is_timeout()` and `is_connect()` are retryable for `ApiError::Request`; `is_request()` includes permanent builder/decode errors.

## Exit Codes

- Hashcat negative exit codes (e.g., -11) arrive as unsigned 8-bit on Unix (e.g., 245). `normalize_exit_code` in `hashcat/exit_code.rs` converts 245-255 back to -11 through -1 before classification.
