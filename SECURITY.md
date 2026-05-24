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

## Scope

This is an audit helper tool. Security concerns would most likely relate to:
- Local file handling in the Rust source scanner.
- CI configuration.
- Output file generation.

Findings are review signals, not automatic proof of vulnerabilities.

## Response expectations

Small project, limited capacity. Reports reviewed as maintainers are available.
