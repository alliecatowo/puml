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
4. Lock docs-example parity contract (`docs/examples/*.md` -> `.puml`/snippet -> `.svg` artifacts).

Acceptance criteria:
- Backlog checklist exists in this roadmap with owner-ready, independently shippable slices.
- Baseline benchmark command set is documented and runnable without Java.
- Parity harness report includes fixture results plus docs/examples parity status.
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

## Diagram Families Parity Program (Execution Slices)

This section defines family-by-family execution slices for the shared IR + layout-engine program.

### Slice A: Sequence hardening baseline

Scope:
1. Keep sequence as the only fully enabled family.
2. Move sequence onto shared family routing boundary without behavior drift.
3. Add deterministic tests for family-aware routing.

Done criteria:
- Existing sequence fixtures remain green.
- New family-routing API is additive and stable.
- Non-sequence families remain explicit deterministic rejections.

### Slice B: Class family bootstrap

Scope:
1. Implement class-family IR builder.
2. Implement minimal class layout engine for core nodes/edges.
3. Add class fixture corpus for accepted + rejected constructs.

Done criteria:
- `DiagramFamily::Class` round-trips basic class diagrams to SVG.
- Unsupported class constructs produce deterministic diagnostics.

### Slice C: State family bootstrap

Scope:
1. Implement state-family IR and transition normalization.
2. Add initial state layout strategy.
3. Cover nested states and transition labels with fixtures.

Done criteria:
- `DiagramFamily::State` handles baseline state diagrams deterministically.
- Invalid state forms are rejected with stable errors.

### Slice D: Activity family bootstrap

Scope:
1. Implement activity-family control-flow IR.
2. Add layout for actions, branches, joins, and terminal nodes.
3. Add deterministic render snapshots for representative flows.

Done criteria:
- `DiagramFamily::Activity` renders core flow constructs.
- Unsupported activity syntax is explicitly diagnosed.

### Slice E: Component family bootstrap

Scope:
1. Implement component-family node/interface IR.
2. Add layout for components, interfaces, and dependencies.
3. Add acceptance/error fixtures.

Done criteria:
- `DiagramFamily::Component` supports minimum useful subset.
- Deterministic scene ordering maintained.

### Slice F: Deployment family bootstrap

Scope:
1. Implement deployment-family node/artifact IR.
2. Add layout for nodes, artifacts, and links.
3. Add deterministic diagnostics for unsupported deployment details.

Done criteria:
- `DiagramFamily::Deployment` baseline diagrams render successfully.
- Fixture suite captures stable rejection boundaries.

### Slice G: Use-case family bootstrap

Scope:
1. Implement use-case IR for actors/use-cases/relations.
2. Add layout tuned for actor-to-use-case readability.
3. Add fixture-first acceptance and rejection tests.

Done criteria:
- `DiagramFamily::UseCase` renders baseline use-case diagrams.
- Relation semantics are deterministic and tested.

### Slice H: Object family bootstrap

Scope:
1. Implement object-family instance/link IR.
2. Add layout for object nodes and relationships.
3. Add deterministic render + error fixtures.

Done criteria:
- `DiagramFamily::Object` renders baseline object diagrams.
- Unsupported forms return stable diagnostics.

### Slice I: Unknown family policy and auto-detection UX

Scope:
1. Finalize unknown-family diagnostic policy.
2. Add CLI guidance for family selection/auto-detection.
3. Ensure docs clearly communicate supported family matrix.

Done criteria:
- Unknown or ambiguous family input yields actionable deterministic error text.
- CLI and docs align on family support status.


## Canonical Examples Corpus (`docs/examples/`)

This corpus is the top-layer parity artifact for user-visible behavior and release readiness.

Execution policy:
1. Create and maintain `docs/examples/<family>/` directories with canonical `.puml` + `.svg` pairs.
2. Require at least one new example pair for every new feature/primitive.
3. Tie parity matrix rows directly to canonical example IDs (`NNN_slug`).

Docs-as-tests policy:
1. Treat the corpus as snapshot-like test vectors in CI (render and compare SVG bytes).
2. Reject undocumented render drift unless intentionally updated with parity rationale.
3. Keep error semantics in `tests/fixtures/errors/**`; keep canonical corpus acceptance-oriented.

Slice integration:
- Slice A (sequence baseline): seed sequence taxonomy examples (`010`-`060` bands) and wire initial docs-as-tests harness.
- Slices B-H: each family bootstrap must add a minimum canonical starter set before family status can move to supported/partial in parity matrix.
- Slice I: unknown-family policy must include canonical negative guidance in docs (without acceptance SVG pairs).

## Benchmark Parity Tracking (No-Java Environment)

- Current baseline (available now): benchmark `puml` only for cold-start, parser, and render paths via `./scripts/bench.sh`.
- First executable parity harness baseline: `python3 scripts/parity_harness.py --output docs/benchmarks/parity_latest.json`.
- Docs/examples canonical layer is enforced by the harness:
  1. Add or update a markdown example in `docs/examples/*.md`.
  2. Ensure it links a `.puml` source and commit the matching `.svg` artifact (or add fenced `puml` and commit `<md-stem>_snippet_<n>.svg`).
  3. Run parity harness; report must show `doc_examples.summary.failed = 0`.
- Environment constraint: Java is not required for baseline runs in this repo.
- PlantUML comparison rows: remain `TODO` until Java + PlantUML jar are available in the benchmark environment.
- Comparison method (documented now, execute later):
  1. Run identical fixture corpus through `puml` and PlantUML.
  2. Capture parse success, render success, and elapsed time per fixture.
  3. Append comparison rows to benchmark markdown with clear tool labels and timestamp.

## Track Status

- New placeholder fixtures were not added in this pass to avoid speculative failures without paired implementation.
- TODO fixture names above are execution-ready and should be introduced at the start of each corresponding implementation slice.
