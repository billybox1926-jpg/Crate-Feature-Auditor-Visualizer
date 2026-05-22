# Release Notes

## v0.1.0 - first tagged release

Cargo Feature Lens (`cargo-feature-lens`) is ready for a first tagged release as a conservative Cargo feature-audit helper.

Release target: create a GitHub release and publish the crate to crates.io after final maintainer validation. If publishing to crates.io is intentionally delayed, the GitHub release should say so clearly and should not imply crates.io availability.

## Highlights

- Cargo subcommand: run as `cargo feature-lens` after installation.
- Resolved feature graph: uses `cargo metadata` as the source of truth for resolved packages and active features.
- Audit findings: reports unused feature candidates, duplicate feature activation, configured conflicts, configured bloat signals, and default-feature review hints.
- Feature provenance: records requester lineages so reports can explain where active features came from.
- Output formats: terminal, Markdown, JSON, Graphviz DOT, and Mermaid.
- Check mode: supports `--check`, `--fail-on`, and `--min-severity` for local or CI-style gating.
- Built-in rules: embeds the built-in `docs/suggestions.json` rule database in the binary so installed usage works outside the repository checkout.
- Local rules: supports additive project-local rules through `feature-lens.toml` in the current working directory.
- Remote crate probe: supports best-effort crates.io analysis with `--remote --crate NAME` and optional `--crate-version VERSION`.

## Scope and limits

Cargo Feature Lens is an audit helper, not a proof system. Findings are review signals and should be checked by a maintainer before dependency features are removed or changed.

The tool does not compile the target project. It does not replace Cargo, `cargo check`, tests, security review, or project-specific dependency policy.

Source scanning is intentionally conservative. It looks for simple literal Rust `cfg` feature references and does not evaluate generated code, build scripts, macros, or every possible conditional compilation expression.

## Validation before release

Before tagging or publishing v0.1.0, run the local gate from `main`:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\check.ps1
cargo package --list
cargo publish --dry-run
```

The local gate covers:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

## Known follow-up work

The following work is intentionally not part of the v0.1.0 release scope:

- #59: harden remote crate input validation and source-scanner boundaries.
- #54: optional broader human code review follow-up, if not completed before the tag.

Do not present these as completed release scope.

## Review posture

Development was AI-assisted in places, with released changes reviewed, tested, and accepted under human maintainer responsibility.
