# cargo-feature-lens

**Crate Feature Auditor & Visualizer**

`cargo-feature-lens` is a Cargo subcommand that statically analyzes a Rust package or workspace dependency graph and reports which crate features are active, where duplicate feature activation may be happening, and which configured conflict or bloat rules match the current graph.

The tool uses `cargo metadata` and manifest parsing. It does not compile the target project.

## Current capabilities

- Build a best-effort feature graph from Cargo metadata and parsed `Cargo.toml` files.
- Track active features, optional dependencies, dependency feature requests, and feature source paths.
- Report unused, duplicate, conflict, and bloat findings.
- Render reports as terminal text, Markdown, or JSON.
- Filter reports by crate-name substring, unused findings, or bloat findings.

## Installation from source

```bash
git clone https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer.git
cd Crate-Feature-Auditor-Visualizer
cargo install --path .
```

Requires Rust 1.70 or newer.

## Quick start

Inside a Cargo package or workspace, run:

```bash
cargo feature-lens
```

Analyze an explicit manifest path:

```bash
cargo feature-lens --manifest-path Cargo.toml
cargo feature-lens --manifest-path path/to/workspace/Cargo.toml
```

Write a Markdown or JSON report:

```bash
cargo feature-lens --format markdown --output report.md
cargo feature-lens --format json --output report.json
```

Focus the report:

```bash
cargo feature-lens --unused
cargo feature-lens --bloat
cargo feature-lens --manifest-path Cargo.toml --crate serde
```

`--crate` currently filters local Cargo metadata. Remote crates.io analysis, such as analyzing `tokio` without a local manifest, is tracked as future work.

## Documentation

Start with the documentation index: [`docs/README.md`](docs/README.md).

Key documents:

- [User guide](docs/guide.md)
- [Architecture](docs/architecture.md)
- [Contributing](docs/CONTRIBUTING.md)
- [Maintainer workflow](docs/MAINTAINER_WORKFLOW.md)
- [Issue labels](docs/ISSUE_LABELS.md)
- [Roadmap](docs/roadmap.md)
- [TODO](docs/TODO.md)

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

Run against the included fixture:

```bash
cargo run -- feature-lens --manifest-path tests/fixtures/basic/Cargo.toml
```

The root-level `suggestions.json` file is an optional rule database used by the conflict and bloat analysis passes.

## Repository workflow

Small maintainer changes currently land directly on `main`. External contributions should still be scoped through issues and pull requests when practical.

Open work is tracked in GitHub Issues for this repository.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
