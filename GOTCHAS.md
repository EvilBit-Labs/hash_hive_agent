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

## File Downloads

- Resources (wordlists, masklists, rulelists) can exceed 100 GB. All download pipelines stream to disk -- never buffer fully in memory.
- Use atomic writes (temp file + `fs::rename`) for crash safety on large downloads.

## Cross-Platform

- Hashcat session file location varies by OS: `~/.local/share/hashcat/sessions/` (Linux), `~/.hashcat/sessions/` (legacy), or next to the binary (Windows).
- `os.Symlink` equivalents require elevated privileges on Windows -- test helpers should skip symlink tests on Windows.

## Exit Codes

- Hashcat negative exit codes (e.g., -11) arrive as unsigned 8-bit on Unix (e.g., 245). `normalize_exit_code` in `hashcat/exit_code.rs` converts 245-255 back to -11 through -1 before classification.
