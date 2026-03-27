# AGENTS

Standards and architecture for the hash_hive_agent project.

@GOTCHAS.md

## Architecture

### Core Concepts

- **Purpose:** A distributed Rust agent for HashHive, managing hash-cracking tasks via hashcat across Linux, macOS, and Windows.
- **Entrypoint:** `main.rs` initializes tracing, parses CLI via clap, loads config, and enters the agent run loop.
- **Configuration:** Environment variables (`HASH_HIVE_*`), CLI flags (clap derive), and optional YAML/TOML config file.

### Task Lifecycle & API Contract

The agent is a long-lived CLI client interacting with the HashHive server API:

1. **Authentication:** Create session with pre-shared token via `POST /sessions`.
2. **Heartbeat:** Send periodic status via `POST /heartbeat`.
3. **Polling for Tasks:** Request new tasks from `POST /tasks/next`.
4. **Task Execution:** Download resources (streaming, no full-file buffering), launch hashcat, report progress via `POST /tasks/{id}/report`.
5. **Benchmarks:** Submit hashcat benchmark results via `POST /benchmark`.
6. **Error Reporting:** Report agent errors via `POST /errors`.
7. **Zap Lists:** Fetch already-cracked hashes via `GET /tasks/{id}/zaps` to skip during processing.

- All API interactions use the Agent API v1 contract (see `../hash_hive/packages/openapi/agent-api.yaml`).
- Implement exponential backoff for failed API requests.

## Rust

The project follows idiomatic Rust 2024 edition practices.

### Project Structure

- `src/main.rs` — Binary entrypoint (clap CLI, tracing init, agent run loop).
- `src/lib.rs` — Library root, re-exports all modules.
- `src/cli.rs` — Clap derive CLI definition.
- `src/config/` — Configuration loading (env, file, CLI overrides) with typed defaults.
- `src/api/` — HTTP client layer: typed request/response structs, reqwest-based client, error types.
- `src/agent/` — Agent lifecycle: heartbeat loop, task polling, graceful shutdown.
- `src/hashcat/` — Hashcat integration: session management, output parsing, exit code classification.
- `src/task/` — Task lifecycle: resource downloads (streaming), hashcat execution, progress reporting.
- `src/benchmark/` — Benchmark execution and disk caching.
- `src/platform/` — Cross-platform abstractions (Linux, macOS, Windows).

### Lint Configuration

- `unsafe_code` is **forbidden** globally.
- `unwrap_used` and `panic` are **denied** — use `?`, `anyhow`, or `thiserror` instead.
- Full pedantic clippy configuration in `Cargo.toml` `[workspace.lints.clippy]`.
- Zero warnings policy: `warnings = "deny"`.

### Error Handling

- Library errors: `thiserror` with structured error enums (see `src/api/error.rs`).
- Application errors: `anyhow` with `.context()` for actionable messages.
- Never `unwrap()` or `expect()` in production code paths.
- Use `?` propagation everywhere.

### Concurrency

- `tokio` async runtime for all I/O, subprocess management, and timers.
- `CancellationToken` (from `tokio-util`) for cooperative shutdown.
- `tokio::select!` for multiplexing cancellation with work.
- No `time::sleep` — always use `select!` with cancellation.

### Performance

- `regex::Regex` compiled at package level via `LazyLock`, never inside functions.
- Streaming file downloads — never buffer 100GB+ files in memory.
- Atomic file writes (temp + rename) for crash safety.

### Formatting & Linting

- **Formatting:** `cargo fmt` (rustfmt 2024 edition).
- **Linting:** `cargo clippy` with workspace lint config.
- **Dev commands:** `just lint-rust`, `just test`, `just ci-check`.

### Testing

- Unit tests in `#[cfg(test)]` modules within the same file.
- Integration tests in `tests/`.
- Use `wiremock` for HTTP mocking, `tempfile` for filesystem tests.
- `insta` for snapshot testing.
- Run with: `just test` (uses `cargo nextest`).

### Tooling

- **Dev tools:** `mise` manages all toolchains via `mise.toml`.
- **Task runner:** `just` (see `justfile`).
- **CI:** GitHub Actions (`.github/workflows/`): quality, test, cross-platform test, coverage.
- **Security:** `cargo audit`, `cargo deny` for supply chain security.
- **Coverage:** `cargo llvm-cov` with 80% minimum threshold.

## Git

### Commit Messages

Conventional Commits: `<type>(<scope>): <description>`. Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.

### Changelog

`CHANGELOG.md` is auto-generated from commit messages using `git-cliff` (`just changelog`).
