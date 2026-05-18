# Roadmap

`cargo-feature-lens` is now maintenance-ready for its intended lightweight auditing scope.

## Current status

Core implementation is complete and usable today:

- terminal/Markdown/JSON reporting with deterministic finding summaries
- DOT and Mermaid graph exports
- `--check` severity gating for CI
- severity filtering (`--min-severity`)
- built-in curated rule guidance plus local `feature-lens.toml` rules
- default-feature analysis and source-aware unused-feature detection
- remote crate analysis and resolver-aware feature graphing

Completed foundational tasks (including earlier work tracked as #8 and #9) are no longer treated as active roadmap debt.

## Ongoing maintenance priorities

- Keep `docs/suggestions.json` curated, high-confidence, and fixture-backed
- Preserve deterministic output behavior across terminal/Markdown/JSON and graph exports
- Maintain docs accuracy as behavior evolves
- Keep CI and lint/test/build gates healthy

## Optional future expansion

Future work is additive and optional, not required for baseline usability:

- expand the rule database conservatively as new high-confidence cases are validated
- add focused analysis passes with clear false-positive boundaries
- improve graph/report ergonomics for larger workspaces
- add release/distribution polish where it reduces maintenance overhead

## Contributor guidance

- Prefer small, reviewable PRs tied to explicit acceptance criteria
- Update fixtures/tests alongside rule or reporting changes
- Treat roadmap additions as new scope proposals, not unfinished core completion work
