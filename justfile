# Cross-platform justfile using OS annotations
# Windows uses PowerShell, Unix uses bash

set windows-shell := ["powershell.exe", "-c"]
set shell := ["bash", "-c"]
set dotenv-load := true
set ignore-comments := true

# Use mise to manage all dev tools
# See mise.toml for tool versions

mise_exec := "mise exec --"
root := justfile_dir()

# =============================================================================
# GENERAL COMMANDS
# =============================================================================

default:
    @just --list

# =============================================================================
# SETUP AND INITIALIZATION
# =============================================================================

# Development setup - mise handles all tool installation via mise.toml
setup:
    mise install

# =============================================================================
# FORMATTING AND LINTING
# =============================================================================

alias format-rust := fmt
alias format-md := format-docs
alias format-just := fmt-justfile

# Main format recipe - calls all formatters
format: fmt format-json-yaml format-docs fmt-justfile

# Individual format recipes

format-json-yaml:
    @{{ mise_exec }} prettier --write "**/*.{json,yaml,yml}"

format-docs:
    @{{ mise_exec }} mdformat --exclude "target/*" --exclude "node_modules/*" .

fmt:
    @{{ mise_exec }} cargo fmt --all

fmt-check:
    @{{ mise_exec }} cargo fmt --all --check

lint-rust: fmt-check
    @{{ mise_exec }} cargo clippy --all-targets --all-features -- -D warnings

# Format justfile
fmt-justfile:
    @just --fmt --unstable

# Lint justfile formatting
lint-justfile:
    @just --fmt --check --unstable

# Main lint recipe - calls all sub-linters
lint: lint-rust lint-docs lint-justfile

# Lint docs
lint-docs:
    @{{ mise_exec }} markdownlint-cli2 docs/**/*.md README.md

alias lint-just := lint-justfile

# Run clippy with fixes
fix:
    @{{ mise_exec }} cargo clippy --fix --allow-dirty --allow-staged

# Quick development check
check: pre-commit-run lint

[private]
pre-commit-run:
    @{{ mise_exec }} pre-commit run -a

# =============================================================================
# BUILDING AND TESTING
# =============================================================================

build:
    @{{ mise_exec }} cargo build

build-release:
    @{{ mise_exec }} cargo build --release

test:
    @{{ mise_exec }} cargo nextest run --no-capture

test-ci:
    @{{ mise_exec }} cargo nextest run --no-capture

# Run all tests including ignored/slow tests
test-all:
    @{{ mise_exec }} cargo nextest run --no-capture -- --ignored

# =============================================================================
# SECURITY AND AUDITING
# =============================================================================

audit:
    @{{ mise_exec }} cargo audit

deny:
    @{{ mise_exec }} cargo deny check

# =============================================================================
# CI AND QUALITY ASSURANCE
# =============================================================================

# Private helper: run cargo llvm-cov with proper setup
[private]
[unix]
_coverage +args:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/llvm-cov-target
    RUSTFLAGS="--cfg coverage" {{ mise_exec }} cargo llvm-cov --lcov --output-path lcov.info {{ args }}

[private]
[windows]
_coverage +args:
    Remove-Item -Recurse -Force target/llvm-cov-target -ErrorAction SilentlyContinue
    $env:RUSTFLAGS = "--cfg coverage"; {{ mise_exec }} cargo llvm-cov --lcov --output-path lcov.info {{ args }}

coverage:
    @just _coverage

coverage-check:
    @just _coverage --fail-under-lines 80

# Generate HTML coverage report for local viewing
[unix]
coverage-report:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/llvm-cov-target
    RUSTFLAGS="--cfg coverage" {{ mise_exec }} cargo llvm-cov --html --open

[windows]
coverage-report:
    $env:RUSTFLAGS = "--cfg coverage"; {{ mise_exec }} cargo llvm-cov --html --open

# Show coverage summary by file
[unix]
coverage-summary:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/llvm-cov-target
    RUSTFLAGS="--cfg coverage" {{ mise_exec }} cargo llvm-cov

[windows]
coverage-summary:
    $env:RUSTFLAGS = "--cfg coverage"; {{ mise_exec }} cargo llvm-cov

# Full local CI parity check
ci-check: pre-commit-run fmt-check lint-rust test-ci build-release audit coverage-check

# =============================================================================
# DISTRIBUTION AND PACKAGING
# =============================================================================

dist:
    @{{ mise_exec }} dist build

dist-check:
    @{{ mise_exec }} dist check

dist-plan:
    @{{ mise_exec }} dist plan

install:
    @{{ mise_exec }} cargo install --path .

# =============================================================================
# CHANGELOG
# =============================================================================

# Generate changelog
[group('docs')]
changelog:
    @{{ mise_exec }} git-cliff --output CHANGELOG.md

# Generate changelog for a specific version
[group('docs')]
changelog-version version:
    @{{ mise_exec }} git-cliff --tag {{ version }} --output CHANGELOG.md

# Generate changelog for unreleased changes only
[group('docs')]
changelog-unreleased:
    @{{ mise_exec }} git-cliff --unreleased --output CHANGELOG.md

# =============================================================================
# RELEASE MANAGEMENT
# =============================================================================

release:
    @{{ mise_exec }} cargo release

release-dry-run:
    @{{ mise_exec }} cargo release --dry-run

release-patch:
    @{{ mise_exec }} cargo release patch

release-minor:
    @{{ mise_exec }} cargo release minor

release-major:
    @{{ mise_exec }} cargo release major
