# Rule Authoring Guide

`docs/suggestions.json` contains the built-in rules used by Cargo Feature Lens for conflict, bloat, and default-feature findings. Treat it as product guidance, not as a scratchpad. Built-in rules should be conservative, broadly useful, and easy to explain in a report.

Use this guide when proposing or reviewing changes to the built-in rule set.

## Built-in rules vs local rules

Use `docs/suggestions.json` for rules that are likely to help many Rust projects. A built-in rule should describe a feature choice that is commonly risky, redundant, mutually exclusive, or worth reviewing across ordinary usage.

Use a project-local `feature-lens.toml` for policy that depends on one repository's target platform, performance goals, binary-size budget, security posture, or internal conventions. Local policy can be stricter, narrower, or more opinionated than built-in guidance.

When in doubt, start with a local rule. Promote it to a built-in rule only after the case is clear, repeatable, and useful beyond one project.

## What belongs in `docs/suggestions.json`

A good built-in rule should meet most of these conditions:

- The crate and feature combination is common enough to matter.
- The finding is high-confidence and unlikely to surprise maintainers.
- The recommendation does not depend heavily on one application's architecture.
- The message clearly explains what to review and why.
- The severity is conservative.
- The behavior can be covered by a small fixture or existing test.

Good examples include mutually exclusive TLS backend choices, broad convenience features that pull in large optional stacks, or default-feature opt-out suggestions that are useful for common constrained targets.

## What does not belong

Avoid built-in rules when the correct choice depends mainly on local context. Examples include:

- Rules that only make sense for one company, service, or binary-size budget.
- Rules that require measuring a specific application before they are meaningful.
- Rules based on guesses about a crate's internals without a clear feature-level signal.
- Rules that imply a feature is always wrong when it is only sometimes expensive.
- Large batches of speculative rules without fixture coverage.

Those cases belong in `feature-lens.toml` until they prove broadly useful.

## Rule messages

Every built-in rule should have a human-readable message or reason that works in terminal, Markdown, and JSON reports. Keep messages short and specific.

Prefer:

```text
TLS backends are mutually exclusive. Choose exactly one.
```

Avoid:

```text
This is bad.
```

A good message should tell the maintainer what to inspect, not pretend the tool has proven the final answer.

## Severity guidance

Use the lowest severity that still communicates the risk clearly.

- `info` — Review suggestion. Useful for default-feature opt-out hints and low-risk bloat guidance.
- `warning` — Likely cleanup opportunity or meaningful risk, but still requires maintainer judgment.
- `error` — Reserved for high-confidence conflicts or combinations that are almost certainly wrong.

Do not use `error` just because a feature is large or optional. Size and compile-time tradeoffs are often project-specific.

## Fixture and test expectations

New built-in rules should be fixture-backed where practical. A good fixture is small, deterministic, and built around one behavior.

Typical update flow:

1. Add or update the rule in `docs/suggestions.json`.
2. Add a small fixture under `tests/fixtures/<case-name>/` when the rule needs isolated coverage.
3. Add or update CLI tests in `tests/cli.rs` for terminal, Markdown, JSON, or `--check` behavior when relevant.
4. Keep expected output deterministic.
5. Run the standard project checks before opening a PR.

For documentation-only rule guidance changes, no fixture is required. For rule behavior changes, prefer at least one fixture or focused test.

## Review checklist

Before merging a built-in rule change, confirm:

- [ ] The rule is broadly useful, not only project-local policy.
- [ ] The message is clear, specific, and not overstated.
- [ ] The severity is conservative.
- [ ] The rule does not duplicate an existing rule.
- [ ] Fixture or test coverage exists where practical.
- [ ] The change keeps report output deterministic.
- [ ] Local-only policy remains in `feature-lens.toml` instead of `docs/suggestions.json`.
