# CLAUDE.md — AI Agent Guide for PUML

This file is the entry point for every Claude Code agent working in this repo.
Read it fully before touching a file. For deeper runbooks, follow the links in section 8.

---

## 1. Project at a glance

PUML is an AI-driven PlantUML-compatible diagram renderer written in Rust. Most code
lands via Claude Code agents running in parallel, coordinated by a human orchestrator
(Allie). The codebase compiles to a native CLI binary, a WebAssembly module for the
in-browser editor, and an MCP server that exposes diagram rendering as a tool. The goal
is parity with PlantUML's output quality while adding a real layout engine, orthogonal
edge routing, and a first-class language service.

**Repo layout:**

```
src/                    Rust library + CLI entry point
  parser/               PEG-based .puml grammar
  normalize/            AST normalization passes
  preproc/              !include resolver (+ in-browser JS variant)
  render/               Per-family renderers + graph_layout.rs (Wave-21+)
  language_service.rs   Hover, completion, diagnostics, semantic tokens
  cli.rs                Command-line interface
crates/puml-wasm/       WASM build target
stdlib/                 Bundled PlantUML stdlib skins
site/                   In-browser editor (TypeScript + Vite)
extensions/             VS Code extension
agent-pack/             MCP server, agent skills, smoke tests
docs/                   Architecture docs, examples, compatibility notes, visual QA docs
scripts/                CI/automation helpers
tests/                  Integration tests + visual_baselines/
```

**Current architectural state (post-Wave-21):**

- Hierarchical layout module live at `src/render/graph_layout.rs` (stage 1 complete)
- Orthogonal edge routing wired into sequence and class diagram families
- In-browser `!include` resolver ships in the JS preproc layer (`site/`)
- Language service APIs unified: hover, completion, diagnostics, semantic tokens,
  formatting — all accessible via MCP and the VS Code extension
- Coverage ratchet active in CI; oracle conformance suite running against PlantUML

---

## 2. Branch and commit conventions

### Branch naming

Every implementation branch:

```
<type>/issue-<NNN>-<short-slug>
```

`<type>` must be one of: `fix`, `feat`, `refactor`, `chore`, `docs`, `test`, `ci`.
Slug: kebab-case, 40 chars max, no trailing dash.

Examples:
- `fix/issue-467-class-hollow-triangle`
- `feat/issue-590-layout-engine-stage-2`
- `chore/issue-560-mindmap-newline-escape`

Multi-issue sprint branches (no single owning issue):

```
chore/wave-<NN><A|B|C>-<short-slug>
```

Example: `chore/wave-19-cov-ratchet`

### Commit messages

Conventional commits format, always:

