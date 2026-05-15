# Codex Workflow

This repo supports fully autonomous engineering loops for Codex and Claude with deterministic harness checks.

## One-Command Entry Points

```console
./scripts/harness-check.sh         # agent-pack + MCP + parity harness only
./scripts/autonomy-check.sh        # full chain: lint/test/bench/harness/smoke
./scripts/harness-check.sh --quick # reduced parity corpus
./scripts/autonomy-check.sh --quick
./scripts/harness-check.sh --dry   # dry-capable harness steps
./scripts/autonomy-check.sh --dry  # bench/harness dry checks only
```

## Full Autonomous Chain

`./scripts/autonomy-check.sh` runs:
1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. `./scripts/bench.sh`
5. `./scripts/harness-check.sh`

Expected success tail:
- `[bench] wrote: ...`
- `[harness] complete`
- `[autonomy] complete`

## Harness-Only Chain

`./scripts/harness-check.sh` runs:
1. `python3 ./scripts/validate_agent_pack.py`
2. `bash ./agent-pack/tests/mcp_smoke.sh`
3. `python3 ./scripts/parity_harness.py`

Expected success tail:
- `[agent-pack] validation complete`
- `[mcp-smoke] complete`
- `parity harness wrote ...`
- `[harness] complete`

## Exact Codex/Claude Harness Runbook

Run from repo root:

```console
git rev-parse --abbrev-ref HEAD
./scripts/harness-check.sh --dry
./scripts/harness-check.sh --quick
./scripts/harness-check.sh
```

If docs/examples source markdown or `.puml` changed, re-render and commit artifacts:

```console
for f in docs/examples/*.puml; do cargo run -- "$f"; done
cargo run -- --from-markdown docs/examples/README.md --output docs/examples/README_snippet_1.svg
python3 ./scripts/parity_harness.py --output docs/benchmarks/parity_latest.json
```

Pre-PR confidence chain:

```console
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
```

Required green markers before opening PR:
- `[harness] complete`
- `[autonomy] complete`
- `doc_examples.summary.failed = 0` in `docs/benchmarks/parity_latest.json`

## Codex and Claude Workflow Recipe

1. Author diagrams and skills under `agent-pack/**`.
2. Run `./scripts/harness-check.sh --quick` during iteration.
3. Run `./scripts/autonomy-check.sh --quick` before broad changes.
4. Run `./scripts/autonomy-check.sh` before handoff/PR.
5. Include `docs/benchmarks` + parity artifacts when behavior/perf changed.

## Troubleshooting

- `mcp runner returned empty response`:
  - Verify `agent-pack/bin/puml-mcp` is executable.
- `tool name mismatch` in validator:
  - Sync `agent-pack/.mcp.json` tools with runtime `TOOL_LIST` in `agent-pack/bin/puml-mcp`.
- `artifact content does not match current renderer output`:
  - Re-render docs examples and commit updated SVG artifacts.
- Dry-run sanity:
  - Use `./scripts/harness-check.sh --dry` and `./scripts/autonomy-check.sh --dry` to inspect planned execution.
