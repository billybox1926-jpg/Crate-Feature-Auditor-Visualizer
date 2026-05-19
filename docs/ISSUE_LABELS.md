# Issue Label Taxonomy

This repo uses labels as a lightweight project-management system. Labels should make an issue understandable at a glance: what kind of work it is, what area it touches, how complex it is, and whether it is suitable for a new contributor.

Use labels as structured metadata, not decoration. A tidy issue usually has one work-type label, zero or more area labels, and optional complexity or automation labels.

## Label principles

- Prefer a small number of accurate labels over a large pile of vague ones.
- Use labels to clarify scope, not to repeat the title.
- Keep dependency details in the issue body with `Depends on` and `Blocks` sections.
- When an issue changes shape, update the labels and relationship notes together.
- Do not use labels as a substitute for acceptance criteria.

## Work-type labels

Work-type labels describe the kind of change being made. Most issues should have exactly one of these.

| Label | Use when |
| --- | --- |
| `bug` | Existing behavior is broken, incorrect, or misleading. |
| `feature` | The issue adds new user-facing or workflow-facing capability. |
| `enhancement` | The issue improves existing behavior without creating a major new capability. |
| `documentation` | The primary work is docs, examples, README/wiki cleanup, or contributor guidance. |
| `maintenance` | The work is repository upkeep, cleanup, dependency/config refreshes, or non-feature polish. |

## Area labels

Area labels describe where the work lands. Use them when they add useful routing or review context.

| Label | Use when |
| --- | --- |
| `analysis` | The issue touches analysis passes, finding logic, rules, severity handling, or feature-risk detection. |
| `output` | The issue changes terminal output, Markdown, JSON, DOT, Mermaid, file output, or machine-readable reporting. |
| `ci` | The issue touches GitHub Actions, check mode, release gates, automation workflows, or CI guardrails. |
| `infra` | The issue affects project structure, packaging, repository settings, release setup, or maintainability infrastructure. |
| `dependencies` | The issue or PR updates Cargo dependencies, GitHub Actions versions, or dependency-management configuration. |

## Complexity and contributor-fit labels

These labels describe how approachable or risky the work is. They are not work types.

| Label | Use when |
| --- | --- |
| `good first issue` | The issue is scoped, low-risk, and includes enough detail for a new contributor to start. |
| `advanced` | The issue requires deeper project context, touches riskier internals, or may need broader design judgment. |

## Automation/source labels

Automation labels explain where work came from or who/what is expected to act on it. Keep these sparse.

| Label | Use when |
| --- | --- |
| `codex` | The work was opened, implemented, reviewed, or materially assisted by Codex. |

## Relationship labels vs relationship sections

Labels describe type, area, complexity, and source. They do not replace dependency tracking.

Use the issue body for dependency relationships:

```markdown
## Depends on
- #12 resolver accuracy, because this report behavior depends on feature provenance being stable.

## Blocks
- #19 graph exports, because visualization output should build on stable report data.
```

When issue A says it blocks issue B, issue B should also say it depends on issue A. Mirrored relationships keep the tracker useful when contributors read issues in isolation.

## Recommended label combinations

| Issue type | Suggested labels |
| --- | --- |
| CLI feature | `feature` plus an area label such as `analysis`, `output`, or `ci` |
| Analysis pass | `feature`, `analysis`; add `advanced` when resolver or provenance behavior is involved |
| Report/export change | `enhancement`, `output` or `feature`, `output` depending on scope |
| README/wiki/docs cleanup | `documentation`; add `maintenance` for stale-reference cleanup |
| Beginner-friendly docs task | `documentation`, `good first issue` |
| Repository upkeep | `maintenance`; add `infra`, `ci`, or `dependencies` when helpful |
| Dependency update | `maintenance`, `dependencies` |
| Automation-assisted work | Add `codex` only when the label helps explain the work source or review context |

## Triage checklist

Before leaving an issue open, confirm:

- [ ] The issue has one clear work-type label.
- [ ] Area labels describe the part of the project being touched.
- [ ] Complexity or contributor-fit labels are accurate.
- [ ] Automation/source labels are used only when helpful.
- [ ] The issue has clear acceptance criteria.
- [ ] Dependency relationships are mirrored where relevant.
- [ ] The issue has a milestone when it belongs to a known phase.
- [ ] The issue is small enough to close in one focused pull request, or explicitly explains why it is larger.
