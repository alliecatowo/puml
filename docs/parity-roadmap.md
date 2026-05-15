# Parity Roadmap

Date: 2026-05-15

This roadmap tracks high-impact sequence-diagram parity work relative to PlantUML behavior and defines execution order, measurable done criteria, and fixture-first delivery.

## Source Inputs

- Current parity research: `docs/parity-research-chunk-g-sequence.md`
- Contract decisions: `docs/decision-log.md`
- Coverage and test signals: `docs/coverage-status.md`, `tests/**`
- Existing fixture constraints for this track: `tests/fixtures/errors/` and `tests/fixtures/basic/` (only when needed)

## Delivery Principles

- Test-first: each backlog item starts with fixtures/tests that fail for the intended parity gap.
- Small blast radius: parser/normalizer/render changes ship in narrow slices with fixture proof.
- Contract clarity: if behavior is intentionally different, document it immediately in decision log + user docs.
- No silent UX drift: CLI flags, error text shape, and output artifact behavior are treated as parity surface.

## Staged Backlog (Execution Order)

### Stage 0: Baseline and Harness Lock (Gate Before Feature Work)

Items:
1. Freeze parity target list from `docs/parity-research-chunk-g-sequence.md` into issue/checklist form.
2. Confirm benchmark harness captures parser/render/check paths for current baseline binary.
3. Document fixture naming rules for this track (`invalid_*` for error fixtures; `valid_*` for positive basic fixtures).

Acceptance criteria:
- Backlog checklist exists in this roadmap with owner-ready, independently shippable slices.
- Baseline benchmark command set is documented and runnable without Java.
- No feature behavior changes in this stage.

Fixture plan:
- No new fixtures required.

### Stage 1: Syntax Compatibility Unblockers (P1)

Items:
1. Arrow syntax parity expansion (high portability blockers first).
2. Add `queue` participant role support.
3. Add `== separator ==` parsing + rendering behavior.
4. Make `hide/show footbox` visually effective in SVG output.

Acceptance criteria:
- Each syntax addition has at least one positive fixture and one negative fixture where applicable.
- Existing valid syntax behavior remains green.
- Render-affecting changes have deterministic snapshot coverage.
- CLI exits non-zero with actionable diagnostics for malformed new syntax paths.

Fixture-first plan:
- TODO fixture candidates (add when implementation begins):
  - `tests/fixtures/basic/valid_separator_equals.puml`
  - `tests/fixtures/errors/invalid_separator_unbalanced_equals.puml`
  - `tests/fixtures/basic/valid_participant_queue.puml`
  - `tests/fixtures/errors/invalid_participant_queue_alias_collision.puml`
  - `tests/fixtures/basic/valid_arrows_extended_set.puml`
  - `tests/fixtures/errors/invalid_arrow_variant_tokenization.puml`
- Footbox behavior may require render snapshot fixtures outside `basic/errors`; if so, track in implementation PR scope and keep this roadmap updated.

### Stage 2: Semantic Fidelity and Behavioral Depth (P2)

Items:
1. Group semantic validation parity (`alt/else/opt/loop/par/critical/break/group/end` matching rules).
2. Autonumber format and restart parity expansion.
3. Virtual endpoint fidelity improvements (`[`, `]`, `[*]`, found/lost directionality semantics).

Acceptance criteria:
- Mis-nested and mismatched group constructs fail with deterministic, user-facing errors.
- Valid complex group scenarios render with stable structure and separator behavior.
- Autonumber options covered by acceptance fixtures for start/stop/restart/format variants in scope.
- Incoming/outgoing endpoint semantics documented and test-covered for accepted vs rejected forms.

Fixture-first plan:
- TODO fixture candidates:
  - `tests/fixtures/errors/invalid_group_mismatched_end_keyword.puml`
  - `tests/fixtures/errors/invalid_group_else_without_alt.puml`
  - `tests/fixtures/basic/valid_group_nested_mixed_fragments.puml`
  - `tests/fixtures/basic/valid_autonumber_restart_format.puml`
  - `tests/fixtures/errors/invalid_autonumber_bad_format_token.puml`
  - `tests/fixtures/basic/valid_virtual_endpoints_directional.puml`

### Stage 3: Policy and Styling Boundary Closure (P3)

Items:
1. Expand `skinparam` in deterministic sequence-focused increments.
2. Resolve preprocessor boundary ambiguity (`!include`, `!define`, `!undef`) with explicit contract.

Acceptance criteria:
- Supported `skinparam` keys are explicitly listed; unsupported keys fail predictably with stable diagnostics.
- Preprocessor behavior is either fully supported for declared subset or rejected consistently at documented boundary.
- Decision log entries capture rationale for every intentional non-parity boundary.

Fixture-first plan:
- TODO fixture candidates:
  - `tests/fixtures/basic/valid_skinparam_sequence_subset.puml`
  - `tests/fixtures/errors/invalid_skinparam_unsupported_key.puml`
  - `tests/fixtures/errors/invalid_preprocessor_disallowed_context.puml`
  - `tests/fixtures/basic/valid_preprocessor_allowed_subset.puml`

### Stage 4: Binary, CLI, and UX Parity Milestone (Cross-Cutting Release Gate)

Scope:
1. Binary behavior parity for core workflows (`render`, `--check`, `--dump model`, `--multi`).
2. CLI UX parity goals for diagnostics, exit codes, and multi-file ergonomics.
3. User-facing docs parity for supported/unsupported syntax visibility.

Acceptance criteria:
- Binary:
  - Release binary produced and benchmarked on agreed fixture corpus.
  - Parse success/render success metrics recorded for each fixture category.
- CLI:
  - Exit code matrix documented and verified (`0` success, non-zero deterministic failure classes).
  - Error messages reference line/construct where available and remain snapshot-stable.
  - `--multi` behavior and file emission semantics documented with examples.
- UX/docs:
  - README and docs include a concise "PlantUML parity status" table with supported/partial/unsupported tags.
  - At least one troubleshooting entry per major failure class introduced in stages 1-3.

Fixture-first plan:
- No new fixtures required by default; this stage validates behavior over prior stage fixtures.

## Benchmark Parity Tracking (No-Java Environment)

- Current baseline (available now): benchmark `puml` only for cold-start, parser, and render paths via `./scripts/bench.sh`.
- Environment constraint: Java is not required for baseline runs in this repo.
- PlantUML comparison rows: remain `TODO` until Java + PlantUML jar are available in the benchmark environment.
- Comparison method (documented now, execute later):
  1. Run identical fixture corpus through `puml` and PlantUML.
  2. Capture parse success, render success, and elapsed time per fixture.
  3. Append comparison rows to benchmark markdown with clear tool labels and timestamp.

## Track Status

- New placeholder fixtures were not added in this pass to avoid speculative failures without paired implementation.
- TODO fixture names above are execution-ready and should be introduced at the start of each corresponding implementation slice.
