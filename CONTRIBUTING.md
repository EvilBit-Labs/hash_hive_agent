# Contributing

Thank you for your interest in contributing to hash_hive_agent.

## Before You Start

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** — system design, module boundaries, task lifecycle
- **[GOTCHAS.md](./GOTCHAS.md)** — known pitfalls and hard-won lessons
- **[docs/development.md](./docs/development.md)** — local setup, toolchain, dev commands
- **[docs/testing.md](./docs/testing.md)** — test strategy, coverage requirements

## Development Setup

```bash
# Install mise (manages all dev tools)
curl https://mise.run | sh

# Install project tools
mise install

# Run tests
just test

# Run full CI check locally
just ci-check
```

## Code Quality

Before submitting a PR:

1. `just fmt` -- format code
2. `just lint-rust` -- pass all lints (zero warnings policy)
3. `just test` -- pass all tests
4. `just coverage-check` -- maintain 80%+ coverage

### Lint Policy

- `unsafe_code` is **forbidden** globally.
- `unwrap_used` and `panic` are **denied** — use `?`, `anyhow`, or `thiserror` instead.
- Full pedantic clippy configuration in `Cargo.toml` `[workspace.lints.clippy]`.
- Zero warnings policy: `warnings = "deny"`.

See [GOTCHAS.md — Clippy / Linting](./GOTCHAS.md#clippy--linting) for the full lint breakdown and edge cases.

### Error Handling

- Library errors: `thiserror` with structured error enums (see `src/api/error.rs`).
- Application errors: `anyhow` with `.context()` for actionable messages.
- Never `unwrap()` or `expect()` in production code paths.
- Use `?` propagation everywhere.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat(agent): add heartbeat retry logic
fix(hashcat): handle v7 error format
docs: update API type documentation
test(parser): add colon-in-hash edge case
```

Changelog is auto-generated from commit messages using `git-cliff` (`just changelog`).

## Pull Requests

- One logical change per PR.
- Include tests for new functionality.
- Update documentation if adding new patterns or pitfalls.
- All CI checks must pass.

## EvilBit-Labs Conventions

This project follows shared conventions across EvilBit-Labs repos (libmagic-rs, mmap-guard, stringy, token-privilege, daemoneye):

- `unsafe_code = "forbid"` and `unwrap_used = "deny"` in `[workspace.lints]`.
- `cargo-sort` enforces dependency ordering in Cargo.toml.
- `cargo-deny` bans `openssl` and `git2` — use `rustls` and `gix` instead.
- `mise.toml` manages all dev tooling — never install tools manually.
- `justfile` provides all dev commands — `just ci-check` for full local CI parity.

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.
