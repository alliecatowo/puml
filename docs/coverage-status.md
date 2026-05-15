# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

Result:
- Gate: failed
- Total line coverage: 88.90%
- Target: 90%
- Gap: 1.10 points

Before/after vs previous baseline (2026-05-15):
- Total line coverage: `86.29% -> 88.90%` (`+2.61`)
- `normalize.rs` line coverage: `79.48% -> 82.26%` (`+2.78`)
- `theme.rs` line coverage: `93.55% -> 100.00%` (`+6.45`)
- `main.rs` line coverage: `75.51% -> 76.91%` (`+1.40`)

Top low-coverage modules from latest run:
- `main.rs`: 76.91% lines
- `normalize.rs`: 82.26% lines

High-confidence next lift areas:
- `main.rs`: hard-to-hit CLI and output error branches.
- `normalize.rs`: remaining lifecycle edge/error paths and uncommon group/directive branches.
- `layout.rs`: long-tail structure and geometry branches now near, but still below, 93% lines.

What changed in this pass:
- Added deterministic sequence skinparam subset expansion for `footbox` and `sequenceFootbox`.
- Added explicit unsupported-value warning policy: `[W_SKINPARAM_UNSUPPORTED_VALUE]`.
- Added focused coverage tests for normalize/theme skinparam paths and warning code determinism.

## Contract Audit Notes

Audit date: 2026-05-15

- Preprocessor/runtime documentation needs alignment: practical checks show `!include` executes with guardrails (not parse-only rejection), while unsupported directive/style paths still produce explicit diagnostics.
- Release validation should keep command UX and docs in lockstep by re-checking `--help`, exit codes, and warning behavior on each release pass.
