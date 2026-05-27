# Testing

## Test Organization

- **Unit tests:** `#[cfg(test)]` modules within the same source file.
- **Integration tests:** `tests/` directory.

## Running Tests

```bash
just test    # Runs cargo nextest
```

## Test Libraries

| Crate      | Purpose                                          |
| ---------- | ------------------------------------------------ |
| `wiremock` | HTTP mocking for API client tests                |
| `tempfile` | Temporary files/directories for filesystem tests |
| `insta`    | Snapshot testing                                 |

## Coverage

Minimum threshold: **80%**.

```bash
just coverage-check    # cargo llvm-cov with threshold enforcement
```

## Lint Relaxations in Tests

`expect_used`, `unwrap_used`, `panic`, and `indexing_slicing` are allowed in `#[cfg(test)]` modules and integration tests via module-level `#[allow(...)]`. Production code must not use these.
