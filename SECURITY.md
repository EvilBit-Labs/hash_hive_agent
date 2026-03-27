# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in hash_hive_agent, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email: **security@evilbitlabs.io**

We will acknowledge receipt within 48 hours and provide a detailed response within 5 business days.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Security Measures

- `unsafe` code is **forbidden** at the workspace level.
- All dependencies are audited daily via `cargo audit` and `cargo deny`.
- CI runs cross-platform tests on Linux, macOS, and Windows.
- No secrets are hardcoded -- all credentials use environment variables.
- TLS via `rustls` (no OpenSSL dependency).