```
<type>(<scope>): <subject line — imperative, ≤72 chars>

<body: what changed and why>

Refs #NNN
Closes #NNN  (only when this commit fully resolves the issue)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Valid scopes: `render`, `parser`, `normalize`, `layout`, `preproc`, `cli`, `lsp`,
`wasm`, `site`, `ci`, `docs`, `tests`.

---

## 3. Agent workflow — the standard loop

When you receive an issue number, execute these steps in order:

1. `gh issue view <N>` — read the full body including acceptance criteria.
2. Comment that work is starting:
   `gh issue comment <N> --body "Started in branch <branch-name>"`
3. The harness creates your worktree/branch (isolation:worktree). Verify with
   `git rev-parse --abbrev-ref HEAD`.
4. If the issue is visual, render the current output to PNG and read it with the
   Read tool to confirm the problem:
   `./target/release/puml --format png <fixture>.puml -o /tmp/before.png`
5. Implement the minimum correct fix. One logical change per commit.
6. Re-render to `/tmp/after.png` and Read it — confirm the fix visually before
   writing any baselines.
7. Run the full test + lint chain (see section 9).
8. Bless visual baselines only after visual confirmation:
   `cargo test --release --test visual_regression bless_baselines -- --ignored`
9. Regenerate affected `docs/examples/*.svg` artifacts and run docs render check.
10. Commit with a conventional message including `Closes #NNN`.
11. Report back: commit SHA, file paths changed, Read-tool description of the PNG diff.

---

## 4. Git ops conventions

- **Every change needs a ticket.** Run `gh issue create` before starting if one
  doesn't exist.
- **Label on start, close on finish:**
  `gh issue edit <N> --add-label in-progress`
  `gh issue close <N> --comment "<evidence>"`
- **Always rebase; never merge main into a feature branch.** Linear history is
  required.
- **Force-push to your own branch is fine; never to main.**
- **Direct-to-main is the norm for the orchestrator.** PRs are for CI-gated changes
  or major-version dependency updates. If you're an implementer agent, open a PR and
  let the orchestrator merge.
- **Before any commit that touches main**, the full chain must be green:
  `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test --release`

---

## 5. Multimodal visual audit pattern

- Always render **PNG**, not SVG, for visual verification. The Read tool triggers
  vision on raster; SVG is parsed as XML text and visual bugs are invisible.
- Full corpus regeneration:
  `python3 scripts/render_corpus.py --force`
  Output lands in `target/audit_corpus/png/`.
- One-off render:
  `./target/release/puml --format png <file>.puml -o /tmp/v.png`
  Then `Read /tmp/v.png`.
- Visual audit findings should become live GitHub issues, regression fixtures,
  visual baselines, or updates to current architecture docs. Do not keep dated
  wave logs around after triage; stale visual-audit notes are pre-v1 clutter.

---

## 6. Determinism and correctness invariants

These are hard rules, not suggestions:

- **No `HashMap` iteration without explicit ordering.** Use `BTreeMap` or sort keys
  before iterating. Nondeterministic output breaks the byte-identical output guarantee.
- **No `unwrap()` or `panic!()` on user input.** Return a `Diagnostic` with a helpful
  message. Panics on bad `.puml` files are bugs, full stop.
- **Same input → byte-identical output.** Rendering is unconditionally deterministic
  (BTreeMap/sorted-key discipline); there is no determinism mode toggle.
- **Bless baselines only after visual inspection.** Never bless to make a red test
  green without first reading the rendered PNG.

---

## 7. Anti-patterns

Do not do these:

- **Do not route through `codex` or `codex-rescue`** from inside a Claude Code agent.
  The sandboxed runtime hits read-only filesystem errors. Use Edit/Write/Bash directly.
- **Do not commit auto-regen PNG artifacts** under `docs/examples/**/*.png` — they are
  gitignored. Committed artifacts live only under `docs/diagrams/` and
  `tests/visual_baselines/`.
- **Do not bypass the coverage gate or disable tests** to make CI pass. Fix the root
  cause.
- **Do not reintroduce removed features.** Check the git log before adding anything
  that sounds like "dual chart renderer" or "--lsp-capabilities manifest".
- **Do not bless baselines without reading the PNG first** — even if the test name
  sounds innocuous.
- **Do not add `#[allow(clippy::...)]` suppressions** without a comment explaining why
  the lint is a false positive in this specific case.

---

## 8. Where to look

| What you need | Where to find it |
|---|---|
| **PlantUML language reference (canonical)** | `docs/internal/spec/PlantUML_Language_Reference_Guide_v1.2025.0.pdf` — the upstream spec PDF; read this for any syntax question, color/icon example, or rendered-output comparison |
| Renderer refactor roadmap | `docs/internal/architecture/renderer-refactor-roadmap.md` |
| PlantUML reference material | `docs/internal/spec/PlantUML_Language_Reference_Guide_v1.2025.0.pdf` and `docs/internal/spec/plantuml-spec.md` — useful references, not automatic assignment sources |
| Agent runbook (deep) | `docs/internal/agents/codex-workflow.md` |
| Autonomous workflow cookbook | `docs/internal/agents/autonomous-workflow-cookbook.md` |
| Architecture + layout engine plan | `docs/internal/architecture/layout-engine-vision.md` |
| Visual audit pipeline | `docs/internal/visual-audit-pipeline.md` |
| Human + agent contribution guide | `CONTRIBUTING.md` |
| Release checklist | `docs/release-checklist.md` |

