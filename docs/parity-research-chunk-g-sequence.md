# Parity Research Chunk G: Sequence Diagrams

Date: 2026-05-15
Scope: Sequence syntax/behavior parity against official PlantUML sequence docs, compared to current `puml` behavior.
Ownership: docs-only (no `src/` or `tests/` edits).

## Baseline

Primary external baseline:
- Official PlantUML sequence documentation: https://plantuml.com/sequence-diagram

Implementation evidence used:
- Parser: `src/parser.rs`
- Normalization/lifecycle semantics: `src/normalize.rs`
- Layout and rendering behavior: `src/layout.rs`, `src/render.rs`
- Current contract docs: `README.md`, `docs/decision-log.md`
- Behavioral tests/fixtures: `tests/integration.rs`, `tests/render_e2e.rs`, `tests/fixtures/**`

## Parity Matrix

Legend:
- `Supported`: works with behavior close to PlantUML baseline for common usage.
- `Partial`: accepted, but semantics/rendering/optionality differ materially.
- `Unsupported`: common PlantUML sequence capability not available.
- `Differentiator`: intentional behavior difference (usually stricter or scoped).

| Area | PlantUML Baseline | `puml` Status | Evidence | Notes |
|---|---|---|---|---|
| Sequence scope | Sequence diagrams plus many other UML families | `Differentiator` | `src/normalize.rs`, `docs/decision-log.md` D-001 | `puml` is intentionally sequence-only; non-sequence input rejected. |
| `@startuml`/`@enduml` blocks | Standard source delimiters | `Supported` | `src/parser.rs`, `tests/fixtures/basic/valid_start_end.puml` | Also tolerates plain single-diagram text input. |
| Basic messages (`->`, `-->`, `<-`, etc.) | Rich arrow forms | `Partial` | `src/parser.rs` `VALID_ARROWS` list | Core arrows supported, but many PlantUML variants are missing (e.g., slanted/top-half syntaxes shown in docs). |
| Bidirectional arrows (`<->`) | Supported | `Partial` | `src/normalize.rs` (`bidirectional` split) | Expanded into two one-way events; rendering semantics differ from native PlantUML style nuances. |
| Participant auto-creation from messages | Supported | `Supported` | `src/normalize.rs` `ensure_implicit` | Participants inferred if not declared. |
| Participant declarations (`participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`) | Supported (+ `queue`) | `Partial` | `src/parser.rs` role list | Missing `queue` role explicitly documented by PlantUML. |
| Aliases / quoted participant names | Supported | `Supported` | `src/parser.rs`, `tests/fixtures/participants/valid_aliases.puml` | Includes quoted display and `as` alias handling. |
| Message to self | Supported | `Supported` | `tests/fixtures/arrows/self.puml`, `src/layout.rs` self-loop handling | Rendered as short rightward loop line. |
| Notes (`left/right/over/across`) | Supported | `Supported` | `src/parser.rs`, `tests/fixtures/notes/*` | Supports single-line and multiline `note ... end note`. |
| `ref over` blocks | Supported | `Partial` | `src/parser.rs`, `src/render.rs`, `tests/fixtures/groups/valid_ref_and_else_rendering.puml` | Supported as group box rendering; not full PlantUML reference semantics/options. |
| Group fragments (`alt/else/opt/loop/par/critical/break/group/end`) | Supported | `Partial` | `src/parser.rs`, `src/layout.rs`, fixtures under `tests/fixtures/groups/` | Parsed/rendered as generic boxes + separators; semantic validation (nesting/type matching) is looser than PlantUML. |
| Divider/separator (`== ... ==`) | Supported | `Unsupported` | PlantUML docs vs `src/parser.rs` (`...` only) | `puml` supports `...` divider/spacer style, not `==` separator syntax. |
| Delay (`||`) | Supported | `Partial` | `src/parser.rs` | Parsed as delay events but currently no visible render treatment in SVG path. |
| `newpage` | Supported | `Partial` | `src/normalize.rs` paginate, `tests/integration.rs` newpage cases | Works with page splitting; CLI behavior differs by `--multi` mode/output contract. |
| `autonumber` | Supported (richer options) | `Partial` | `src/layout.rs` `AutonumberState`, fixtures in `tests/fixtures/autonumber/` | Supports start/stop/restart basics; advanced PlantUML formatting is narrower. |
| Lifecycle commands (`activate`, `deactivate`, `create`, `destroy`, `return`) | Supported | `Partial` | `src/normalize.rs`, lifecycle fixtures/tests | Supported with strict stack/liveness constraints and return inference rules that are intentionally fail-fast. |
| Lifecycle arrow shortcuts (`++`, `--`, `**`, `!!`) | Supported | `Supported` | `src/parser.rs`, `src/normalize.rs`, fixtures `arrows/modifiers_basic.puml` | Implemented on arrow endpoints and normalized to lifecycle events. |
| Incoming/outgoing messages via bracket endpoints (`[`, `]`, `[*]`) | Supported | `Partial` | `src/parser.rs` `normalize_virtual_endpoint`, `src/layout.rs` virtual endpoint bounds | Virtual endpoints collapsed to `[*]`; nuanced side-specific PlantUML shapes are simplified. |
| Footbox controls (`hide/show footbox`) | Supported | `Partial` | `src/parser.rs`, `src/normalize.rs` | Parsed and modeled, but rendering currently does not visibly vary by footbox flag. |
| `skinparam` breadth | Extensive | `Differentiator` | `docs/decision-log.md` D-004, `src/normalize.rs` | Only `skinparam maxmessagesize` allowed; others raise warning->error behavior. |
| `!theme` | Supported in PlantUML | `Unsupported` (parse-only warning) | `src/parser.rs`, `src/normalize.rs` | Captured as warning then fails normalization contract. |
| Preprocessor (`!include`, `!define`, `!undef`) | Supported | `Partial` + `Differentiator` | `src/parser.rs`, `docs/decision-log.md` D-003/D-005 | Preprocess expansion exists with guards; contract intentionally fail-fast for directives at normalization boundary. |
| Multi-diagram sources | Supported | `Differentiator` | `docs/decision-log.md` D-002, `tests/integration.rs` | Requires explicit `--multi` opt-in in CLI contract. |

