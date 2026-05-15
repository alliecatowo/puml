# Autonomous Workflow Cookbook

Codex/Claude command cookbook for repeatable branch-to-PR execution.

## Setup And Branching

```console
# optional isolated worktree flow
git worktree add ../puml-wave6 -b wave6-docs-harness
cd ../puml-wave6

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
git commit -m "docs+harness: canonical examples and drift audit gates"
git push -u origin wave6-docs-harness
```
