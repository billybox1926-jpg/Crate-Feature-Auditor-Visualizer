# Security Policy

Crate Feature Auditor & Visualizer is a local Rust CLI tool. It has no network server, authentication layer, or database.

## Supported versions

Security reports should target the latest `main` branch.

## Reporting a concern

Open a GitHub issue with a concise description. Avoid posting exploit details or private data in public issues.

Useful reports include:
- The affected command, flag, or output format.
- A small reproduction case without private data.
- Expected vs. actual behavior.

## Security Model & Safety Guarantees

`cargo-feature-lens` is designed to be a safe, read-only audit helper. To minimize attack surface, it adheres to the following principles:
- **No Compilation:** The tool does not invoke `cargo build` or `cargo check`. It does not compile your code.
- **No Build Script Execution:** It does not execute `build.rs` scripts or arbitrary macros.
- **Metadata-Driven:** It relies primarily on `cargo metadata` and static parsing of `Cargo.toml` manifests.
- **Conservative Source Scanning:** Any Rust source code scanning is strictly read-only and uses conservative pattern matching (e.g., looking for `#[cfg(feature = "...")]`), not full AST parsing or code execution.
- **Isolated Remote Analysis:** When using `--remote`, crate probing is performed in an isolated, uniquely generated temporary directory that is immediately cleaned up upon completion.

## Scope

This is an audit helper tool. Security concerns would most likely relate to:
- Local file handling in the Rust source scanner.
- CI configuration.
- Output file generation.

Findings are review signals, not automatic proof of vulnerabilities.

## Response expectations

Small project, limited capacity. Reports reviewed as maintainers are available.
