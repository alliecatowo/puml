# PlantUML Frontend Conformance Matrix

Issue lane: #130  
Last updated: 2026-05-16 (America/Los_Angeles)

This matrix tracks deterministic parser/frontend conformance for block boundaries,
directive handling edges, comments/quoted-text behavior, and multi-block extraction.

## Matrix

| Area | Scenario | Fixture | Expected Deterministic Outcome | Coverage Anchor |
|---|---|---|---|---|
| Block boundaries | `@startuml`/`@enduml` in uppercase | `tests/fixtures/structure/multi_three.puml` + inline uppercase case in tests | Multi split succeeds with stable block order | `tests/integration.rs::multi_mode_splits_uppercase_start_enduml_blocks` |
| Block boundaries | `@startuml` optional suffix (`@startuml name` / quoted) | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Marker recognized; block contents parsed | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Block boundaries | `@enduml` without open block (suffix tolerated) | `tests/fixtures/errors/invalid_unmatched_enduml_with_suffix.puml` | Error includes unmatched boundary + `without a preceding @startuml` | `tests/integration.rs::check_mode_reports_enduml_without_startuml_even_with_suffix_text` |
| Block boundaries | nested `@startuml` before close (suffix tolerated) | `tests/fixtures/errors/invalid_nested_startuml_with_suffix.puml` | Error includes unmatched boundary + nested open diagnostic | `tests/integration.rs::check_mode_reports_nested_startuml_even_with_suffix_text` |
| Directives | known compatibility directives (`!pragma`, `!theme`) are non-fatal | `tests/fixtures/basic/valid_pragma_directives.puml` | Parse/check succeeds; warnings deterministic where applicable | `tests/integration.rs::check_mode_emits_styling_warnings_but_succeeds` |
| Directives | unknown preprocessor directive fails deterministically | `tests/fixtures/errors/invalid_unknown_only.puml` | `E_PREPROC_UNSUPPORTED` deterministic diagnostic | `src/parser.rs::unknown_preprocessor_directive_errors_deterministically` |
| Comments + quoted text | apostrophe full-line comment ignored | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Comment line ignored and does not affect block extraction | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Comments + quoted text | apostrophe inside quoted label preserved | `tests/fixtures/conformance/valid_named_blocks_and_comments.puml` | Message label preserves `don't` inside quotes | `tests/integration.rs::multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers` |
| Multi-block extraction | trailing unterminated block in `--multi` | `tests/fixtures/errors/invalid_unterminated_second_block.puml` | Error includes unmatched boundary + missing closing marker | `tests/integration.rs::multi_mode_reports_unterminated_trailing_startuml_block` |
| Multi-block extraction | `@enduml` before `@startuml` in `--multi` | `tests/fixtures/errors/invalid_unmatched_enduml.puml` | Error includes unmatched boundary + missing open marker | `tests/integration.rs::multi_mode_reports_enduml_without_startuml` |

## Notes

- Matrix rows are intentionally fixture-first so each behavior is executable in CI.
- This matrix is not a broad PlantUML language parity claim; it is a deterministic runtime contract for the implemented frontend slice.
