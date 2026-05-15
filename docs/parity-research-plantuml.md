# PlantUML Sequence Parity Audit (`puml`)

Date: 2026-05-15  
Scope owner: `docs/parity-research-plantuml.md` only

## Sources
- PlantUML sequence reference: https://plantuml.com/sequence-diagram
- PlantUML command line reference: https://plantuml.com/command-line
- PlantUML sources/file naming/includes reference: https://plantuml.com/sources
- PlantUML preprocessing reference: https://plantuml.com/preprocessing (mirrored content also visible on localized pages such as https://plantuml.com/ko/preprocessing)
- `puml` implementation + tests:
  - `src/parser.rs`, `src/normalize.rs`, `src/main.rs`
  - `tests/integration.rs`, `tests/render_e2e.rs`
  - `README.md`, `docs/decision-log.md`

## Status Legend
- `Supported`: works and broadly matches PlantUML expectations for common cases.
- `Partial`: implemented, but narrower semantics/syntax than PlantUML.
- `Missing`: expected PlantUML behavior not implemented.
- `Intentional OOS`: intentionally out-of-scope for `puml` contract.

## Sequence Syntax Parity Matrix
| Area | PlantUML baseline | `puml` status | Evidence | Notes |
|---|---|---|---|---|
| Diagram family scope | PlantUML supports many UML diagram families | Intentional OOS | `docs/decision-log.md` D-001; `normalize` rejects non-sequence | Sequence-only is intentional product scope, not a bug. |
| `@startuml` / `@enduml` | Standard delimiters | Supported | `src/main.rs` `split_diagrams`; integration boundary tests | Also accepts plain text single diagram input. |
| Participant auto-creation from message usage | Supported | Supported | `normalize.rs` `ensure_implicit` flow | Matches common PlantUML sequence behavior. |
| Participant kinds | Includes `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`, `queue` | Supported | `parser.rs` `parse_participant` roles | `queue` is included. |
| Aliases/quoted names | Supported | Supported | `parse_participant`; fixtures `participants/valid_aliases.puml` | Works with `as` and quoted display names. |
| Arrow syntax breadth | Rich set including style variants/slanted forms and more | Partial | `parser.rs` `VALID_ARROWS`; canonical slash removal only | Core arrows supported; full PlantUML arrow vocabulary is broader. |
| Bidirectional messages | Supported | Partial | `normalize.rs` `bidirectional` split into two messages | Behavior is normalized to two one-way events, not native styling semantics. |
| Lifecycle shortcuts (`++`, `--`, `**`, `!!`) | Supported in sequence docs | Supported | parser encode/decode + lifecycle tests | Implemented and validated with diagnostics. |
| Incoming/outgoing short endpoints (`[`, `]`, `[o`, `o]`, `[x`, `x]`, etc.) | Supported | Partial | `normalize_virtual_endpoint` collapses to `[*]` | Endpoint variants lose side/shape-specific semantics. |
| Notes (`left/right/over/across`) | Supported | Partial | `parse_keyword` note parsing; render tests | Core forms work; advanced note shape features (`hnote`, `rnote`) not present. |
| Group fragments (`alt/else/opt/loop/par/critical/break/group/end`) | Supported | Partial | group parsing + `normalize` stack checks | Rendering is pragmatic; not full PlantUML semantic richness. |
| `ref over` (single and multiline block) | Supported | Partial | parser multiline ref handling + ref fixtures | Supported as grouped block; options are narrower than PlantUML reference features. |
| Divider/separator/delay | `== ... ==`, `...`, `||` supported | Supported | `parse_keyword` handles all three | No parity blocker at syntax recognition level. |
| `newpage` | Split into several images/pages | Partial | `parse_keyword` + paginate/layout + CLI behaviors | Core split works; UX differs significantly in stdin/multi contract (see runtime section). |
| `ignore newpage` | PlantUML supports ignoring page splits | Missing | Not parsed in `parser.rs` | No equivalent today. |
| `autonumber` | Rich formatting/reset/increment syntax | Partial | stored as raw command; layout applies subset | Basic/common usage works; full format parity not guaranteed. |
| `title/header/footer/caption/legend` | Supported | Supported | parsed + carried in model/page | Includes page header/footer/title text handling. |
| `hide footbox` / `show footbox` | Supported | Supported | `StatementKind::Footbox`; render/layout tests | Behavior is visually represented in output snapshots. |
| `skinparam` | Large config surface | Partial | only `maxmessagesize` semantically supported; warnings for others | Deterministic warning behavior is deliberate contract. |
| `!theme` | Supported | Missing (warning-only acceptance) | `normalize` emits `W_THEME_UNSUPPORTED` | Non-fatal diagnostic, no theme semantics. |
| Preprocessor `!include` | Supported (file/url/include-id variants, etc.) | Partial | parser preprocess include with root/cycle/escape guards | Local relative include is implemented; URL/id variants are missing. |
| Preprocessor `!define` / `!undef` | Legacy but available in PlantUML preprocessing | Partial | parser preprocess substitution map | Simple token substitution only; not full preprocessor model. |
| Preprocessor conditionals (`!if`/`!elseif`/`!else`/`!endif`, `!ifdef`, `!ifndef`) | Supported | Partial | parser preprocess conditional execution | Deterministic subset only (simple expressions + explicit balance/order diagnostics). |
| Preprocessor loops (`!while`/`!endwhile`) | Supported | Partial | parser preprocess bounded loop execution | Deterministic subset only; bounded iteration guard and no advanced loop semantics. |

