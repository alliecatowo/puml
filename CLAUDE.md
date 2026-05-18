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
docs/                   Architecture docs, examples, parity tracking, visual audits
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
9. Regenerate affected `docs/examples/*.svg` artifacts and run parity drift check.
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
- The visual audit notes for the current cycle live at
  `docs/internal/visual-audit-<date>.md`.

---

## 6. Determinism and correctness invariants

These are hard rules, not suggestions:

- **No `HashMap` iteration without explicit ordering.** Use `BTreeMap` or sort keys
  before iterating. Nondeterministic output breaks the byte-identical output guarantee.
- **No `unwrap()` or `panic!()` on user input.** Return a `Diagnostic` with a helpful
  message. Panics on bad `.puml` files are bugs, full stop.
- **Same input → byte-identical output** (`DeterminismMode::Strict`). CI will catch
  violations via the oracle conformance suite.
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
| Agent runbook (deep) | `docs/internal/agents/codex-workflow.md` |
| Autonomous workflow cookbook | `docs/internal/agents/autonomous-workflow-cookbook.md` |
| Architecture + layout engine plan | `docs/internal/architecture/layout-engine-vision.md` |
| Current visual audit notes | `docs/internal/visual-audit-<date>.md` |
| PlantUML parity tracking | `docs/internal/parity/` |
| Human + agent contribution guide | `CONTRIBUTING.md` |
| Release checklist | `docs/release-checklist.md` |

---

## 9. Quick command reference

```bash
# Build
cargo build --release

# Lint + test (run before every main-branch commit)
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release

# Quick harness loop (agent-pack + MCP smoke + parity)
./scripts/harness-check.sh --quick

# Full autonomy chain (lint + test + bench + harness)
./scripts/autonomy-check.sh

# Render a single diagram to PNG
./target/release/puml --format png path/to/file.puml -o /tmp/out.png

# Regenerate full PNG audit corpus
python3 scripts/render_corpus.py --force

# Regenerate docs/examples SVG artifacts
find docs/examples -name "*.puml" | while read f; do
  ./target/release/puml "$f" -o "${f%.puml}.svg"
done

# Parity drift check (fail-fast)
python3 scripts/parity_harness.py --fail-on-doc-drift --quiet

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
- `doc_examples.summary.failed = 0` in `docs/benchmarks/parity_latest.json`

---

## 10. Current open epics

| Epic | Title |
|---|---|
| [#82](https://github.com/alliecatowo/puml/issues/82) | Truth-reset parity |
| [#88](https://github.com/alliecatowo/puml/issues/88) | Oracle conformance |
| [#89](https://github.com/alliecatowo/puml/issues/89) | CI hardening |
| [#399](https://github.com/alliecatowo/puml/issues/399) | Language service |
| [#590](https://github.com/alliecatowo/puml/issues/590) | Layout engine (stages 1-4) |

Check the epic body for child issues — that's where active implementation work is tracked.

---

*This file is authoritative for agent behavior. If it conflicts with another doc, this
wins — and file an issue so we can reconcile.*
