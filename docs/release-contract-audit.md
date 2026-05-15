# Release Contract Audit

Audit issue: `#18`  
Audit date: `2026-05-15` (America/Los_Angeles)

## Full Gate Contract (Deterministic Order)

`./scripts/check-all.sh` full mode must execute:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. `cargo llvm-cov --all-features --workspace --fail-under-lines 90`
5. `cargo build --release`
6. `./scripts/bench.sh --enforce-gates`

Quick mode contract:

- `./scripts/check-all.sh --quick` skips coverage + release build.
- Quick mode still enforces benchmark perf and binary-size gates.

## Contract Guards Added

- Script gate enforcement:
  - [x] `scripts/check-all.sh` full mode now includes `cargo build --release`.
- Deterministic regression checks:
  - [x] `tests/release_contract_audit.rs` validates required full-gate command ordering.
  - [x] `tests/release_contract_audit.rs` verifies release docs mention coverage + release build contract.
  - [x] `tests/fixtures/contract/release_gate_full_commands.txt` is the canonical command-order fixture.
- Documentation sync:
  - [x] `README.md` now documents full + quick gate usage.
  - [x] `docs/release-checklist.md` now includes explicit full-gate command contract.
  - [x] `docs/decision-log.md` includes D-014 for release-build validation policy.

## Remaining Known Deviations

- Coverage target is currently below policy per `docs/coverage-status.md` (`76.28%` vs `90%` target on the last recorded run). This remains a tracked gap and is not relaxed by this audit.
- Current quick/full gate execution is blocked by an existing parser unit test panic on `origin/main`:
  `parser::tests::parses_filled_virtual_endpoint_side_from_message_context` (index out of bounds in `src/parser.rs` test path).
