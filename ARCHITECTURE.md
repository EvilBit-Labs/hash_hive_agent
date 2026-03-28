# Architecture

## Overview

hash_hive_agent is a distributed Rust agent for HashHive. It manages hash-cracking tasks via hashcat across Linux, macOS, and Windows. The agent is a long-lived CLI client that authenticates with the HashHive server, polls for work, executes hashcat, and reports results.

**Entrypoint:** `main.rs` initializes tracing, parses CLI via clap, loads config, and enters the agent run loop.

**Configuration:** Environment variables (`HASH_HIVE_*`), CLI flags (clap derive), and optional YAML/TOML config file.

## Project Structure

```text
src/
  main.rs        — Binary entrypoint (clap CLI, tracing init, agent run loop)
  lib.rs         — Library root, re-exports all modules
  cli.rs         — Clap derive CLI definition
  config/        — Configuration loading (env, file, CLI overrides) with typed defaults
  api/           — HTTP client layer: typed request/response structs, reqwest-based client, error types
  agent/         — Agent lifecycle: heartbeat loop, task polling, graceful shutdown
  hashcat/       — Hashcat integration: session management, output parsing, exit code classification
  task/          — Task lifecycle: resource downloads (streaming), hashcat execution, progress reporting
  benchmark/     — Benchmark execution and disk caching
  platform/      — Cross-platform abstractions (Linux, macOS, Windows)
```

## Task Lifecycle & API Contract

The agent interacts with the HashHive server API through these stages:

1. **Authentication:** Create session with pre-shared token via `POST /sessions`.
2. **Heartbeat:** Send periodic status via `POST /heartbeat`.
3. **Polling for Tasks:** Request new tasks from `POST /tasks/next`.
4. **Task Execution:** Download resources (streaming, no full-file buffering), launch hashcat, report progress via `POST /tasks/{id}/report`.
5. **Benchmarks:** Submit hashcat benchmark results via `POST /benchmark`.
6. **Error Reporting:** Report agent errors via `POST /errors`.
7. **Zap Lists:** Fetch already-cracked hashes via `GET /tasks/{id}/zaps` to skip during processing.

All API interactions use the Agent API v1 contract (see `../hash_hive/packages/openapi/agent-api.yaml`). All API requests use exponential backoff with jitter for transient failures (see [Retry / Backoff](#retry--backoff)).

## Retry / Backoff

- `backon` crate provides exponential backoff with jitter via `ExponentialBuilder` + `Retryable`.
- `RetryConfig` in `src/api/client.rs` maps `AgentConfig` backoff fields to `backon` builder params.
- Only `ApiError::Server` (5xx) and transient `ApiError::Request` (timeout/connect) are retryable; `Auth`, `NotFound`, `Parse`, `Unexpected`, and `UrlParse` fail immediately.
- `ApiClient::new(base_url, retry_config)` returns `Result<Self, ApiError>` — callers provide `RetryConfig` explicitly.
- Never add default/optional constructor paths that bypass `AgentConfig` — all runtime config must flow through explicitly.
- All public `ApiClient` methods use `with_retry` internally — retries are transparent to callers.

## Concurrency

- `tokio` async runtime for all I/O, subprocess management, and timers.
- `CancellationToken` (from `tokio-util`) for cooperative shutdown.
- `tokio::select!` for multiplexing cancellation with work.
- No `time::sleep` — always use `select!` with cancellation.

## Performance Constraints

- `regex::Regex` compiled at package level via `LazyLock`, never inside functions.
- Streaming file downloads — never buffer 100GB+ files in memory.
- Atomic file writes (temp + rename) for crash safety.
