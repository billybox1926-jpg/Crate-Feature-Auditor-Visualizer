# Contributing to cargo-feature-lens

Thanks for helping improve `cargo-feature-lens`! This project aims to make Cargo feature behavior easier to understand without compiling a workspace.

## Getting started

```bash
git clone https://github.com/billybox1926-jpg/Crate-Feature-Auditor-Visualizer.git
cd Crate-Feature-Auditor-Visualizer
cargo test
```

Rust 1.70 or newer is required.

## Project layout

- `src/main.rs` — CLI parsing, cargo subcommand handling, and top-level orchestration.
- `src/metadata.rs` — `cargo metadata` invocation and lightweight metadata parsing.
- `src/manifest.rs` — best-effort `Cargo.toml` parsing for feature-related fields.
- `src/resolver.rs` — feature graph construction and source tracking.
- `src/analysis/` — independent analysis passes.
- `src/report.rs` — terminal, Markdown, and JSON report rendering.
- `tests/fixtures/` — small Cargo workspaces used by integration tests.
- `docs/suggestions.json` — optional conflict and bloat rule database (canonical path updated from root `suggestions.json`).

See `docs/architecture.md` for more detail.

## Code style

- Keep the CLI layer thin; prefer reusable logic in library modules.
- Add or update tests for behavior changes.
- Prefer deterministic ordering with `BTreeMap`/`BTreeSet` when output stability matters.
- Avoid panics in production paths; return errors with useful context.
- Keep report output stable unless the change is intentional and documented.

## Adding an analysis pass

1. Add a new module under `src/analysis/`.
2. Return `Vec<Finding>` from a focused `analyze(&AnalysisContext)` function.
3. Wire the pass into `analysis::run_all`.
4. Add unit tests or fixture-based CLI tests covering the new findings.
5. Update `README.md`, `TODO.md`, and `docs/guide.md` if users need to understand the new finding kind.

## Adding fixtures

Create a small workspace under `tests/fixtures/<name>/` with its own `Cargo.toml`, source files, and lockfile when needed. Keep fixtures narrow so a failing test points to one behavior.

## Pull request checklist

Before opening a PR, run:

```bash
cargo fmt
cargo test
```

Then confirm:

- [ ] The change is covered by tests or a clear manual check.
- [ ] User-facing behavior is documented.
- [ ] `TODO.md` is updated if a tracked task was completed or changed.
- [ ] New report output is deterministic.