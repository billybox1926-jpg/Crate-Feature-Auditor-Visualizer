# Architecture

`cargo-feature-lens` is split into a small CLI binary and reusable library modules. The core flow is:

1. Parse CLI options in `src/main.rs`.
2. Load Cargo's resolved graph with `cargo metadata` in `src/metadata.rs`.
3. Parse the manifest subset needed for feature analysis in `src/manifest.rs`.
4. Build an enriched feature graph in `src/resolver.rs`.
5. Run independent analysis passes from `src/analysis/`.
6. Render the selected report format in `src/report.rs`.

## CLI layer

`src/main.rs` intentionally stays thin. It handles cargo-subcommand argument normalization, option parsing, loading optional suggestion rules from `docs/suggestions.json`, selecting the report format, and writing either stdout or an output file.

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
- feature-relevant `[workspace.dependencies]` entries inherited with `workspace = true`
- feature-relevant target-specific dependency sections such as `[target.'cfg(unix)'.dependencies]`

The parser is deliberately small and best-effort. It does not evaluate target `cfg` expressions; target-specific dependency data is folded into the same optional-dependency and dependency-feature maps so reports remain deterministic across platforms. Workspace inheritance is resolved by walking to the nearest ancestor manifest with `[workspace.dependencies]` and merging inherited feature lists with member-local dependency features. Missing package manifests still use an empty fallback manifest so unavailable local files do not abort graph construction.

## Feature graph resolution

`src/resolver.rs` creates a `FeatureGraph` keyed by Cargo package ID. Each `FeatureNode` stores package identity, active features, available manifest features, optional dependencies, dependency feature requests, dependency IDs, and recorded feature sources.

Resolution treats Cargo metadata resolve nodes as the source of truth for active features and dependency edges. That means edition 2021 / resolver v2 unification decisions made by Cargo, including direct dependency feature requests, transitive feature forwarding, build dependencies, dev dependencies present in metadata, and workspace-member unification, are preserved instead of being recomputed by this crate.

Parsed manifest data is used mainly to enrich provenance. The resolver records sources for dependency feature requests (for example, `app/serde`) and for active feature expansions that forward to other package features (for example, `app/default -> serde/derive`). The graph uses `BTreeMap`/`BTreeSet` storage plus sorted vectors for dependencies and feature-source lists so report output remains deterministic.

Known best-effort edges remain intentionally small and dependency-free: the manifest parser does not evaluate target `cfg` expressions, does not model every Cargo table form, and does not distinguish build/dev/normal provenance when Cargo has unified multiple dependency kinds into one resolve node. In those cases, the active feature set still follows Cargo metadata, while the explanatory source labels are conservative hints for auditing.

## Analysis passes

Each file under `src/analysis/` implements one pass:

- `unused.rs` reports active features that are not referenced by the parsed manifest data.
- `duplication.rs` reports features requested by more than one unique ancestor.
- `conflicts.rs` applies conflict rules from `docs/suggestions.json`.
- `bloat.rs` applies bloat rules from `docs/suggestions.json`.

`src/analysis/mod.rs` owns shared data types (`AnalysisContext`, `Finding`, `FindingKind`, `Severity`) and merges pass output into a stable sorted list.

## Suggestion database

`docs/suggestions.json` is the canonical optional rule database. Conflict rules contain a crate name, feature set, severity, and message. Bloat rules contain a crate name, feature, optional `pulls_in` list, and message. For backwards compatibility with older checkouts, the CLI checks `docs/suggestions.json` first and then falls back to a root-level `suggestions.json` if the docs file is not present.

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