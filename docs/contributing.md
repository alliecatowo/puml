# Contributing

## Standard Development Flow

1. Run setup once.

```console
./scripts/setup.sh
```

2. Implement changes on a branch.
3. Run fast loop.

```console
./scripts/dev.sh
```

4. Run autonomous harness loop.

```console
./scripts/harness-check.sh --quick
```

5. Run full autonomous quality chain before PR.

```console
./scripts/autonomy-check.sh
```

## Autonomous PR Checklist

- [ ] `./scripts/harness-check.sh` passes.
- [ ] `./scripts/autonomy-check.sh` passes.
- [ ] `agent-pack` manifest/runtime contracts stay in sync (`validate_agent_pack.py`).
- [ ] Docs updated for command or contract changes.
- [ ] Any intentional contract deviation logged in `docs/decision-log.md`.

## Safety Constraints

- Do not use destructive git commands in automation scripts.
- Do not require hidden global dependencies beyond `bash`, `python3`, and Rust toolchain.
- Keep failure output deterministic and actionable.
