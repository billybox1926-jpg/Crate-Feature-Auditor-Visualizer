# Contributing to Crate Feature Auditor & Visualizer

`cargo-feature-lens` is a Cargo subcommand for auditing Cargo feature usage in Rust projects. It builds a resolved dependency and feature graph, reports findings, and exports graph views.

## Local setup

Requires Rust 1.70+.

```bash
git clone https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer.git
cd Crate-Feature-Auditor-Visualizer
cargo build
```

## Verification

```bash
cargo test
cargo run -- feature-lens --manifest-path tests/fixtures/conflict-reqwest/Cargo.toml --check --fail-on warning
```

## Project principles

- **Audit helper, not a replacement** for `cargo check` or professional security review.
- **Conservative source scanning.** Findings are review signals, not proof of bugs.
- **No compilation required.** Uses `cargo metadata` as the source of truth.
- **Multiple output formats.** Terminal, Markdown, JSON, Graphviz DOT, Mermaid.

## Repository structure

```
.
├── Cargo.toml
├── Cargo.lock
├── src/
├── tests/
├── docs/
│   ├── guide.md
│   ├── architecture.md
│   └── CONTRIBUTING.md
└── scripts/
```

## Pull request checklist

- Run `cargo test`.
- Describe what changed and whether it affects output format or finding severity.
- Keep changes scoped to one concern.
- Update docs if you change CLI flags, output format, or rule behavior.
- Confirm the conflict fixture still produces expected output.
