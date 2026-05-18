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
- [ ] Any intentional contract deviation logged in `docs/internal/architecture-decisions.md`.

## Issues and discussions

Use [GitHub Discussions](https://github.com/alliecatowo/puml/discussions) for
questions, early ideas, showcases, parity reports that need discussion before
they become scoped work, and AI-swarm workflow notes.

Open an issue when there is a concrete bug, compatibility gap, docs task,
tooling task, or regression that someone can act on. Link back to the
discussion when a thread turns into follow-up work.

See [docs/discussions.md](discussions.md) for the full routing guide and
maintainer setup notes.

## Safety Constraints

- Do not use destructive git commands in automation scripts.
- Do not require hidden global dependencies beyond `bash`, `python3`, and Rust toolchain.
- Keep failure output deterministic and actionable.
