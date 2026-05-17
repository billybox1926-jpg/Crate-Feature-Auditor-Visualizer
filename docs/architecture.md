# Architecture

`cargo-feature-lens` is split into a small CLI binary and reusable library modules. The core flow is:

1. Parse CLI options in `src/main.rs`.
2. Load Cargo's resolved graph with `cargo metadata` in `src/metadata.rs`.
3. Parse the manifest subset needed for feature analysis in `src/manifest.rs`.
4. Build an enriched feature graph in `src/resolver.rs`.
5. Run independent analysis passes from `src/analysis/`.
6. Render the selected report format in `src/report.rs`.

## CLI layer

`src/main.rs` intentionally stays thin. It handles cargo-subcommand argument normalization, option parsing, loading `suggestions.json`, selecting the report format, and writing either stdout or an output file.

## Metadata loading

`src/metadata.rs` shells out to:

```bash
cargo metadata --format-version 1 --manifest-path <manifest>
```

The parser extracts only the package, workspace member, resolve node, feature, and dependency fields the resolver currently needs. This keeps the crate dependency-free while still allowing tests to exercise the expected metadata shape.

## Manifest parsing

`src/manifest.rs` reads each resolved package manifest and extracts:

- package name
- `[features]` entries
- default feature contents
- optional dependencies
- dependency feature requests

The parser is deliberately small and best-effort. Robust workspace inheritance, target-specific dependencies, and fallback behavior for unavailable manifests are tracked in `TODO.md`.

## Feature graph resolution

`src/resolver.rs` creates a `FeatureGraph` keyed by Cargo package ID. Each `FeatureNode` stores package identity, active features, available manifest features, optional dependencies, dependency feature requests, dependency IDs, and recorded feature sources.

Resolution currently combines Cargo's resolved active features with manifest feature expansion. It also records best-effort ancestor paths so duplicate feature requests can be reported with enough context to be actionable.

## Analysis passes

Each file under `src/analysis/` implements one pass:

- `unused.rs` reports active features that are not referenced by the parsed manifest data.
- `duplication.rs` reports features requested by more than one unique ancestor.
- `conflicts.rs` applies conflict rules from `suggestions.json`.
- `bloat.rs` applies bloat rules from `suggestions.json`.

`src/analysis/mod.rs` owns shared data types (`AnalysisContext`, `Finding`, `FindingKind`, `Severity`) and merges pass output into a stable sorted list.

## Suggestion database

`suggestions.json` is an optional rule database loaded from the current working directory. Conflict rules contain a crate name, feature set, severity, and message. Bloat rules contain a crate name, feature, optional `pulls_in` list, and message.

## Reporting

`src/report.rs` applies CLI filters and renders one of three formats:

- terminal for interactive use
- Markdown for issue and PR reports
- JSON for scripts, dashboards, and custom CI processing

Filters are applied before rendering, so all report formats expose the same selected findings.

## Test strategy

The repository uses:

- unit tests for parsing, rendering, and utility helpers
- a CLI integration test against `tests/fixtures/basic`
- fixture manifests and lockfiles that can be extended for additional resolver scenarios

Future tests should prefer small fixtures with explicit expected warnings so changes in Cargo feature behavior are easy to review.
