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

The 2026-05-24/25 merge wave completed the first-slice contracts:
- #1111 registry and scene-contract foundation
- #1112 typed RenderScene geometry contract
- #1113 visual quality gate foundation
- #1114 frontend source-map and feature-loss diagnostic spine
- #1115 PlantUML element registry and mixed-element graph foundation
- #1116 authored Rust file-size split program and warning guardrail
- #1117 shared diagnostics/output/text/style/geometry/directive utilities
- #1150 typed unsupported/deferred/malformed syntax variants

Do not pick those closed issues as assignments. Use the focused follow-ups:
- #590 renderer architecture epic
- #1181 enforce Rust file-size guardrail in CI
- #1182 return render artifacts with SVG and typed RenderScene
- #1183 eliminate remaining StatementKind::Unknown normalizer paths
- #1184 build shared style cascade
- #1185 add LSP workspace commands for renderScene/export/explainDiagnostic
- #592 finish hierarchical graph-layout adoption
- #593 shared orthogonal route channels
- #594 visual quality refinement gate
- #816/#1145 typed pre-SVG scene invariants in render paths
- #870 shared scene hooks and renderer invariants

Pick one issue only. Read its full body with `gh issue view <N>`. Then make the
smallest coherent change that advances that issue.

Rules:
- Preserve existing public behavior unless the issue explicitly changes it.
- Prefer shared contracts over one-off family patches.
- Do not add another audit document.
- If a visual change is involved, render PNG and inspect it.
- If project-board commands fail because the token lacks Projects v2 scope,
  ignore the board and continue with GitHub issues.
- Verify with the targeted tests for the files/docs you touched. For docs-only
  roadmap edits, run the parity-roadmap audit test if fixture references changed.

Suggested first implementation choices:
1. #1181 CI enforcement for the file-size guardrail: small, mechanical, high leverage.
2. #1182 graph-family render artifact: high value, keep adapters source-compatible.
3. #1183 typed unknown cleanup in one normalizer family: contained and testable.
4. #1184 style cascade call site: choose one or two migrated paths only.

Report back with:
- issue number
- files changed
- tests run
- any follow-up issues discovered
```
