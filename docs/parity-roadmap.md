# Parity Roadmap

Date: 2026-05-17

## Reading This Roadmap

This file records the parity mission, historical closure notes, and future
execution slices. It is not the measured parity scoreboard. Current support
status lives in `docs/internal/parity/plantuml_parity_source_of_truth.md`, and measured
oracle evidence comes from JAR-backed `oracle-report-<run>` CI artifacts or a
fresh local `PUML_ORACLE_JAR` run.

The committed `docs/benchmarks/oracle_report.json` may be a skip sentinel from
a Java-free local run. A skip sentinel means "comparison not run"; it is useful
for deterministic local workflows but is not measured parity evidence.

Closure update (Epic #30, 2026-05-15):
- Final closure pass for the staged sequence-parity epic was green across `./scripts/check-all.sh`, `./scripts/check-all.sh --quick`, and `./scripts/harness-check.sh` at the time of that audit.
- Docs-example drift checks are enforced in `tests/svg_bounds_audit.rs` (`doc_examples.summary.failed == 0`).
- `scripts/parity_harness.py` canonicalizes trailing SVG newlines to prevent false doc parity failures between stdin-rendered SVG and checked-in artifacts.

This roadmap tracks high-impact parity work relative to PlantUML behavior and defines execution order, measurable done criteria, and fixture-first delivery.

Product language policy baseline:
- PicoUML is the first-class canonical language for the engine.
- PlantUML is a first-class compatibility target and long-term mission, not a blanket claim that every official PlantUML construct currently matches.
- Mermaid is a first-class supported frontend for scoped sequence coverage.

## Source Inputs

- Current parity research: `docs/internal/research/parity-research-chunk-g-sequence.md`
- Frontend conformance contract matrix: `docs/internal/parity/plantuml_frontend_conformance_matrix.md`
- Contract decisions: `docs/internal/architecture-decisions.md`
- Coverage and test signals: `docs/internal/coverage-status.md`, `tests/**`
- Existing fixture constraints for this track: `tests/fixtures/errors/` and `tests/fixtures/basic/` (only when needed)

## Delivery Principles

- Test-first: each backlog item starts with fixtures/tests that fail for the intended parity gap.
- Small blast radius: parser/normalizer/render changes ship in narrow slices with fixture proof.
- Contract clarity: if behavior is intentionally different, document it immediately in decision log + user docs.
- No silent UX drift: CLI flags, error text shape, and output artifact behavior are treated as parity surface.

## Staged Backlog (Execution Order)

### Stage 0: Baseline and Harness Lock (Gate Before Feature Work)

Items:
1. Freeze parity target list from `docs/internal/research/parity-research-chunk-g-sequence.md` into issue/checklist form.
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
- Implemented fixture coverage:
  - `tests/fixtures/basic/valid_separator_equals.puml`
  - `tests/fixtures/errors/invalid_separator_unbalanced_equals.puml`
  - `tests/fixtures/basic/valid_participant_queue.puml`
  - `tests/fixtures/errors/invalid_participant_queue_alias_collision.puml`
  - `tests/fixtures/basic/valid_arrows_extended_set.puml`
  - `tests/fixtures/arrows/valid_arrow_variant_tokenization.puml`
  - `tests/fixtures/errors/invalid_arrow_variant_tokenization.puml`
- Render parity coverage for footbox and syntax interactions is enforced via deterministic snapshots in `tests/render_e2e.rs`.

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
- Implemented fixture coverage:
  - `tests/fixtures/errors/invalid_group_mismatched_end_keyword.puml`
  - `tests/fixtures/errors/invalid_group_else_without_alt.puml`
  - `tests/fixtures/groups/valid_group_nested_mixed_fragments.puml`
  - `tests/fixtures/structure/valid_autonumber_restart_step_format.puml`
  - `tests/fixtures/structure/valid_autonumber_off_resume_edges.puml`
  - `tests/fixtures/errors/invalid_autonumber_bad_format_token.puml`
  - `tests/fixtures/arrows/valid_endpoint_variants.puml`

### Stage 3: Policy and Styling Boundary Closure (P3)

Items:
1. Expand `skinparam` in deterministic sequence-focused increments.
2. Resolve preprocessor boundary ambiguity (`!include`, `!define`, `!undef`) with explicit contract.

Acceptance criteria:
- Supported `skinparam` keys are explicitly listed; unsupported keys fail predictably with stable diagnostics.
- Preprocessor behavior is either fully supported for declared subset or rejected consistently at documented boundary.
- Decision log entries capture rationale for every intentional non-parity boundary.

Fixture-first plan:
- Implemented fixture coverage:
  - `tests/fixtures/styling/valid_skinparam_sequence_footbox_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_arrow_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_lifeline_border_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_participant_background_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_participant_border_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_note_background_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_note_border_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_group_background_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_group_border_color_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_sequence_alias_colors_supported.puml`
  - `tests/fixtures/styling/valid_skinparam_unsupported_mixed_deterministic.puml`
  - `tests/fixtures/include/include_ok_child.puml`
  - `tests/fixtures/include/include_with_tag_ok.puml`
  - `tests/fixtures/errors/invalid_include_only.puml`
  - `tests/fixtures/errors/invalid_define_only.puml`
  - `tests/fixtures/errors/invalid_undef_only.puml`
  - `tests/fixtures/errors/invalid_include_tag_missing.puml`
  - `tests/fixtures/errors/invalid_include_url.puml`
  - `tests/fixtures/preprocessor/valid_if_elseif_else.puml`
  - `tests/fixtures/preprocessor/valid_ifdef_ifndef.puml`
  - `tests/fixtures/preprocessor/valid_while_define_counter.puml`
  - `tests/fixtures/errors/invalid_preproc_conditional_order.puml`
  - `tests/fixtures/errors/invalid_preproc_unclosed_if.puml`
  - `tests/fixtures/errors/invalid_preproc_procedure_unsupported.puml`
  - `tests/fixtures/errors/invalid_preproc_endwhile_without_while.puml`

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
  - README remains product-oriented and links to the audit table instead of carrying a large parity wall.
  - Detailed docs expose supported/partial/unsupported status through the parity source-of-truth audit and aligned CSV exports.
  - At least one troubleshooting entry per major failure class introduced in stages 1-3.

Fixture-first plan:
- No new fixtures required by default; this stage validates behavior over prior stage fixtures.

## Diagram Families Parity Program (Execution Slices)

This section defines family-by-family execution slices for the shared IR + layout-engine program.

### Slice A: Sequence hardening baseline

Scope:
1. Preserve sequence as the deepest compatibility lane while other families continue breadth/depth hardening.
2. Keep sequence on the shared family routing boundary without behavior drift.
3. Add deterministic tests for family-aware routing.

Done criteria:
- Existing sequence fixtures remain green.
- New family-routing API is additive and stable.
- Unsupported constructs in any family remain explicit deterministic rejections.
- Mermaid frontend baseline for `sequenceDiagram` subset routes through the same first-class shared parse pipeline entrypoint.
- Unsupported Mermaid families/constructs fail with deterministic compatibility diagnostics.

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
- PlantUML comparison rows in the generic benchmark trend remain `TODO` for
  no-Java runs. Differential oracle evidence is tracked separately through
  `docs/benchmarks/oracle_smoke_latest.json` and optional `PUML_ORACLE_JAR`
  reports.
- Comparison method (documented now, execute later):
  1. Run identical fixture corpus through `puml` and PlantUML.
  2. Capture parse success, render success, and elapsed time per fixture.
  3. Append comparison rows to benchmark markdown with clear tool labels and timestamp.

## Track Status

### Parity Blitz Completion (2026-05-15)

Baseline render/parser lanes for all tracked diagram families and broad preprocessor
surface areas landed during the blitz. That milestone closed bring-up work; it did
not mean full PlantUML semantic or pixel parity for every advanced row. The
current measured/planning status remains the audit table.

Historical blitz summary:

#### Diagram Families — Done (2026-05-15)

| Family | Status | Closed Issue |
|---|---|---|
| Sequence | Supported — deepest lane; advanced breadth still audited conservatively | #111 |
| Class / Object / UseCase | Supported — real renderers | #146 |
| Component / Deployment | Supported — real renderers | #109 |
| State / Activity / Timing | Supported — real renderers | #110 |
| Salt (wireframe) | Supported — real tree render | #98, #99 |
| MindMap / WBS | Supported — hierarchical layout | #96, #97 |
| Gantt / Chronology | Supported — timeline render with project date axes and Gantt closed-weekday calendar notes | #94, #95 |
| JSON / YAML | Supported | #102 |
| nwdiag | Supported | #101 |
| Archimate | Supported | #100 |
| Regex / EBNF | Supported (railroad SVG) | #104 |
| Math / SDL / Ditaa | Supported (deterministic stub) | #105 |
| Chart | Supported (bar/line/pie) | #106 |

#### Preprocessor — Done (2026-05-15)

| Area | Status | Closed Issue |
|---|---|---|
| `!function`/`!procedure`/`!return` + all builtins | Supported | #147 |
| `!include_many`/`!include_once`/`!includesub` | Supported | #115 |
| `!import` / stdlib resolution | Supported | #116 |
| `!theme` local catalog | Supported | #117 |
| JSON variable assignment | Supported | #135 |
| URL include policy | Native CLI supports URL includes behind `--allow-url-includes`; default CLI/LSP/WASM no-surprise surfaces disable or reject remote fetches | #255 / #288 |

#### Sequence Parity — Done (2026-05-15)

- All sequence partial/missing parity rows from `parity_gap_core.csv` closed: #111.
- Mermaid sequence frontend parity: all supported constructs route through shared pipeline.
- PicoUML canonical baseline routing: #128.
- PlantUML frontend conformance matrix: #130.

#### CLI / Runtime — Baseline Done (2026-05-15)

Landed (2026-05-15): `--check`, `--dump`, `--multi`, `--from-markdown`, `--diagnostics`, `--include-root`, `--output`, `--overwrite`, `--fail-on-warn`, `--charset`, `--duration`, `--quiet`, `--verbose`, `--format`, `--dialect`, `--compat`, `--determinism`, `--lint-input`, `--lint-glob`, `--lint-report`.

#87 is closed. Follow-up runtime gaps are tracked on their owning parity rows
rather than by reopening the blitz issue:
- `hide unlinked` filters unreferenced explicit sequence participants; broader non-sequence `hide @unlinked` parity remains tracked separately.
- `--format png`, `--format jpg`, and `--format webp` are supported through deterministic SVG rasterization with `--dpi`; advanced PlantUML raster flags beyond DPI remain out of scope. `--format html` writes a self-contained HTML document around the rendered SVG.

#### Differential Oracle — Smoke Workflow Landed / Full Parity Deferred

The deterministic Java-free oracle smoke report is available at
`docs/benchmarks/oracle_smoke_latest.json`, and CI has a differential oracle
smoke workflow with optional live PlantUML execution. `scripts/oracle.sh` can
produce `docs/benchmarks/oracle_report.json` when `PUML_ORACLE_JAR` is set.
This is comparison evidence only. The checked-in `oracle_report.json` may be a
skip sentinel when generated without `PUML_ORACLE_JAR`; the latest CI artifact
or a fresh JAR-backed local report is the measured evidence to use. The generic
benchmark trend can still show no-Java TODO placeholders, and full semantic/pixel
parity remains deferred.

#### JSON Projection Adapters — Follow-Up Breadth

JSON/YAML projection work has landed and tracking issue #103 is closed/Done.
Broader cross-diagram projection breadth remains audited as partial in the source
of truth until additional fixture and oracle evidence supports promotion.

---

- Sequence parity epic closure status (2026-05-15):
  - Sequence parity fixtures/tests in active contract suites are green:
    - `cargo test --test integration --test render_e2e --test virtual_endpoint_fidelity`
    - `cargo test --test svg_bounds_audit`
  - Parity harness report is current and docs example parity is green:
    - `python3 scripts/parity_harness.py --output docs/benchmarks/parity_latest.json`
    - Expected state: `doc_examples.summary.failed = 0`
  - No known unexpected failing fixtures/tests remain in sequence parity areas.
