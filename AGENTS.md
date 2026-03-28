# Agent Context

This file provides AI coding assistants with project context. All substantive documentation lives in the files linked below.

@GOTCHAS.md

## Project Documentation

- **Architecture & Design**: [ARCHITECTURE.md](./ARCHITECTURE.md) — system overview, module boundaries, task lifecycle, retry/backoff design, concurrency patterns
- **Contributing Standards**: [CONTRIBUTING.md](./CONTRIBUTING.md) — code style, lint policy, error handling, commit format, PR process
- **Known Gotchas**: [GOTCHAS.md](./GOTCHAS.md) — hashcat output routing, clippy edge cases, dependency constraints, cross-platform issues
- **Development Setup**: [docs/development.md](./docs/development.md) — mise toolchain, just commands, CI, pre-commit hooks
- **Testing**: [docs/testing.md](./docs/testing.md) — test organization, libraries, coverage requirements

## Agent-Specific Notes

- The project uses Rust 2024 edition with strict linting. Read `Cargo.toml` `[workspace.lints]` before suggesting code.
- All runtime config flows through `AgentConfig` explicitly — never introduce default/optional constructors that bypass it.
- All API requests retry transparently via `with_retry` in `ApiClient` — callers should not add their own retry logic.
- Prefer `select!` with `CancellationToken` over bare `time::sleep` for long waits. Short backoff delays in `backon` are an accepted exception.

## Agent Rules <!-- tessl-managed -->

@.tessl/RULES.md follow the [instructions](.tessl/RULES.md)
