# Autonomous Workflow Cookbook

Codex/Claude command cookbook for repeatable branch-to-PR execution.

## Setup And Branching

```console
# isolated worktree flow (recommended)
git fetch origin
git worktree add ../puml-issue-131 -b feat/issue-131-docs-harness origin/main
cd ../puml-issue-131

# confirm branch state
git rev-parse --abbrev-ref HEAD
git status --short
```

## Fast Iteration Loop

```console
# quick harness loop (agent-pack + mcp smoke + parity)
./scripts/harness-check.sh --quick

# docs examples drift-only gate
python3 ./scripts/parity_harness.py --quick --quiet --fail-on-doc-drift
```

## Refresh Docs Example Artifacts

```console
# source-file examples
for f in docs/examples/*.puml; do cargo run -- "$f"; done
for f in docs/examples/*/*.puml; do [ -f "$f" ] && cargo run -- "$f"; done

# markdown fenced snippet artifacts
cargo run -- --from-markdown docs/examples/README.md --output docs/examples/README_snippet_1.svg
cargo run -- --from-markdown --multi docs/examples/sequence/README.md

# strict drift check
python3 ./scripts/parity_harness.py --fail-on-doc-drift --quiet
```

## Pre-PR Chain

```console
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
```

## PR Packaging

```console
git add -A
git commit -m "docs(harness): upgrade codex+claude runbook and gallery audit plumbing"
git push -u origin feat/issue-131-docs-harness
```
