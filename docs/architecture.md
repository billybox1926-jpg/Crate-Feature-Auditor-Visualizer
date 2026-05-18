# Architecture

`cargo-feature-lens` is a lightweight, dependency-auditing CLI with a small library core. The end-to-end flow is:

1. Parse CLI options in `src/main.rs`.
2. Load Cargo's resolved dependency/feature graph via `cargo metadata` in `src/metadata.rs`.
3. Parse the manifest subset needed for conservative feature reasoning in `src/manifest.rs`.
4. Build an enriched, deterministic feature graph in `src/resolver.rs`.
5. Run analysis passes in `src/analysis/`.
6. Render terminal, Markdown, JSON, DOT, or Mermaid output in `src/report.rs` and `src/graph_export.rs`.

## CLI surface and behavior

`src/main.rs` handles:

- cargo-subcommand argument normalization
- input selection (`--manifest-path`, crate targeting)
- output format selection (terminal, markdown, json, dot, mermaid)
- finding filters (`--min-severity`)
- CI/automation mode (`--check` + `--fail-on`)
- optional output file writing
- built-in rule loading from `docs/suggestions.json` plus optional local `feature-lens.toml` overrides/extensions

`--check` returns a non-zero exit code when visible findings meet or exceed the chosen severity threshold, which keeps rule-driven and structural findings usable in CI gates.

## Metadata + manifest model

The resolver treats `cargo metadata` as the source of truth for package graph edges and active features, including resolver-v2 unification behavior.

Manifest parsing is intentionally conservative and dependency-free. It enriches metadata with:

- declared package features and default feature composition
- optional dependencies and dependency-feature forwarding
- workspace dependency inheritance (`workspace = true`)
- target-specific dependency tables (kept deterministic without full cfg evaluation)

This is a pragmatic auditing model, not a full Cargo/Rust semantic reimplementation.

## Feature graph and findings

`src/resolver.rs` stores deterministic graph data (ordered maps/sets and sorted lists) so repeated runs produce stable output.

Analysis passes currently include:

- unused feature detection (including source-aware suppression when feature usage is observed in project source)
- duplicate feature-lineage detection
- conflict rule matching
- bloat rule matching
- default-feature opt-out suggestions

Findings are normalized into shared types (`Severity`, `FindingKind`) and summarized before reporting.

## Outputs and exports

`cargo-feature-lens` supports:

- **Terminal** output for interactive auditing
- **Markdown** output for PR/issues
- **JSON** output for scripting and dashboards
- **DOT** graph export
- **Mermaid** graph export

Across output modes, the same filtered finding set and summary counts are used, keeping terminal/markdown/json views consistent.

## Rule sources

Built-in curated guidance lives in `docs/suggestions.json` and intentionally stays small/high-confidence. Local project teams can add or override rules through `feature-lens.toml` in the working directory.

## Scope posture

The project is feature-complete for its intended lightweight auditor scope: dependency-feature visibility, conservative findings, deterministic reports, local rule customization, remote crate analysis support, graph exports, and CI-friendly check behavior. Future work is expected to be incremental expansion rather than core architectural rewrite.
