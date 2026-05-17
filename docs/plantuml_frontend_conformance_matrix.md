# PlantUML Frontend Conformance Matrix

Issue lane: #130  
Last updated: 2026-05-16 (America/Los_Angeles)

This matrix tracks deterministic parser/frontend conformance for block boundaries,
directive handling edges, comments/quoted-text behavior, and multi-block extraction.
It is scoped to exercised frontend behaviors only. It does not assert full
PlantUML 1:1 parity; use
[`docs/audits/plantuml_parity_source_of_truth.md`](audits/plantuml_parity_source_of_truth.md)
as the canonical current implemented/partial/missing status. Rows here are
runtime contract checks for selected frontend behavior, not a broader language
support source of truth.

## Matrix

| Area | Scenario | Fixture | Expected Deterministic Outcome | Coverage Anchor |
|---|---|---|---|---|
| Block boundaries | `@startuml`/`@enduml` in uppercase | `tests/fixtures/structure/multi_three.puml` + inline uppercase case in tests | Multi split succeeds with stable block order | `tests/integration.rs::multi_mode_splits_uppercase_start_enduml_blocks` |
| Block boundaries | `@startuml` optional suffix (`@startuml name` / quoted) | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Marker recognized; block contents parsed | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Block boundaries | `@enduml` without open block (suffix tolerated) | `tests/fixtures/errors/invalid_unmatched_enduml_with_suffix.puml` | Error includes unmatched boundary + `without a preceding @startuml` | `tests/integration.rs::check_mode_reports_enduml_without_startuml_even_with_suffix_text` |
| Block boundaries | nested `@startuml` before close (suffix tolerated) | `tests/fixtures/errors/invalid_nested_startuml_with_suffix.puml` | Error includes unmatched boundary + nested open diagnostic | `tests/integration.rs::check_mode_reports_nested_startuml_even_with_suffix_text` |
| Directives | known compatibility directives (`!pragma`, `!theme`) are non-fatal | `tests/fixtures/basic/valid_pragma_directives.puml` | Parse/check succeeds; warnings deterministic where applicable | `tests/integration.rs::check_mode_emits_styling_warnings_but_succeeds` |
| Directives | sequence `!pragma teoz true` compatibility boundary | inline render case | Render output stays deterministic and equal to standard sequence layout | `tests/render_e2e.rs::render_svg_pragma_teoz_boundary_keeps_sequence_render_output_stable` |
| Sequence arrows/style | bracketed PlantUML arrow color/style payloads | inline parser/render cases | Parser accepts `-[#red,dashed]>` / `-[hidden]->` and normalizes to the portable arrow core for deterministic rendering | `src/parser.rs::parses_sequence_decorated_arrow_styles_as_portable_arrow_core`, `tests/render_e2e.rs::render_sequence_decorated_arrows_and_teoz_boundary_stay_deterministic` |
| Directives | unknown preprocessor directive fails deterministically | `tests/fixtures/errors/invalid_unknown_only.puml` | `E_PREPROC_UNSUPPORTED` deterministic diagnostic | `src/parser.rs::unknown_preprocessor_directive_errors_deterministically` |
| Preprocessor breadth | list/map/stringification builtins inside `!assert` and `!log` | `tests/fixtures/preprocessor/valid_builtin_list_map_stringification_assert_log.puml` | `%size` counts collections, map aliases resolve, assert/log payloads expand without rendering noise | `tests/integration.rs::preprocessor_builtin_list_map_stringification_assert_and_log_surface_passes` |
| Core UML broad partials | class-like/object-map/usecase-actor declarations, `usecase (Name) as Alias`, include/extend dependencies, activity backward/goto/split, and state broad forms | inline parser/render cases | Parser preserves broad forms; render output contains expected labels/markers | `src/parser.rs::parses_core_uml_broad_partial_declaration_forms`, `src/parser.rs::parses_activity_switch_split_goto_and_terminal_controls`, `tests/render_e2e.rs::render_core_uml_broad_partials_surface_expected_labels` |
| Data projection partial rows | JSON/YAML object-like projections tolerate partial key rows and quoted braces | inline projection cases | Projection rows render as deterministic key/value text without leaking raw body lines | `tests/integration.rs::json_projection_accepts_partial_rows_and_quoted_braces`, `tests/integration.rs::yaml_projection_accepts_partial_rows_and_quoted_braces` |
| Comments + quoted text | apostrophe full-line comment ignored | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Comment line ignored and does not affect block extraction | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Comments + quoted text | apostrophe inside quoted label preserved | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Message label preserves `don't` inside quotes | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Multi-block extraction | trailing unterminated block in `--multi` | `tests/fixtures/errors/invalid_unterminated_second_block.puml` | Error includes unmatched boundary + missing closing marker | `tests/integration.rs::multi_mode_reports_unterminated_trailing_startuml_block` |
| Multi-block extraction | `@enduml` before `@startuml` in `--multi` | `tests/fixtures/errors/invalid_unmatched_enduml.puml` | Error includes unmatched boundary + missing open marker | `tests/integration.rs::multi_mode_reports_enduml_without_startuml` |

## Notes

- Matrix rows are intentionally fixture-first so each behavior is executable in CI.
- This matrix is not a broad PlantUML language parity claim; it is a deterministic runtime contract for the implemented frontend slice.
- Fixture and example coverage here should be treated as coverage seeds, not exhaustive parity proof.
