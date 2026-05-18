# cargo-feature-lens User Guide

This guide explains how to run `cargo-feature-lens`, read its reports, and turn findings into concrete `Cargo.toml` changes.

## Running an audit

From a Cargo package or workspace root:

```bash
cargo feature-lens
```

For a workspace outside the current directory, pass the manifest directory or manifest file:

```bash
cargo feature-lens --manifest-path path/to/workspace
cargo feature-lens --manifest-path path/to/workspace/Cargo.toml
```

The command uses `cargo metadata`, so it resolves the dependency graph without compiling your project.

## Choosing output formats

`cargo-feature-lens` supports three report formats:

```bash
# Human-readable terminal report.
cargo feature-lens --format terminal

# Markdown report for issues, PRs, and documentation.
cargo feature-lens --format markdown --output feature-report.md

# Machine-readable report for scripts and dashboards.
cargo feature-lens --format json --output feature-report.json
```

When `--output` is supplied without `--format`, the tool writes Markdown for backwards compatibility. Without `--output`, it prints the terminal report.

## Focusing an audit

Use filters when you already know the area you want to inspect:

```bash
# Show only findings about one crate name substring.
cargo feature-lens --crate serde

# Show only unused-feature findings.
cargo feature-lens --unused

# Show only bloat findings from suggestions.json.
cargo feature-lens --bloat

# Show only warning and error findings in the report.
cargo feature-lens --min-severity warning

# Fail CI when warning-or-higher findings are present.
cargo feature-lens --check --fail-on warning

# Analyze a crates.io crate through a temporary local probe manifest.
cargo feature-lens --remote --crate tokio --crate-version 1
```

Filters apply to both the rendered crate list and the findings included in each report. In remote mode, `--crate` names the crates.io package to resolve instead of narrowing the report; the rendered report includes that crate and its reachable dependency graph, with the temporary probe package removed.

## Interpreting findings

### Unused

An unused finding means a feature is active but is not referenced by the package's manifest-level feature expansion, dependency feature requests, or optional dependency names. This is a static manifest heuristic; source-aware `#[cfg(feature = "...")]` inspection is planned.

Common fixes:

- Remove the feature from your direct dependency declaration if you enabled it explicitly.
- Disable defaults with `default-features = false` and add back only the features you need.
- Confirm the feature is not intentionally used by code paths the current heuristic cannot see.

### Duplicate

A duplicate finding means more than one meaningful requester lineage in Cargo's resolved feature graph requested the same active crate feature. The auditor uses the enriched `FeatureGraph::feature_sources` data derived from `cargo metadata` as the source of truth, and the finding message lists the sorted requester paths that caused the warning. Cargo feature unification can make repeated requests valid, so duplication is a review signal rather than proof of a broken build; it may still point to redundant dependency declarations or unnecessary feature bloat.

Common fixes:

- Prefer enabling a feature in the highest-level crate that truly needs it.
- Remove redundant feature flags from crates that inherit the feature through another dependency path.
- Keep duplicate requests when each requester genuinely needs the feature independently.

### Conflict

Conflict findings are loaded from `suggestions.json`. They flag known combinations that are mutually exclusive, redundant, or suspicious for a specific crate.

Common fixes:

- Choose one feature from a mutually exclusive pair.
- Replace a lower-level feature with the higher-level feature that implies it.
- Add or adjust a project-local rule only after confirming the crate's documentation.

### Bloat

Bloat findings are also loaded from `suggestions.json`. They highlight optional features that commonly pull in large or duplicate dependency trees. When the rule lists `pulls_in`, the report includes those crate names so you can quickly inspect whether they are already present elsewhere.

### DefaultFeature

Default-feature findings come from the `default_features.suggest_opt_out` rules in `docs/suggestions.json`. They are advisory suggestions to review whether a dependency should opt out of default features; they are not proof that default features are always wrong or that the current configuration is broken.

The rule is evaluated against Cargo's resolved feature graph from `cargo metadata`, not by guessing from raw `Cargo.toml` text. The current heuristic emits a finding when a matching crate is present and the resolved active features include `default`, which indicates default-feature behavior is active for that crate. The finding uses the reason configured in `docs/suggestions.json` and defaults to `info` severity when a rule does not specify a severity.

Common fixes:

- Keep defaults enabled when they match the project requirements.
- Disable defaults with `default-features = false` if your code does not need them.
- Add back only the narrower features you need, such as a specific TLS backend.
- Measure before and after with compile-time and binary-size tools when the tradeoff is unclear.

## Example feature-trimming workflow

1. Run a broad audit:

   ```bash
   cargo feature-lens --format markdown --output before.md
   ```

2. Pick one crate with unused, duplicate, or bloat findings.

3. Inspect the relevant dependency declaration in your `Cargo.toml`.

4. Try a minimal change, such as removing one explicit feature or adding `default-features = false` plus required features.

5. Validate with your normal build and tests:

   ```bash
   cargo test
   ```

6. Re-run the audit and compare:

   ```bash
   cargo feature-lens --crate crate-name
   ```

## Using JSON in scripts

The JSON report includes:

- `crate_count`
- `total_active_features`
- `crates[]`
- per-crate `active_features[]`
- per-crate `dependencies[]` and `optional_dependencies[]`
- per-crate `dependency_features` requested from dependencies
- per-crate `feature_sources` showing who requested each active feature
- per-crate `findings[]` with `kind`, `severity`, `feature`, and `message`

This makes it suitable for custom CI summaries or dashboards even before dedicated CI annotations are implemented.
