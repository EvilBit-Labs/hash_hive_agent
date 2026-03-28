# Development

## Prerequisites

All dev tooling is managed by [mise](https://mise.jdx.dev/) via `mise.toml`. Never install tools manually.

```bash
# Install mise
curl https://mise.run | sh

# Install all project tools (Rust toolchain, just, cargo extensions, etc.)
mise install
```

## Common Commands

All dev commands run through [just](https://github.com/casey/just) (see `justfile`):

```bash
just fmt            # Format code (rustfmt, 2024 edition)
just lint-rust      # Run clippy with workspace lint config
just test           # Run tests (cargo nextest)
just coverage-check # Verify 80%+ coverage (cargo llvm-cov)
just ci-check       # Full local CI parity check
just changelog      # Generate CHANGELOG.md from commits (git-cliff)
```

## CI

GitHub Actions workflows in `.github/workflows/` cover: quality checks, tests, cross-platform tests, and coverage.

## Security Tooling

- `cargo audit` — vulnerability scanning for dependencies.
- `cargo deny` — supply chain security. Bans `openssl` and `git2`; use `rustls` and `gix` instead.

## Release Pipeline

- `release-plz.toml` + `cargo-dist` for releases.
- `git-cliff` generates the changelog from conventional commits.

## Pre-commit Hooks

The following hooks run automatically on commit:

`rustfmt`, `clippy`, `cargo-check`, `cargo-machete`, `mdformat`, `cargo-audit`, `cargo-sort`, `actionlint`

See [GOTCHAS.md](../GOTCHAS.md#pre-commit-hooks) for known quirks with auto-fixing hooks.
