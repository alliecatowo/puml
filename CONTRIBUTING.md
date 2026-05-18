# Contributing to PUML

Welcome! PUML is an experiment in AI-driven development: most of the code lands via Claude Code agents working in parallel, with a human (allisonemilycoleman@gmail.com) orchestrating from above. Human contributions are very welcome too — both forms are first-class here, and the docs below treat them equally.

If you're here because you saw something broken, missing, or interesting and want to help — thank you. Read on.

---

## How contribution actually works

There are three ways to contribute, in roughly increasing weight:

### 1. File an issue

The lowest-friction contribution. Use the templates:
- **🐛 Bug report** — something PUML does wrong. Include a minimal `.puml` snippet, the command you ran, and what you expected vs. got. Render-bug reports with an attached PNG are gold.
- **✨ Feature request** — a PlantUML construct we don't yet cover, a CLI flag you wish existed, a quality-of-life improvement for the LSP or VS Code extension.
- **💬 [Discussions](https://github.com/alliecatowo/puml/discussions)** — open-ended ideas, show-and-tell, "how would I…" questions.

Don't filter yourself. We currently have 100+ open issues and welcome more — surfacing problems is the whole point.

### 2. Open a pull request

Real code changes, big or small. The PR template walks you through what we need:
- What the change does and why
- Type (bug fix / feature / breaking / visual / refactor / infra / docs)
- Linked issues (`Closes #N`)
- **Visual evidence for renderer changes** — even a path to the regenerated PNG works; you don't have to drag-drop an image
- Test plan (cargo test, clippy, fmt, parity harness, baseline blessing if applicable)
- Self-review checklist

We don't gatekeep on style or sophistication. Land the smallest change that fixes the thing, and we'll iterate from there.

### 3. Run an agent

This is what makes PUML different. If you're using Claude Code (or another agent harness) you can:
- Spawn a worker against an open issue: "fix issue #NNN" → the agent reads the linked context, drafts a fix, runs tests, opens a PR
- Run the multimodal visual audit loop: render the PNG corpus, ingest the images, file issues for visual flaws, dispatch fix waves
- Use the `agent-ready` label as a filter — those issues are written with self-contained context that an agent can act on

Agentic contributions follow the same PR template + CI gates as human PRs. The Claude Code agent already knows how to:
- Render PNGs and embed paths in the PR description
- Bless visual baselines after intentional changes
- Update the issue with progress comments
- Close the issue on merge

If you want to contribute agentically and don't know where to start, see [`docs/internal/agents/codex-workflow.md`](docs/internal/agents/codex-workflow.md) and [`docs/internal/agents/autonomous-workflow-cookbook.md`](docs/internal/agents/autonomous-workflow-cookbook.md).

---

## Quick start (for humans)

```bash
git clone https://github.com/alliecatowo/puml
cd puml
./scripts/setup.sh              # rustfmt, clippy, llvm-cov, etc.
./scripts/install-hooks.sh      # opt-in lefthook git hooks
cargo build --release

# render your first thing
echo '@startuml
Alice -> Bob: Hi
@enduml' | ./target/release/puml --format png - -o /tmp/hi.png
```

Before opening a PR:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release
python3 scripts/parity_harness.py --fail-on-doc-drift
```

If your change affects the renderer, also:

```bash
# regenerate the docs/examples SVG corpus
find docs/examples -name "*.puml" | while read f; do
  ./target/release/puml "$f" -o "${f%.puml}.svg"
done

# re-bless visual baselines for intentional visual changes
cargo test --release --test visual_regression bless_baselines -- --ignored
```

The PR gate runs all of the above automatically, so don't sweat it if you forget — the bot will tell you.

---

## What to work on

| Where to look | Why |
|---|---|
| **[Open issues with `agent-ready`](https://github.com/alliecatowo/puml/issues?q=is%3Aopen+label%3Aagent-ready)** | Triaged with enough context to act on |
| **[P0 issues](https://github.com/alliecatowo/puml/issues?q=is%3Aopen+label%3AP0)** | Critical correctness or visual bugs |
| **[The visual audit notes](docs/internal/visual-audit-2026-05-18.md)** | Per-PNG findings from the last multimodal sweep |
| **[Layout engine epic #590](https://github.com/alliecatowo/puml/issues/590)** | The biggest architectural lift — stages 1-4 are filed as children |
| **[`docs/internal/parity/`](docs/internal/parity/)** | PlantUML parity tracking — find a gap, fill it |

The biggest currently-open work areas:
1. **Visual perfection** — driving every diagram family to look as good as or better than Mermaid/PlantUML reference output
2. **Layout engine refactor** — replacing the per-family grid layouts with a real hierarchical / orthogonal-routing layout module
3. **PlantUML parity** — adding diagram families and syntax we don't yet support (Chen ER, Board, swimlanes, etc.)
4. **Polish** — error messages, CLI ergonomics, LSP completions

---

## Project conventions

A few things the agents (and humans) try to follow:

- **No PRs without tests** — if you change behavior, add a test. If you fix a bug, the test should reproduce the original bug.
- **Determinism** — same input must produce byte-identical output. Avoid `HashMap` iteration without sorting; sort all collections before serialization.
- **One logical change per commit** — easier to review, easier to revert.
- **Reference issues in commits** — `Fix #NNN: <one-liner>` for fixes; `Refs #NNN` for partial progress.
- **No `unwrap()` / `panic!()` on user input** — surface a `Diagnostic` instead.
- **Visual evidence for visual changes** — even just paths in the PR description.

Code style is enforced by `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings`. The PR gate fails if either complains.

---

## Agent-specific guidance

If you're running an agent against PUML:

- **Always render to PNG, not SVG** when doing multimodal visual verification. The Read tool triggers vision on raster formats; SVG is parsed as XML text and you'll miss visual flaws entirely.
- **Use `scripts/render_corpus.py --force`** to regenerate the whole PNG corpus into `target/audit_corpus/png/`.
- **Slice work by file locality** to minimize merge conflicts when running parallel workers. `src/render/family.rs` is contended; coordinate.
- **Skip the Codex sandbox** — agents that route through `codex` / `codex-rescue` / `apply_patch` have hit read-only filesystem errors. Use Claude Code's built-in Edit/Write/Bash tools directly.
- **Update issue status as you go** — `gh issue edit <N> --add-label in-progress` when you start, `gh issue close <N> --comment "<evidence>"` when you finish. The board drifts otherwise.

For the full agent runbook, see [`docs/internal/agents/`](docs/internal/agents/).

---

## Releases

PUML is pre-1.0. We tag a release when a notable change lands; security fixes always land on `main` and ship in the next tagged release.

- See [`docs/release-checklist.md`](docs/release-checklist.md) for the release process.
- Funding: [GitHub Sponsors / Ko-fi](.github/FUNDING.yml).

---

## Code of conduct

By participating in this project you agree to abide by the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

---

## License

PUML is MIT-licensed. By submitting a PR you agree your contribution is licensed under the same terms.

---

## Thanks

Whether you're a human, an agent, or both at once — thanks for being here.
