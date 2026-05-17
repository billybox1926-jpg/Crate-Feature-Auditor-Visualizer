# Architecture

`cargo-feature-lens` is a static analysis tool that builds a detailed feature footprint of a Cargo workspace without compiling any code. This document describes the high‑level design, core components, and data flow.

---

## Overview

The tool acts as a `cargo` subcommand (`cargo feature-lens`). It leverages `cargo metadata` to obtain the fully resolved dependency graph, then enriches that data by parsing the `Cargo.toml` of every package in the graph. By simulating Cargo's feature resolution and applying heuristics, it produces a report highlighting:

- Which features are active in each crate.

- The origin path(s) that activated each feature.

- Features that are potentially unused or redundant.

- Conflicts (e.g., mutually exclusive features both enabled).

- Compile‑time bloat (optional features that pull in heavy dependencies already present from other paths).

---

## Core Modules

### 1. CLI Interface (`src/main.rs`)

Handles argument parsing (via `clap`) and dispatches to the appropriate sub‑commands. The main entry point:

- `cargo feature-lens` -- runs the full analysis and prints the terminal report.

- `cargo feature-lens --output <file>` -- writes a Markdown report.

- Flags like `--unused`, `--bloat`, `--crate` filter the output.

### 2. Metadata Ingestion (`src/metadata.rs`)

Uses `cargo metadata --format-version 1` to get the raw dependency graph.

- Runs `cargo metadata` as a subprocess and deserializes JSON into structured `Package` and `Resolve` types.

- The resulting `Metadata` struct provides all workspace members, all dependencies (including transitive), and the `resolve` graph (a flattened list of packages with feature lists).

### 3. Cargo.toml Parsing (`src/manifest.rs`)

Parses `Cargo.toml` files for every package in the dependency graph using the `cargo_toml` (or `toml`) crate.

- For each package, we extract:

  - All available `[features]` and their dependency mappings.

  - Optional dependencies (which are also features).

  - Default features.

  - The `[dependencies]` section to know what features are requested by the manifest itself (independent of the resolver).

This module also handles workspace manifests to understand member configurations.

### 4. Feature Resolution Engine (`src/resolver.rs`)

This is the heart of the tool. It simulates Cargo's feature unification in a deterministic way, without actually building.

- **Input**: the raw package graph and parsed manifests.

- **Process**:

  1. Walk the dependency tree from the root workspace members downwards, following the edges from the `resolve` graph.

  2. For each dependency edge, collect features activated by the parent (direct feature flags, default features, features of dependencies).

  3. Perform feature expansion: a feature like `foo` might enable `dep:bar/feat` -- resolve those recursively.

  4. Unify features across the graph: if two parents request different feature sets for the same dependency, Cargo merges them. We mimic that unification, and also record *who* requested what.

- **Output**: an enriched dependency graph where each node (package) holds:

  - `active_features`: the final set of features that will be compiled.

  - `feature_sources`: a map of `feature_name → list of activation paths` (a path is a chain from a root crate to this node).

  - `raw_requested`: what the package's own manifest asked for (if it's a root).

### 5. Analysis Passes (`src/analysis/`)

A collection of passes that inspect the enriched graph to detect issues.

- **`unused.rs`** -- identifies active features that are not required by any downstream code or by explicit user request. A feature is considered "potentially unused" if:

  - It is not a transitive dependency of any other active feature.

  - It is not relied upon by any `cfg(feature = ...)` (future: static source analysis; currently a heuristic based on `Cargo.toml` usage).

- **`conflicts.rs`** -- catches situations where multiple features that should be mutually exclusive are both enabled. This requires a database of known conflicts (starting with well‑known crates like `tokio` with `rt` / `rt-multi-thread`). Also checks if a feature implies another but the implied one is explicitly listed.

- **`duplication.rs`** -- finds features enabled from multiple ancestors when one would suffice. For example, both `crate-a` and `crate-b` enable `serde/derive` independently, but if `crate-a` already requires it, `crate-b`'s flag is redundant (and could be removed from `Cargo.toml` to reduce clutter).

- **`bloat.rs`** -- calculates the "cost" of a feature by counting the additional crates it pulls into the dependency tree. Flags features that add significant weight (e.g., enabling `json` on `reqwest` adds `serde_json` when it's already present). Uses the difference in package counts with and without the feature.

### 6. Report Generation (`src/report.rs`)

Collects findings from analysis passes and renders them.

- **Terminal renderer**: uses `crossterm` or `ansi_term` for coloured output with icons and tree‑like layout.

- **Markdown renderer**: produces a table‑based summary plus detailed sections per crate.

- Both formats include actionable suggestions like "consider removing feature `X` from your `Cargo.toml`".

### 7. Utilities (`src/util.rs`)

Shared helpers:

- Graph traversal (iterating dependencies, collecting ancestors).

- Regex matching for crate name filtering.

- Caching of parsed manifests to avoid re‑reading files.

---

## Data Flow

```

User runs `cargo feature-lens`

                │

                ▼

        CLI parsing (clap)

                │

                ▼

   Obtain `cargo metadata` JSON

                │

                ▼

   Parse manifests of all packages

                │

                ▼

   Build enriched dependency graph

   (resolve features, track origins)

                │

                ▼

   Run analysis passes

   (unused, conflicts, duplication, bloat)

                │

                ▼

   Generate report (terminal / Markdown)

                │

                ▼

          Output to stdout / file

```

---

## Important Design Decisions

- **No compilation**: Everything is static. `cargo feature-lens` never invokes `rustc`. It relies on `cargo metadata` and on `Cargo.toml` parsing. This makes it fast enough to run in CI or in a pre‑commit hook.

- **No source code analysis (yet)**: The initial version uses only manifest information to guess feature usage. Future versions will parse Rust source files to detect `#[cfg(feature = "...")]` and provide definitive "unused" detection.

- **Best‑effort resolver**: Cargo's feature resolver is complex (especially edition 2021). We approximate the unification rules that cover the vast majority of cases. For precise edge cases, users can always fall back to `cargo tree --features ...`.

- **Extensibility**: Analysis passes are independent. New checks (e.g., security‑relevant feature misuse) can be added by implementing a simple trait.

---

## Configuration

The tool requires no configuration file. Filters and output mode are controlled entirely through CLI flags. Future plans may include a `feature-lens.toml` to define project‑specific ignore lists or custom conflict rules.

---

## Performance Considerations

- `cargo metadata` can be slow for very large workspaces. The command is run once and the result is cached in memory.

- Manifest parsing uses a lazy approach: only the `Cargo.toml` files of packages that appear in the resolve graph are read (dependencies that are not used are not parsed).

- The feature expansion algorithm is recursive, but depth is bounded by the package tree depth (typically < 20). Unification is done via `HashMap` merges, which is efficient.

---

## Testing Strategy

- **Unit tests**: for resolver logic, feature expansion, conflict detection heuristics.

- **Integration tests**: using small, hand‑crafted workspaces under `tests/fixtures/`. Run `cargo feature-lens` on them and assert the output contains expected warnings.

- **Benchmarks** (future): to ensure that analysis time does not grow unreasonably with the number of crates.

---

## Future Directions

- **`#[cfg]`‑aware analysis** -- parse source files with `syn` and match feature conditions to get definitive usage data.

- **JSON output** -- for integration with dashboards, CI reports, or `cargo-deny` hooks.

- **Interactive TUI** -- allowing users to explore the feature graph in real time, enable/disable features and see the effect.

- **Automatic fix suggestions** -- a `--fix` mode that can comment out or remove unused features from `Cargo.toml` files.