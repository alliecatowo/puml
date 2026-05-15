# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

Result:
- Gate: failed
- Total line coverage: 76.28%
- Target: 90%
- Gap: 13.72 points

Before/after vs previous baseline (2026-05-15):
- Total line coverage: `88.90% -> 76.28%` (`-12.62`)
- `main.rs` line coverage: `76.91% -> 78.05%` (`+1.14`)
- `normalize.rs` line coverage: `83.04% -> 84.80%` (`+1.76`)

Top low-coverage modules from latest run:
- `bin/puml-lsp.rs`: 26.93% lines
- `main.rs`: 78.05% lines

High-confidence next lift areas:
- `bin/puml-lsp.rs`: add focused coverage for high-branch LSP request handling.
- `main.rs`: keep adding CLI routing/error-path tests around rare output and diagnostics flows.
- `lib.rs`: improve line coverage in shared library entry points.

What changed in this pass:
- Added CLI coverage tests for `--check-fixture` warning JSON diagnostics payloads.
- Added stdin-empty validation coverage (`no diagram content provided` exit path).
- Added markdown auto-detect coverage for `.mdown` extension handling.
- Added focused normalize hotspot tests for malformed AST arrow rejection, `else`-inside-`loop` group validation, empty `!theme` warning formatting, and `maxMessageSize` supported no-op behavior.
- Added new focused fixtures: `tests/fixtures/errors/invalid_else_inside_loop_group.puml` and `tests/fixtures/basic/valid_skinparam_maxmessagesize.puml`.
- Wired new fixtures into integration coverage lists and deterministic diagnostic assertions.

## Perf/Binary Gate Relationship

- Coverage gate is enforced only in full `./scripts/check-all.sh` mode.
- Quick `./scripts/check-all.sh --quick` skips coverage but enforces benchmark perf + binary-size gates.
- Benchmark regressions are tracked in `docs/benchmarks/latest_trend.{md,json}` with deterministic scenario rows.

## Contract Audit Notes

Audit date: 2026-05-15

- Preprocessor/runtime documentation needs alignment: practical checks show `!include` executes with guardrails (not parse-only rejection), while unsupported directive/style paths still produce explicit diagnostics.
- Release validation should keep command UX and docs in lockstep by re-checking `--help`, exit codes, and warning behavior on each release pass.
- No-Java baseline remains intentional for oracle hooks; keep placeholder fields until explicit Java enablement.
