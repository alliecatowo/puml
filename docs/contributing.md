# Contributing

## Contribution Flow

1. Run one-time setup.

```console
./scripts/setup.sh
```

2. Create a branch and implement changes.
3. Run the fast loop while iterating.

```console
./scripts/dev.sh
```

4. Run the full gate before opening or updating a PR.

```console
./scripts/check-all.sh
```

5. If behavior changes, update docs and fixtures/snapshots as needed.
6. Add a decision-log entry for intentional contract changes in `docs/decision-log.md`.

## PR Readiness Checklist

- [ ] `./scripts/check-all.sh` passes locally.
- [ ] New behavior has tests and/or snapshots.
- [ ] Relevant docs are updated (`README.md` and `docs/**`).
- [ ] Benchmark artifacts refreshed when performance-sensitive code changed.
- [ ] Any intentional contract deviation is captured in `docs/decision-log.md`.

## Scope Reminders

- `puml` is sequence-diagram focused.
- Multi-diagram input requires explicit `--multi`.
- Unsupported/partial PlantUML features should fail clearly, not silently degrade.