## Key Differentiators (Current Strengths)

- Deterministic, strict validation surface suitable for CI snapshots and tooling.
- Explicit fail-fast contract for unsupported styling/features avoids ambiguous partial render output.
- Strong lifecycle safety checks (`destroy`/activation/return inference) provide clearer diagnostics than permissive rendering.

## Prioritized Gap List

Scoring:
- Implementation complexity: `Low` / `Medium` / `High`.
- User impact: `Low` / `Medium` / `High`.
- Priority: P1 highest.

1. P1: Expand arrow syntax compatibility to cover common PlantUML forms beyond current whitelist.
- Complexity: `Medium`
- User impact: `High`
- Why: Arrow grammar is a top portability blocker when importing existing sequence sources.

2. P1: Add `queue` participant role parity.
- Complexity: `Low`
- User impact: `Medium`
- Why: Low-effort compatibility win for documented participant declarations.

3. P1: Implement `== separator ==` syntax.
- Complexity: `Low` to `Medium`
- User impact: `Medium`
- Why: Common readability pattern in PlantUML docs currently rejected.

4. P1: Make `footbox` materially affect SVG output.
- Complexity: `Medium`
- User impact: `Medium`
- Why: Users expect visible effect from accepted syntax.

5. P2: Deepen group semantics (nest/type validation + richer rendering semantics for `alt/opt/par/...`).
- Complexity: `High`
- User impact: `High`
- Why: Current generic-box rendering loses behavioral nuance for complex diagrams.

6. P2: Improve `autonumber` format parity (format controls, richer restart options).
- Complexity: `Medium`
- User impact: `Medium`
- Why: Frequently used in documentation-quality diagrams.

7. P2: Improve virtual endpoint fidelity for incoming/outgoing arrows (`[`, `]`, found/lost style specifics).
- Complexity: `Medium`
- User impact: `Medium`
- Why: Partial support exists but side-specific semantics are simplified.

8. P3: Broaden `skinparam` support incrementally (sequence-relevant keys first).
- Complexity: `High` (if broad)
- User impact: `Medium` to `High`
- Why: Styling parity matters for adoption, but can destabilize deterministic rendering unless carefully scoped.

9. P3: Reconcile preprocessor contract with implementation (either fully support or remove contradiction).
- Complexity: `High`
- User impact: `Medium`
- Why: Current docs/code history indicate boundary ambiguity for directives.

## Benchmark Recommendations (No-Java Environment)

Current constraint: no Java baseline available in this environment.

### Recommended now (ours-only)

1. Standardize workload corpus.
- Use existing fixtures as seed (`tests/fixtures/basic`, `groups`, `notes`, `lifecycle`, `autonumber`, `structure`), then add a generated scale set (10/100/1k messages, varying participants and nested groups).

2. Use `hyperfine` for stable CLI timing.
- Benchmark `cargo run --release -- ...` or `target/release/puml ...` for:
  - single render SVG
  - `--check`
  - `--dump model`
  - `--multi` with `newpage` and multi-block files
- Capture mean, p95, stddev, and command warmup behavior.

3. Track memory and binary metrics.
- Memory: `/usr/bin/time -v` (max RSS).
- Binary size: `size target/release/puml` and artifact bytes.

4. Pin deterministic benchmark conditions.
- Fixed CPU governor where possible, isolated runner, repeated runs, no debug builds.
- Record hardware + OS + Rust toolchain metadata in benchmark report.

5. Automate in CI-friendly script.
- Evolve `scripts/bench.sh` from placeholder into repeatable benchmark harness with machine-readable output (JSON/CSV).

### Optional future (with PlantUML baseline available)

1. Corpus parity benchmark.
- Run identical corpus through `puml` and PlantUML.
- Track parse success rate, render success rate, and elapsed time.

2. Compatibility-first dashboard.
- For each fixture: classify `equivalent render`, `accepted but visually divergent`, `rejected`.
- Pair with parity matrix tags to prioritize high-impact compatibility work.

3. Cost-per-diagram comparison.
- Compare throughput and memory across diagram sizes, not only averages.

## Source Links

External:
- PlantUML Sequence Diagram docs: https://plantuml.com/sequence-diagram

Repository (local evidence):
- [README.md](/home/Allie/develop/puml/README.md)
- [decision-log.md](/home/Allie/develop/puml/docs/decision-log.md)
- [parser.rs](/home/Allie/develop/puml/src/parser.rs)
- [normalize.rs](/home/Allie/develop/puml/src/normalize.rs)
- [layout.rs](/home/Allie/develop/puml/src/layout.rs)
- [render.rs](/home/Allie/develop/puml/src/render.rs)
- [integration.rs](/home/Allie/develop/puml/tests/integration.rs)
- [render_e2e.rs](/home/Allie/develop/puml/tests/render_e2e.rs)
- [bench.sh](/home/Allie/develop/puml/scripts/bench.sh)