## CLI/Runtime UX Parity Matrix
| Area | PlantUML baseline | `puml` status | Evidence | Notes |
|---|---|---|---|---|
| Multi-diagram from one file | PlantUML auto-generates numbered outputs from multi `@start...` blocks | Partial | `src/main.rs` + `tests/integration.rs` | File input can emit numbered files, but stdin requires explicit `--multi`. |
| Multi-diagram stdin behavior | PlantUML `-pipe` reads stdin/writes image bytes to stdout | Partial | PlantUML command-line docs; `stdin` tests | `puml` uses SVG for single stdin diagram, JSON array for multi-page/multi-diagram stdin with `--multi`. |
| Multi-page (`newpage`) output contract | PlantUML splits into multiple images/pages | Partial | sequence docs + `stdin_newpage_*` tests | `puml` file mode writes `-1`, `-2` files; stdin mode errors without `--multi`, then emits JSON array. |
| Output naming determinism | PlantUML: default source-based name + numbering | Supported | PlantUML sources docs + `write_output_files` tests | `puml` numbering order is stable and snapshot-tested. |
| Include path policy | PlantUML supports broader include mechanisms (`!include`, URL, include path settings) | Partial | preprocessing/sources docs; parser include guards | `puml` intentionally constrains includes to canonical root and blocks escapes. |
| Preprocessor breadth | PlantUML preprocessor has variables, conditionals, functions, stdlib constructs | Partial (bounded) | preprocessing docs vs parser behavior | `puml` supports include + define/undef + conditional/while subset; functions/procedures/advanced expression surface remains unsupported. |
| Diagnostic formatting options | PlantUML has `-stdrpt` variants | Missing | command-line docs vs `puml` CLI | `puml` has one deterministic diagnostic format (line/column/caret). |
| Diagnostics quality (source mapping) | PlantUML reports parse errors; format varies by option | Supported (strong local contract) | integration tests for line/column/caret in check/dump/render | High quality and deterministic within current contract. |
| Deterministic output expectations | PlantUML does not market strict deterministic JSON/scene contracts | Supported (differentiator) | determinism snapshots (`dump scene`, `render svg`) | `puml` deterministic behavior is a product strength, not a parity gap. |

## Prioritized Gap List (Actionable)

### P1
1. Arrow syntax expansion
- Gap: Missing common PlantUML arrow variants beyond current whitelist.
- Severity: High
- Difficulty: Medium
- Suggested fixtures:
  - `tests/fixtures/arrows/plantuml_arrow_variants_portability.puml`
  - `tests/fixtures/arrows/plantuml_arrow_variants_invalid_mixed.puml`

2. `ignore newpage` support
- Gap: PlantUML supports `ignore newpage`; `puml` does not parse it.
- Severity: High
- Difficulty: Medium
- Suggested fixtures:
  - `tests/fixtures/structure/valid_ignore_newpage_single_scene.puml`
  - `tests/integration_ignore_newpage_stdin_behavior.rs` (or integration test name `ignore_newpage_keeps_single_svg_output`)

3. Stdin multi-page UX parity
- Gap: `newpage` on stdin fails without `--multi`, and with `--multi` emits JSON array wrapper rather than stream/image behavior.
- Severity: High
- Difficulty: Medium
- Suggested fixtures:
  - `tests/fixtures/structure/newpage_stdin_contract.puml`
  - `tests/integration__stdin_newpage_cli_contract_modes.snap`

### P2
4. Preprocessor breadth (`!include` variants, richer macro semantics)
- Gap: No URL includes, include-by-id (`file!TAG`) parity, and very limited macro semantics.
- Severity: Medium
- Difficulty: High
- Suggested fixtures:
  - `tests/fixtures/include/include_with_id_block.puml`
  - `tests/fixtures/include/include_url_rejected_or_supported.puml`
  - `tests/fixtures/preprocessor/define_function_like_macro_portability.puml`

5. Endpoint fidelity for incoming/outgoing arrows
- Gap: Distinct endpoint tokens collapse to `[*]`, losing side/shape semantics.
- Severity: Medium
- Difficulty: Medium
- Suggested fixtures:
  - `tests/fixtures/arrows/virtual_endpoints_side_specific.puml`
  - `tests/snapshots/render_e2e__virtual_endpoints_shape_fidelity.snap`

6. `ref over` feature depth
- Gap: Baseline ref blocks work, but advanced reference options are narrow.
- Severity: Medium
- Difficulty: Medium
- Suggested fixtures:
  - `tests/fixtures/groups/ref_over_extended_forms.puml`
  - `tests/fixtures/groups/ref_over_multiblock_labels.puml`

### P3
7. `skinparam` incremental parity beyond `maxmessagesize`
- Gap: Most keys warn and are ignored.
- Severity: Medium
- Difficulty: High
- Suggested fixtures:
  - `tests/fixtures/styling/skinparam_sequence_core_keys.puml`
  - `tests/integration__skinparam_supported_keys_no_warning.snap`

8. Optional diagnostic mode parity (`-stdrpt`-like output styles)
- Gap: No selectable machine-oriented error format styles.
- Severity: Low
- Difficulty: Medium
- Suggested fixtures:
  - `tests/integration__diagnostics_stdrpt_single_line_mode.snap`
  - `tests/integration__diagnostics_default_mode_unchanged.snap`

## Intentional Non-Gaps (Do Not Treat As Defects)
- Sequence-only scope vs full PlantUML family support.
- Deterministic scene/render JSON and strict validation behavior (this is a `puml` differentiator).
- Include confinement model (`--include-root`, canonical path escape prevention) where security and reproducibility are prioritized over full PlantUML permissiveness.

## Recommended Implementation Order
1. P1 arrow syntax expansion.
2. P1 `ignore newpage` + stdin multi-page contract alignment.
3. P2 preprocessor parity increments (start with include-id syntax and explicit diagnostics for unsupported include URL).
4. P2 endpoint fidelity and ref-depth improvements.
5. P3 styling breadth and optional diagnostics format modes.