### Always check current evidence before implementing

Before touching parser, normalize, or render code for any diagram family, read:
1. The owning GitHub issue body and acceptance criteria
2. The relevant tests/fixtures/examples that already exercise the behavior
3. The matching section of the upstream PlantUML reference when parity matters
4. `docs/internal/architecture/renderer-refactor-roadmap.md` for renderer,
   frontend, source-map, scene, geometry, and invariant work

Do not treat legacy parity ledgers or dated audit notes as assignment truth.
If they conflict with tests, examples, or current issues, fix or retire the stale
doc instead of coding to it.

---

## 9. Quick command reference

```bash
# Build
cargo build --release

# Lint + test (run before every main-branch commit)
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release

# Quick harness loop (agent-pack + MCP smoke + render check)
./scripts/harness-check.sh --quick

# Full autonomy chain (lint + test + bench + harness)
./scripts/autonomy-check.sh

# Render a single diagram to PNG
./target/release/puml --format png path/to/file.puml -o /tmp/out.png

# Regenerate full PNG audit corpus
python3 scripts/render_corpus.py --force

# Regenerate committed diagram artifacts (docs/diagrams SVG+PNG, docs/examples SVGs)
# After any renderer change that lands on main, run this and commit the result.
# The main-gate CI check-artifact-freshness step will catch stale artifacts.
scripts/regen-artifacts.sh --force
git add docs/diagrams/ docs/examples/
# then: git commit -m "docs(diagrams): regen artifacts after <change>"

# Regenerate docs/examples SVG artifacts (targeted, single-family)
find docs/examples -name "*.puml" | while read f; do
  ./target/release/puml "$f" -o "${f%.puml}.svg"
done

# Docs render check (fail-fast on drift)
python3 scripts/render_check.py --fail-on-doc-drift --quiet

# Refresh the committed render-check report intentionally
python3 scripts/render_check.py --fail-on-doc-drift --quiet --output docs/benchmarks/render_check_latest.json

# Bless visual baselines (after visual confirmation only)
cargo test --release --test visual_regression bless_baselines -- --ignored

# GitHub issue ops
gh issue view <N>
gh issue comment <N> --body "Started in branch <name>"
gh issue edit <N> --add-label in-progress
gh issue close <N> --comment "<evidence>"

# Pre-PR confidence chain
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
```

Required green markers before any merge to main:

- `[harness] complete`
- `[autonomy] complete`
- `python3 scripts/render_check.py --fail-on-doc-drift --quiet` exits `0`
- `summary.failed = 0` in `docs/benchmarks/render_check_latest.json` after an intentional report refresh

---

## 10. Current open epics

