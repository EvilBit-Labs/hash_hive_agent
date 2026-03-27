# Contributing

Thank you for your interest in contributing to hash_hive_agent.

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

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat(agent): add heartbeat retry logic
fix(hashcat): handle v7 error format
docs: update API type documentation
test(parser): add colon-in-hash edge case
```

## Pull Requests

- One logical change per PR.
- Include tests for new functionality.
- Update AGENTS.md / GOTCHAS.md if adding new patterns or pitfalls.
- All CI checks must pass.

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.
