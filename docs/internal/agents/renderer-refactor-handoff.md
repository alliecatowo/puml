# Renderer Refactor Handoff Prompt

Use this prompt for the next implementation agent:

```text
You are working in `/Users/allie/Develop/puml`.

Read these first:
- `CLAUDE.md`
- `docs/internal/architecture/renderer-refactor-roadmap.md`
- the GitHub issue you are assigned

Do not use old parity audit ledgers, dated wave logs, or broad end-state parity
tickets as assignment truth. Current work is organized around focused GitHub
issues and executable tests/fixtures.

The fresh 2026-05-24 architecture cleanup created this issue backbone:
- #590 renderer architecture epic
- #1111 renderer architecture registry and scene contract
- #1112 typed RenderScene geometry contract before SVG
- #1113 visual quality gates and promoted corpus cases
- #1114 frontend source maps and explicit feature-loss diagnostics
- #1115 PlantUML element registry and mixed-element graph model
- #1116 authored Rust files under 600 lines with staged splits
- #1117 shared diagnostics/output/text/style/geometry/directive utilities

Pick one issue only. Read its full body with `gh issue view <N>`. Then make the
smallest coherent change that advances that issue.

Rules:
- Preserve existing public behavior unless the issue explicitly changes it.
- Prefer shared contracts over one-off family patches.
- Do not add another audit document.
- If a visual change is involved, render PNG and inspect it.
- If project-board commands fail because the token lacks Projects v2 scope,
  ignore the board and continue with GitHub issues.
- Verify with at least `cargo check --all-targets` plus the targeted tests for
  the files you touched.

Suggested first implementation choices:
1. #1117 diagnostics helper extraction: lowest risk and reduces CLI/LSP/metadata duplication.
2. #1116 warning-only line-count check: low risk and gives the refactor program a guardrail.
3. #1114 source-map wrapper around current frontend adapters: high value but more invasive.
4. #1112 typed geometry primitives: start tiny, with one renderer path only.

Report back with:
- issue number
- files changed
- tests run
- any follow-up issues discovered
```