| Epic | Title |
|---|---|
| [#88](https://github.com/alliecatowo/puml/issues/88) | Oracle conformance |
| [#89](https://github.com/alliecatowo/puml/issues/89) | CI hardening |
| [#399](https://github.com/alliecatowo/puml/issues/399) | Language service |
| [#590](https://github.com/alliecatowo/puml/issues/590) | Renderer architecture and layout |

Check the epic body for child issues — that's where active implementation work is tracked.

---

## 11. Development Flows

This section is the authoritative encyclopedia of the flows this project uses. When in
doubt about which flow to use, read the "when to use" note for each.

### Flow A: Issue queue → file → fix → close

**When:** Standing triage loop; the orchestrator or an assigned triage agent walks
current GitHub issues between waves. Project board access requires a maintainer
token and is optional. Do not block implementation on `gh project` access.

1. `gh issue list --state open --label P0 --limit 30` — find high-priority open work
2. `gh issue view <N>` — read full body + acceptance criteria
3. If no issue exists yet for the bug you are about to fix, `gh issue create` first
4. Apply the implementation flow (B or C below)
5. `gh issue close <N>` on completion
6. After every wave, re-audit: `gh issue list` + render PNG corpus + spot-check visually

### Flow B: PR-per-agent (the default after Wave 22)

**When:** Any implementer agent doing feature work, bug fixes, refactors, test additions,
or documentation. **This is the default going forward.** The harness provisions a
worktree automatically via `isolation: "worktree"`.

1. Branch is created as `<type>/issue-<NNN>-<slug>` (harness does this)
2. Implement + commit using conventional-commits; include `Closes #<NNN>` in the commit
   footer
3. Push: `git push -u origin <branch>`
4. Open PR: `gh pr create --title "<type>(<scope>): <subject>" --body "..."` — include
   visual evidence and test plan; body must contain `Closes #<NNN>`
5. Enable auto-merge: `gh pr merge <PR> --auto --squash --delete-branch`
6. Label: `gh issue edit <N> --add-label in-review`
7. **Baby the PR**: poll `gh pr checks <PR>` until green; if CI fails, fix on the same
   branch and re-push; address every Copilot code-review comment
8. If `main` moves: `git fetch origin && git rebase origin/main`; resolve conflicts
   deliberately; `git push --force-with-lease`
9. After merge: GitHub auto-closes the issue via the `Closes #` link

Deep runbook: `docs/internal/agents/autonomous-workflow-cookbook.md`

### Flow C: Direct-to-main (orchestrator + emergencies only)

**When (and only when):**
- Emergency CI unblockers (cargo fmt drift, snapshot bless after intentional renderer
  change)
- Orchestrator merging parallel wave results into a single coherent commit on main
- Documentation-only commits authored by the user directly
- Reconciliation passes where multiple workers' branches need synthesis before any single
  PR makes sense

**Not for implementer agents.** If you are an implementer, use Flow B.

### Flow D: Checkpoint branches (multi-agent sprint synthesis)

**When:** 3+ parallel workers whose results need combining before merging, especially
when multiple workers will touch the same high-contention files (see section 12).

1. Orchestrator creates: `git checkout -b chore/wave-<NN>-checkpoint` from current main
2. Each worker opens a PR against the checkpoint branch (not main)
3. After all worker PRs merge to checkpoint, orchestrator opens ONE PR from checkpoint
   to main with synthesized result
4. Required for visual-render waves where multiple workers touch `render/family.rs`

### Flow E: Hotfix sprint

**When:** Bug found on origin/main that is blocking other work or blocking CI.

1. Branch from main: `git checkout -b fix/hotfix-<short>`
2. Minimum correct fix; one commit
3. Open PR, enable auto-merge, baby it through CI
4. After merge, file a follow-up ticket if root-cause analysis revealed something deeper

### Flow F: Coverage ratchet

**When:** Coverage falls below gate, or as a standing "always have a coverage worker in
every swarm wave" directive.

1. `cargo llvm-cov` — measure current %
2. Identify lowest-coverage modules not in the ignore-regex
3. Add MEANINGFUL tests (not synthetic fill) in 3-5 module batches
4. Bump `--fail-under-lines` threshold in `scripts/check-all.sh`
5. PR-per-agent (Flow B)
6. Always include a coverage-uplift worker in every swarm wave

### Flow G: Visual self-driving loop

**When:** After a render change, or on a scheduled visual-quality wave.

1. Regenerate PNG corpus: `python3 scripts/render_corpus.py --force`
2. Multimodal audit: spawn focused agents that inspect PNGs and report concrete
   fixture paths, file references, and suggested invariant tickets
3. Convert confirmed findings into live GitHub issues, regression fixtures, or
   architecture/visual-regression docs
4. Fire implementer wave (Flow B), grouped by file locality
5. After merges land, regenerate corpus, re-audit, loop until pixel-perfect

Do not retain dated wave-log notes after triage. They go stale quickly. Current
GitHub issues, tests, fixtures, visual baselines, and architecture docs are the
actionable record.

---

## 12. Concurrency and conflict patterns

- When 2+ workers will likely touch the same file, **queue them sequentially** OR use
  Flow D (checkpoint branch)
- `src/render/family.rs` — most contended file in the repo; always coordinate before
  assigning multiple workers to it in the same wave
- `src/render/graph_layout.rs` — second most contended; same rule applies
- A worker that discovers a merge conflict mid-rebase should **stop, report, and let the
  orchestrator resolve** rather than force-resolving and possibly silently dropping code
- **Never fire parallel agents on the SAME bug.** They produce competing PRs with
  opposite-direction fixes that conflict, force pauses, and waste hours. If first
  agent stalls, send a stronger SendMessage rather than firing a redundant one.

### When the orchestrator (Opus) implements directly vs delegates

Default: delegate. But these patterns force the Opus orchestrator to implement:

- **Layout-engine bugs that ≥2 Sonnet agents have failed on.** Sonnet excels at
  well-scoped "one bug → one file → one test" fixes. For "the whole layout looks wrong"
  problems requiring algorithm-level reasoning, Sonnet patches symptoms; Opus sees
  the geometry. (Lesson: arch-overview group-collision saga took 5 Sonnet agents + 10
  PRs before the Opus orchestrator personally shipped PR #864.)
- **Cross-file algorithm refactors where the diff is small but reasoning is deep.**
- **When user is in-loop watching and frustration is mounting** — delegation latency
  compounds frustration; Opus directly iterating with visual-gate-each-iteration is
  faster.

### Layout debugging: grep SVG coords first

For layout bugs (overlapping frames, touching packages, edge through node), grep the
rendered SVG for actual bbox coordinates BEFORE patching:

```bash
./target/release/puml docs/diagrams/architecture-overview.puml -o /tmp/arch.svg
grep -oE 'class="uml-group-frame"[^>]*' /tmp/arch.svg
```

Exact `x="…" y="…" width="…" height="…"` per frame is ground truth. Two frames with
matching dark header colors can look like they touch when there's actually a 40px gap.
Diagnose with coords, then re-verify the fix with PNG Read.

---

## 13. Memory and persistent notes

- Orchestrator memory: `/home/Allie/.claude/projects/-home-Allie-develop-puml/memory/`
- Visual audit findings belong in GitHub issues, focused fixtures, blessed visual
  baselines, or current architecture docs. Avoid persistent dated wave logs unless
  the orchestrator explicitly asks for a short-lived scratch note.
- Per-agent memory links are in `memory/MEMORY.md`; read before spawning subagents

---

## 14. CI gates that block merge

| Workflow | What it checks |
|---|---|
| `pr-gate.yml` | fmt, clippy, test, coverage (85% line gate), wasm build, docs drift, site smoke |
| `main-gate.yml` | Same as pr-gate + diagram drift safety net |
| `oracle.yml` | Differential conformance against PlantUML JAR |

Notes:
- `differential-svg-oracle` is currently **NOT** a required check; will become required
  once oracle JAR pinning is verified — do not treat a red oracle as a merge blocker yet
- Coverage gate is `--fail-under-lines 85` in `scripts/check-all.sh`; bump it in Flow F
- Docs drift check runs `python3 scripts/render_check.py --fail-on-doc-drift` without mutating tracked reports by default

---

## 15. Issue label taxonomy

| Label | Meaning |
|---|---|
| `P0` / `P1` / `P2` / `P3` | Priority (P0 = drop everything) |
| `agent-ready` | Has enough context for an agent to act without further clarification |
| `bug` | Regression or correctness defect |
| `enhancement` | New capability or behavior improvement |
| `refactor` | Internal restructure with no behavior change |
| `epic` | Parent tracking issue; child issues listed in body |
| `parity` | PlantUML compatibility gap |
| `visual-audit` | Found by a multimodal visual audit (Flow G) |
| `in-progress` | A branch exists and work has started |
| `in-review` | PR is open and awaiting CI + review |
| `architecture` | Touches the layout engine or other core architecture decisions |

---

*This file is authoritative for agent behavior. If it conflicts with another doc, this
wins — and file an issue so we can reconcile.*
