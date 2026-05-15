# Mermaid Sequence Parity + Differentiation Audit

Date: 2026-05-15
Ownership: `docs/parity-research-mermaid.md` only

## Scope

This audit compares Mermaid sequence-diagram behavior and Mermaid-oriented tooling expectations against current `puml` behavior, then proposes deterministic-native backlog items aligned with `puml`'s compiler architecture (parse -> normalize -> layout -> render + CLI contract).

## External Baseline (Authoritative)

- Mermaid sequence syntax and semantics: https://mermaid.js.org/syntax/sequenceDiagram
- Mermaid syntax structure and failure behavior: https://mermaid.js.org/intro/syntax-reference.html
- Mermaid runtime usage and parse-time validation API (`mermaid.parse`): https://mermaid.js.org/config/usage
- Mermaid CLI (`mmdc`) and operational model: https://github.com/mermaid-js/mermaid-cli
- GitHub Markdown Mermaid rendering support (fenced blocks): https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-diagrams
- GitHub code-block docs noting Mermaid support: https://docs.github.com/github/writing-on-github/creating-and-highlighting-code-blocks
- GitLab Markdown Mermaid support and version note: https://docs.gitlab.com/user/markdown/
- GitLab docs pipeline mermaid linting expectation: https://docs.gitlab.com/development/documentation/testing/

## `puml` Baseline (Current)

Implementation/docs evidence:
- `README.md`
- `src/parser.rs`
- `src/normalize.rs`
- `src/layout.rs`
- `src/main.rs`
- `docs/decision-log.md`
- `docs/troubleshooting.md`
- `tests/integration.rs`
- `tests/render_e2e.rs`

Key current strengths:
- Deterministic sequence-only contract with explicit exit codes and strict validation.
- Source-mapped diagnostics with line/column/caret snippets.
- Scriptable CLI with `--check`, `--dump ast|model|scene`, `--multi`, stdin/file symmetry.
- No JS runtime/browser dependency for rendering path.

## Mermaid Sequence Matrix vs `puml`

Legend:
- `Parity`: comparable user outcome.
- `Gap`: Mermaid feature expectation not yet matched.
- `Diff+`: intentional `puml` differentiator where we should be better for CI/compiler workflows.

| Area | Mermaid Baseline | `puml` Today | Status | Deterministic-native Direction |
|---|---|---|---|---|
| Markdown-first authoring | Mermaid is commonly embedded in fenced ```mermaid blocks on GitHub/GitLab | Accepts PlantUML-style text directly; no Mermaid fence ingestion mode | `Gap` | Add `--from-markdown` extractor with deterministic block ordering and source maps back to original `.md` offsets. |
| Participant declaration ergonomics | Implicit participants and order-by-appearance are core sequence behavior | Implicit participant creation already supported | `Parity` | Keep strict determinism; add optional diagnostics hint when implicit creation occurs. |
| Activation/deactivation shortcuts | Mermaid supports dedicated and shortcut activation syntax | Supported (lifecycle statements and arrow modifiers) | `Parity` | Preserve current strict lifecycle validation as a quality differentiator. |
| Group/control blocks (`alt/opt/loop/par/critical/break`) | Supported in Mermaid sequence syntax | Supported in `puml` normalized model/render path | `Parity` | Improve block-specific diagnostics and structural mismatch messaging. |
| Sequence numbering | Mermaid supports sequence numbers via config/directive | `autonumber` supported in `puml` | `Parity` | Add deterministic formatting controls parity where practical. |
| Actor menus/interactive links | Mermaid supports actor link/menu features in runtime contexts | No runtime interactivity features | `Diff+` | Keep non-interactive deterministic SVG default; consider explicit opt-in static link annotations only if reproducible. |
| Comments and escaped entities | Mermaid supports `%%` comments and entity escapes | Core parsing differs (PlantUML-oriented grammar) | `Gap` | Add a Mermaid-compat parser mode with deterministic canonicalization into existing AST. |
| Error handling contract | Mermaid `parse` can throw, or return `false` with suppress option | `puml` emits deterministic diagnostics + explicit exit codes | `Diff+` | Keep hard-fail defaults; add machine-readable diagnostic JSON output for pipeline UX parity and better-than-Mermaid automation. |
| CLI rendering stack | Mermaid CLI (`mmdc`) is Node/CLI-oriented and commonly paired with browser tooling | Native Rust CLI, no browser runtime dependency | `Diff+` | Emphasize offline, dependency-light, deterministic CI path as primary advantage. |
| Security/runtime variability | Mermaid runtime docs include securityLevel and browser rendering concerns | `puml` avoids JS runtime/sandbox variation in render path | `Diff+` | Continue deterministic offline rendering; document reproducibility guarantees (same input -> same SVG bytes under pinned version). |

## UX Features Mermaid Users Expect In Tooling Pipelines

These expectations are visible across Mermaid docs and Markdown platform docs:

1. Simple syntax acceptance.
- Users expect short, forgiving diagram text and quick parse feedback (Mermaid syntax docs + `mermaid.parse` behavior).

2. Markdown-friendly ergonomics.
- Users expect diagrams to live directly in Markdown comments/PRs/docs using fenced blocks (` ```mermaid ` on GitHub/GitLab).

