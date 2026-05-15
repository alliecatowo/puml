# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

Result:
- Gate: failed
- Total line coverage: 86.29%
- Target: 90%
- Gap: 3.71 points

Top low-coverage modules from latest run:
- `main.rs`: 75.51% lines
- `normalize.rs`: 79.48% lines

High-confidence next lift areas:
- `main.rs`: hard-to-hit CLI and output error branches.
- `normalize.rs`: lifecycle edge/error paths and rare directive/style branches.

## Contract Audit Notes

Audit date: 2026-05-15

- Preprocessor/runtime documentation needs alignment: practical checks show `!include` executes with guardrails (not parse-only rejection), while unsupported directive/style paths still produce explicit diagnostics.
- Release validation should keep command UX and docs in lockstep by re-checking `--help`, exit codes, and warning behavior on each release pass.
