# cargo-feature-lens

[![Crates.io](https://img.shields.io/crates/v/cargo-feature-lens?style=flat-square)](https://crates.io/crates/cargo-feature-lens)

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue?style=flat-square)](#license)

**Crate Feature Auditor & Visualizer**  

> *See your dependency features. Shrink your compile times.*

`cargo-feature-lens` is a cargo subcommand that statically analyzes your workspace's entire dependency graph and builds a precise map of how crate features propagate, where duplication hides, and which optional features are silently bloating your builds. It goes far beyond `cargo tree` -- it reads `Cargo.toml` files of your dependencies, flags unused active features, detects conflicting feature requirements, and outputs a clean, scannable **feature footprint** report.

---

## 🤔 The Problem

The Rust language team has called out crate features as one of the most frustrating, unresolved documentation failures in the ecosystem. There is currently no standard, elegant way to know:

- What each optional feature flag actually pulls in.

- When a feature is strictly required vs. unnecessarily activated.

- Where features are duplicated across your dependency graph.

Developers are left with trial‑and‑error compilation, oversized binaries, and creeping build times.

---

## 🔍 What `cargo-feature-lens` Does

- **Full dependency graph traversal** -- walks your entire workspace, including transitive dependencies.

- **Feature origin tracking** -- traces exactly *who* enables a feature and *why*.

- **Unused feature detection** -- identifies features you've activated but that nothing in your build actually needs.

- **Conflict & duplication spotting** -- catches crates that enable mutually incompatible features, or features that are redundantly turned on from multiple sources.

- **Compile‑time bloat analysis** -- flags optional features that are pulling in heavy dependencies you might not want.

- **Human‑ and machine‑readable output** -- terminal, Markdown, or JSON reports for issues, docs, scripts, and dashboards.

All without building your project -- purely static analysis.

---

## 📦 Installation

```bash

cargo install cargo-feature-lens

```

Requires Rust **1.70+** (stable).

---

## 🚀 Quick Start

Inside any Cargo workspace, run:

```bash

# Default -- terminal report

cargo feature-lens

# Export a Markdown report

cargo feature-lens --output report.md

# Select an explicit output format

cargo feature-lens --format terminal
cargo feature-lens --format markdown --output report.md
cargo feature-lens --format json --output report.json

# Show only unused features

cargo feature-lens --unused

# List features that contribute the most to the dependency tree size

cargo feature-lens --bloat

# Filter to a specific crate (with regex)

cargo feature-lens --crate serde

```

---

## 📊 Example Output (Terminal)

```

 Feature Footprint Report for `my-project` (workspace: 3 members)

───────────────────────────────────────────────────────────────

 ✓ 42 total features active across 15 crates

 ⚠ 5 potentially unused features detected

 🟡 3 duplicate feature activations

 ┌─ reqwest (0.11.27)

 │  ✓ default-features = false

 │  ⚠ unused: "cookies" (enabled by `my-crate/Cargo.toml`, but never required)

 │  🟡 duplicate: "rustls-tls" (also enabled via `tokio-tungstenite`)

 │  ⚡ bloat: "json" → pulls in `serde_json` (already present via `serde`)

 │

 ├─ tokio (1.37.0)

 │  🔒 conflict: features "rt-multi-thread" and "rt" both active (the latter is implied)

 │

 └─ ...

```

---

## 📄 Markdown Report Example

When using `--output report.md`, you get a ready‑to‑share document:

```markdown

## Feature Footprint -- my-project

| Crate        | Version | Active Features | Issues |

|--------------|---------|-----------------|--------|

| reqwest      | 0.11.27 | rustls-tls, json, cookies | ⚠ unused `cookies`, 🟡 duplicate `rustls-tls`, ⚡ bloat `json` |

| tokio        | 1.37.0  | rt-multi-thread, rt | 🔒 conflict: redundant `rt` |

...

```

---

## 🧠 How It Works

1\. **Resolve the dependency graph** using `cargo metadata` (no compilation).

2\. **Parse every `Cargo.toml`** in the resolved graph to understand available features, dependencies, and optionality.

3\. **Walk feature resolution** as Cargo would, tracking exact activation paths.

4\. **Apply heuristics** to detect:

   - Features that are never referenced by code (`#[cfg(feature = ...)]` inspection is planned, currently based on usage in `Cargo.toml`).

   - Features that are enabled by multiple ancestors but could be trimmed.

   - Features that pull in crates already present through other paths (duplicate dependency versions).

5\. **Generate the report** with clear severities and actionable recommendations.

## 📘 Documentation

- [User guide](docs/guide.md) -- report interpretation, feature-trimming workflows, and JSON usage.
- [Architecture notes](docs/architecture.md) -- module-level overview for contributors.
- [Contributing guide](CONTRIBUTING.md) -- setup, style, testing, and PR checklist.

---

## 📈 Why Teams Love It

Compilation times are consistently a **top‑3 pain point** for teams scaling Rust codebases. A typical mid‑sized project may have dozens of features enabled "just in case". `cargo-feature-lens` gives you the data to:

- Trim 10--30% of unnecessary feature flags in minutes.

- Avoid pulling entire frameworks when you only need one codec.

- Keep your `Cargo.toml` lean, understandable, and review‑ready.

It's an easy win for performance, CI budgets, and developer happiness.

---

## 🔧 Planned Features

- Integration with `cargo-deny` to reject known‑bloated features in CI.

- `#[cfg]`‑aware analysis (parse source to confirm feature usage).

- Interactive TUI mode.

- Suggestions for feature unification (e.g., "replace `foo/a` + `foo/b` with `foo/full`").

---

## 🤝 Contributing

Contributions are hugely welcome! Take a look at the [open issues](https://github.com/your-org/cargo-feature-lens/issues) or open a discussion.

1\. Fork the repository.

2\. Create a branch (`git checkout -b feat/amazing-idea`).

3\. Commit your changes (`git commit -am 'Add amazing feature'`).

4\. Push and open a PR.

Please run `cargo fmt`, `cargo clippy`, and existing tests before submitting.

---

## 🧪 Development

```bash

# Build

cargo build

# Run on a test workspace

cargo run -- feature-lens --manifest-path tests/fixtures/workspace/Cargo.toml

# Test

cargo test

```

---

## 📜 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

## 🙏 Acknowledgments

Inspired by countless hours staring at `cargo tree` output, discussions in the Rust language team, and the community's tireless fight against slow compile times.

**Don't guess your features -- lens them.** 🔍