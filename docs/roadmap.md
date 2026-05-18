# Roadmap

This roadmap outlines planned and ongoing work for `cargo-feature-lens`.

## Current tasks (open issues)

- **#9** Add CI workflow for test and formatting validation — Ensure contributors' PRs run `cargo fmt`, `cargo clippy`, `cargo test`, and optional builds automatically. [`feature`, `infra`, `ci`, `good first issue`] 
- **#8** Expand report output with richer dependency and feature insights — Improve Markdown/JSON/terminal reports with additional insights, counts, summaries, and terminal formatting. [`enhancement`, `output`] 
- **#7** Implement real remote crate analysis for `--crate` — Support analyzing crates from crates.io without a local manifest. [`feature`, `advanced`, `output`] 

## Planned enhancements

- Remote crate resolution fully implemented
- Additional analysis passes (e.g., feature overlap metrics, optional dependency impact)
- Enhanced visualization for dependency features
- Release-ready builds and CI pipeline enhancements

## Contributor workflow improvements

- Ensure triage and labeling matches `docs/ISSUE_LABELS.md`
- Add issue-specific acceptance criteria and mirrored dependencies
- Maintain PR discipline for focused changes and updates to roadmap items

## Milestones

- Output safety and CI readiness
- Documentation readiness
- Release readiness
- Distribution readiness
- Advanced parser expansion
- Contributor workflow polish
