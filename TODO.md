# TODO

This is the working task list for `cargo-feature-lens`. Items are roughly ordered by priority within each section, but feel free to pick up anything that interests you.

---

## Core Functionality

- [ ] **Feature resolution engine**
  - [ ] Implement exact unification matching Cargo's edition 2021 resolver (currently basic merge).
  - [ ] Handle platform‑specific features (`[target.'cfg(...)'.dependencies]`).
  - [ ] Support `dep:` syntax fully (including namespaced features).

- [ ] **Manifest parsing robustness**
  - [ ] Handle workspace inheritance (`workspace = true`).
  - [ ] Parse `Cargo.lock`‑only packages when manifest is not available (graceful fallback).

- [ ] **Error handling & diagnostics**
  - [ ] Replace `unwrap()` calls with proper error types and `anyhow`/`thiserror`.
  - [ ] Provide clear error messages when `cargo metadata` fails or manifests are malformed.

---

## Analysis Passes

- [ ] **Unused feature detection**
  - [ ] Integrate `#[cfg(feature = "...")]` source scanning with `syn` to get definitive usage.
  - [ ] Distinguish between “never used” and “used only in tests/benches”.
  - [ ] Add flag to ignore features used only in dev-dependencies.

- [ ] **Conflict detection**
  - [ ] Build a community‑contributed rules database (like `suggestions.json`) and load it automatically.
  - [ ] Detect feature implication relationships by parsing crate docs or metadata.
  - [ ] Support custom rules via `feature-lens.toml`.

- [ ] **Bloat analysis**
  - [ ] Calculate exact “size cost” (e.g., additional unique crate count) when a feature is enabled.
  - [ ] Detect duplicate crate versions introduced by features (e.g., `serde 1.0` vs `serde 0.9`).
  - [ ] Integrate with `cargo-bloat` or `twiggy` data if available.

- [ ] **Duplication analysis**
  - [ ] Refine detection of redundant requests when features come from multiple ancestors.
  - [ ] Suggest specific `Cargo.toml` changes to remove duplication.

---

## Output & Reporting

- [ ] **Terminal UI**
  - [ ] Add a rich TUI (using `ratatui`) for interactive exploration.
  - [ ] Allow expanding/collapsing crate nodes and seeing feature origins in real time.

- [ ] **Markdown/JSON output**
  - [ ] Generate detailed per‑crate sections in Markdown with actionable advice.
  - [x] Provide machine‑readable JSON output (`--format json`).
  - [ ] Support structured output for CI (e.g., GitHub Actions annotations, GitLab Code Quality report).

- [ ] **Colour & accessibility**
  - [ ] Respect `NO_COLOR` and `CLICOLOR` conventions.
  - [ ] Ensure icons and colours degrade gracefully for non‑unicode terminals.

---

## Integration & CI

- [ ] **Pre‑commit / CI mode**
  - [ ] Add `--check` flag that exits with non‑zero on unused or conflicting features (suitable for CI).
  - [ ] Provide severity thresholds (e.g., `--deny warning` treats warnings as errors).

- [ ] **cargo-deny integration**
  - [ ] Export findings in a format that `cargo-deny` can consume as an advisory database.
  - [ ] Allow `cargo-feature-lens` to generate a deny.toml snippet for bloated features.

- [ ] **IDE integration (future)**
  - [ ] LSP server or diagnostic output for editors like VS Code / Rust Analyzer.

---

## Performance & Robustness

- [ ] **Benchmarks**
  - [ ] Set up benchmarks for workspace with 100+ crates.
  - [ ] Optimise feature unification (avoid cloning entire feature maps).

- [ ] **Caching**
  - [ ] Cache parsed manifests and metadata to disk to speed up repeated runs.
  - [ ] Respect `CARGO_HOME` and work offline where possible.

- [ ] **Testing**
  - [ ] Add more integration tests with real‑world crates (e.g., `tokio`, `reqwest`).
  - [ ] Fuzz the resolver with randomly generated feature graphs.
  - [ ] CI pipeline with `cargo test`, `cargo fmt`, `cargo clippy`, and example runs.

---

## Documentation & Community

- [ ] **User guide**
  - [x] Write a `docs/guide.md` explaining how to interpret reports and fix common issues.
  - [x] Add example workflows (trimming a single crate’s features, auditing a whole workspace).

- [ ] **Contribution guide**
  - [x] Document the architecture for new contributors (`docs/architecture.md`).
  - [x] Add a `CONTRIBUTING.md` with code style, PR checklist, and how to add new analysis passes.

- [ ] **Website / demo**
  - [ ] Create a landing page showing a live terminal recording.
  - [ ] Publish to crates.io with a good `README`.

---

## Miscellaneous

- [ ] **Feature unification suggestions**
  - [ ] Detect when using `foo/a` + `foo/b` can be replaced with `foo/full`.
  - [ ] Offer a `--fix` mode that automatically edits `Cargo.toml` to apply safe optimisations.

- [ ] **Platform‑aware analysis**
  - [ ] Option to filter features active on a specific target triple.

- [ ] **Dependency tree visualisation**
  - [ ] Export graph in DOT format for Graphviz.
  - [ ] Generate a Mermaid.js dependency diagram in the Markdown report.

---

Want to help? Pick an unassigned task, open an issue to discuss, and jump in!