# Cargo Feature Lens

[![CI](https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer/actions/workflows/ci.yml?query=branch%3Amain)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-Apache--2.0-blue)

**Crate Feature Auditor & Visualizer**

`cargo-feature-lens` is a Cargo subcommand for auditing Cargo feature usage. It builds a resolved dependency and feature graph, reports feature-related findings, and can export graph views for review or documentation.

The tool uses `cargo metadata`, manifest parsing, configured rules, and conservative Rust source scanning. It does not compile the target project.

## Scope and limits

Cargo Feature Lens is an audit helper, not a replacement for Cargo, `cargo check`, or project-specific review. It treats `cargo metadata` as the source of truth for resolved packages and active features, then adds lightweight analysis for feature provenance, rule-driven findings, and simple source references.

Findings are review signals. They can highlight risky or redundant feature choices, but they do not automatically prove that a feature should be removed. Source scanning is intentionally conservative, and the tool does not evaluate generated code, build scripts, macros, or every possible `cfg` expression.

## Demo

Run the included conflict fixture to see a short, reproducible audit report:

```bash
cargo run -- feature-lens --manifest-path tests/fixtures/conflict-reqwest/Cargo.toml --check --fail-on warning
```

Sample output (abridged):

```text
Feature Footprint Report
────────────────────────
✓ 2 total features active across 2 crates
⚠ 3 findings detected
Finding summary: 3 visible findings
  severity: info 0, warning 2, error 1
  kind: Unused 2, Conflict 1

┌─ reqwest (0.0.0)
│  active features: native-tls, rustls-tls
│  ✖ Conflict: TLS backends are mutually exclusive. Choose exactly one (native-tls, rustls-tls, or default-tls).
```

Read this top-down: the summary is CI-friendly triage, then crate sections show exactly which feature set triggered each advisory finding. This complements Cargo's native tooling by focusing on feature provenance and rule-driven audit hints, not build correctness proof.

## What it reports

- Active crate features and where feature activation comes from.
- Unused feature candidates, including source-aware checks for simple `#[cfg(feature = "...")]` and `cfg!(feature = "...")` usage.
- Duplicate feature activation across dependency paths.
- Conflict, bloat, and default-feature opt-out findings from built-in and local rules.
- CI-friendly pass/fail results through `--check` and severity thresholds.

## Output formats

Reports can be rendered as:

- Terminal text
- Markdown
- JSON
- Graphviz DOT
- Mermaid

## Installation from source

```bash
git clone https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer.git
cd Crate-Feature-Auditor-Visualizer
cargo install --path .
```

Requires Rust 1.70 or newer.

## Quick start

Analyze the current Cargo package or workspace:

```bash
cargo feature-lens
```

Analyze an explicit manifest:

```bash
cargo feature-lens --manifest-path Cargo.toml
cargo feature-lens --manifest-path path/to/workspace/Cargo.toml
```

Write a report:

```bash
cargo feature-lens --format markdown --output report.md
cargo feature-lens --format json --output report.json
```

Export a graph:

```bash
cargo feature-lens --format dot --output graph.dot
cargo feature-lens --format mermaid --output graph.md
```

Focus or gate results:

```bash
cargo feature-lens --unused
cargo feature-lens --bloat
cargo feature-lens --min-severity warning
cargo feature-lens --check --fail-on error
cargo feature-lens --manifest-path Cargo.toml --crate serde
```

Analyze a crate from crates.io:

```bash
cargo feature-lens --remote --crate tokio
cargo feature-lens --remote --crate serde --crate-version 1
```

## Local rules

Add an optional `feature-lens.toml` file in the working directory to define project-specific conflict, bloat, or default-feature guidance. Local rules are additive with the built-in rule database in `docs/suggestions.json`.

## CI usage

Use `--check` with `--fail-on` to turn advisory findings into a CI gate:

```bash
cargo feature-lens --check --fail-on warning
```

Use `--min-severity` when you want reports to hide lower-severity findings but keep the full analysis behavior available for stricter runs.

## More documentation

- [User guide](docs/guide.md)
- [Architecture](docs/architecture.md)
- [Contributing](docs/CONTRIBUTING.md)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).