3. Error readability close to authoring surface.
- Users expect parse failures to point clearly to offending lines, especially in docs CI (GitLab mermaid lint job pattern).

4. Automation-ready CLI behavior.
- Users expect easy batch conversion/validation and stable non-interactive invocation (`mmdc` usage model).

## Where `puml` Should Be Better Than Mermaid Tooling

1. Determinism as a first-class contract.
- `puml` can provide stronger guarantees than browser/runtime-based render paths: fixed normalization rules, fixed exits, stable snapshots.

2. Diagnostics engineered for CI.
- `puml` already has source spans; we should add structured diagnostics output (`--diagnostics json`) for machine triage, SARIF conversion, and annotation bots.

3. CLI-native automation.
- `puml` already has `--check`/`--dump`; we should add bulk modes and Markdown extraction to remove glue scripting.

4. Offline/no-runtime-deps reliability.
- Position native binary workflow as a deterministic alternative to JS/browser rendering stacks.

## Priority-Ranked Backlog (Deterministic-Native Equivalents)

## P1

1. Markdown Mermaid block ingestion mode.
- Problem: Mermaid users author in Markdown first; `puml` currently expects standalone diagram text.
- Proposal: `puml --from-markdown --check docs/**/*.md` extracts fenced `mermaid` blocks deterministically and maps diagnostics to source file line/column.
- Architecture fit: pre-parse extraction phase feeds existing parser/normalizer unchanged.
- Why now: biggest adoption unlock for Mermaid pipeline parity.

2. Structured diagnostics output.
- Problem: human-readable errors are good, but CI integrations need machine-readable payloads.
- Proposal: `--diagnostics json` with stable schema (`code`, `message`, `severity`, `file`, `line`, `column`, `snippet`).
- Architecture fit: extends `Diagnostic` emission in CLI layer, not parser semantics.
- Why now: immediate differentiation for automation and docs linting workflows.

3. Mermaid-compat parsing profile (sequence subset).
- Problem: syntax friction blocks migration from Mermaid-authored sequence sources.
- Proposal: optional `--syntax mermaid-sequence` profile that normalizes Mermaid-compatible constructs into current model where semantics overlap.
- Architecture fit: parser profile front-end -> existing normalize/layout stages.
- Why now: parity gain without abandoning sequence-only deterministic scope.

## P2

1. Batch workflow command.
- Problem: users need one command to validate many docs/diagrams with stable exit semantics.
- Proposal: `puml lint` or `puml check --glob` with deterministic file ordering and summary counts.
- Architecture fit: CLI orchestration only.

2. Deterministic compatibility reports.
- Problem: migration teams need to know what is unsupported before rollout.
- Proposal: `--report compatibility.json` listing accepted/rejected constructs per file + stable error codes.
- Architecture fit: aggregate diagnostics over existing parse/normalize results.

3. Message quality hints for implicit/ambiguous constructs.
- Problem: Mermaid-style permissiveness can hide intent.
- Proposal: warning class for implicit participants or ambiguous arrow forms with actionable fixes.
- Architecture fit: normalize-time warning emission.

## P3

1. Optional static link annotations in SVG.
- Problem: Mermaid supports interactive actor menus; some teams want navigation affordances.
- Proposal: deterministic, explicit opt-in static `<a>` wrapping for vetted constructs only; no script interactivity.
- Architecture fit: render-stage opt-in transformation.

2. Reproducibility attestation mode.
- Problem: CI/security teams want stronger artifact guarantees.
- Proposal: `--attest` outputs input hash + version + output hash manifest for each diagram.
- Architecture fit: CLI post-render metadata emission.

## Notes on Non-Goals

- Full Mermaid language parity is not required for this roadmap.
- Browser/runtime interactive features should not compromise deterministic/offline guarantees.
- Sequence-only scope remains intact; parity is targeted where it improves migration and authoring UX.

## References

- Mermaid sequence docs: https://mermaid.js.org/syntax/sequenceDiagram
- Mermaid syntax reference: https://mermaid.js.org/intro/syntax-reference.html
- Mermaid usage/config (`mermaid.run`, `mermaid.parse`): https://mermaid.js.org/config/usage
- Mermaid CLI (`mmdc`): https://github.com/mermaid-js/mermaid-cli
- GitHub Mermaid diagram docs: https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-diagrams
- GitHub code block + Mermaid support docs: https://docs.github.com/github/writing-on-github/creating-and-highlighting-code-blocks
- GitLab Markdown Mermaid support: https://docs.gitlab.com/user/markdown/
- GitLab docs testing (`docs-lint mermaid`): https://docs.gitlab.com/development/documentation/testing/
