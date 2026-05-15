use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use predicates::prelude::*;
use puml::render_source_to_svg;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn single_file_defaults_to_svg_file_output() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let output = tmp.path().join("single_valid.svg");
    assert!(output.exists());

    let expected = fs::read_to_string(fixture("single_valid.svg")).unwrap();
    let actual = fs::read_to_string(output).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn default_frontend_matches_explicit_plantuml() {
    let default = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let plantuml = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "plantuml",
            "--dump",
            "ast",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(default, plantuml);
}

#[test]
fn strict_modes_parse_and_route_without_regression() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--compat",
            "strict",
            "--determinism",
            "strict",
            "--check",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn default_compat_matches_explicit_strict() {
    let default = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .clone();

    let strict = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--compat",
            "strict",
            "--check",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(default.stdout, strict.stdout);
    assert_eq!(default.stderr, strict.stderr);
}

#[test]
fn strict_stdin_include_requires_explicit_include_root() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("common.puml");
    fs::write(&include, "Bob -> Alice: from include\n").unwrap();
    let stdin_input = "@startuml\n!include common.puml\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "-", "--compat", "strict"])
        .write_stdin(stdin_input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_ROOT_REQUIRED"));
}

#[test]
fn extended_stdin_include_uses_current_directory_when_include_root_is_missing() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("common.puml");
    fs::write(&include, "Bob -> Alice: from include\n").unwrap();
    let stdin_input = "@startuml\n!include common.puml\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "-", "--compat", "extended"])
        .write_stdin(stdin_input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_frontend_fails_deterministically() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "picouml",
            "--check",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "frontend 'picouml' is not implemented yet",
        ));
}

#[test]
fn mermaid_sequence_subset_routes_through_shared_pipeline() {
    let src = r#"sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: hello
Bob-->>Alice: ack"#;

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_non_sequence_family_fails_deterministically() {
    let src = "graph TD\nA-->B";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_MERMAID_FAMILY_UNSUPPORTED]"));
}

#[test]
fn mermaid_unsupported_sequence_construct_fails_deterministically() {
    let src = r#"sequenceDiagram
alt happy path
Alice->>Bob: hello
end"#;
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[E_MERMAID_CONSTRUCT_UNSUPPORTED]",
        ));
}

#[test]
fn docs_examples_svg_corpus_matches_renderer() {
    for stem in ["basic_hello", "groups_notes", "lifecycle_autonumber"] {
        let puml_path = format!("{}/docs/examples/{stem}.puml", env!("CARGO_MANIFEST_DIR"));
        let svg_path = format!("{}/docs/examples/{stem}.svg", env!("CARGO_MANIFEST_DIR"));
        let source = fs::read_to_string(&puml_path).expect("example source");
        let expected_svg = fs::read_to_string(&svg_path).expect("example svg");
        let actual_svg = render_source_to_svg(&source).expect("rendered svg");
        assert_eq!(
            actual_svg, expected_svg,
            "docs example drift detected for {stem}"
        );
    }
}

#[test]
fn check_mode_fails_for_invalid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("invalid_single.puml")])
        .assert()
        .code(1);
}

#[test]
fn check_mode_passes_for_additional_valid_fixtures() {
    for case in [
        "single_valid.puml",
        "basic/valid_start_end.puml",
        "basic/valid_arrow.txt",
        "participants/valid_aliases.puml",
        "participants/valid_queue_separator.puml",
        "basic/valid_separator_equals.puml",
        "basic/valid_participant_queue.puml",
        "basic/valid_pragma_directives.puml",
        "basic/valid_arrows_extended_set.puml",
        "basic/valid_skinparam_maxmessagesize.puml",
        "arrows/valid_directions.puml",
        "arrows/self.puml",
        "arrows/modifiers_basic.puml",
        "arrows/valid_expanded_forms.puml",
        "arrows/valid_slanted_heads.puml",
        "arrows/valid_endpoint_variants.puml",
        "arrows/valid_arrow_portability_expanded.puml",
        "arrows/valid_arrow_slash_portability.puml",
        "arrows/valid_arrow_variant_tokenization.puml",
        "notes/valid_note_over.puml",
        "groups/valid_alt_end.puml",
        "groups/valid_loop_end.puml",
        "groups/valid_par_else_end.puml",
        "groups/valid_ref_and_else_rendering.puml",
        "groups/valid_group_nested_mixed_fragments.puml",
        "groups/valid_group_empty_group_block.puml",
        "autonumber/valid_basic.puml",
        "autonumber/valid_with_format.puml",
        "lifecycle/valid_activate_return.puml",
        "lifecycle/valid_create_activate_destroy.puml",
        "lifecycle/valid_shortcuts_expansion.puml",
        "lifecycle/valid_return_inferred_from_shortcut_activation.puml",
        "lifecycle/valid_return_inferred_from_last_message.puml",
        "notes/valid_multiline_blocks.puml",
        "notes/valid_note_across_multi.puml",
        "structure/valid_separator_delay_divider_spacer.puml",
        "structure/ignore_newpage_single_output.puml",
        "structure/valid_autonumber_restart_step_format.puml",
        "structure/valid_autonumber_format_only_and_canonical_spacing.puml",
        "structure/valid_autonumber_off_resume_edges.puml",
        "include/include_with_tag_ok.puml",
        "preprocessor/valid_if_elseif_else.puml",
        "preprocessor/valid_ifdef_ifndef.puml",
        "preprocessor/valid_while_define_counter.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn dump_mode_outputs_ast_json_for_multiline_blocks() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("notes/valid_multiline_blocks.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("dump_mode_outputs_ast_json_for_multiline_blocks", json);
}

#[test]
fn check_mode_fails_for_additional_invalid_fixtures() {
    for case in [
        "invalid_single.puml",
        "errors/invalid_plain.txt",
        "errors/invalid_unclosed.puml",
        "errors/invalid_unknown_only.puml",
        "errors/invalid_include_only.puml",
        "errors/invalid_define_only.puml",
        "errors/invalid_undef_only.puml",
        "non_sequence/invalid_state_diagram.puml",
        "include/error_include_cycle_self.puml",
        "include/error_include_chain_a.puml",
        "lifecycle/valid_destroy_then_message.puml",
        "lifecycle/invalid_return_without_caller_context.puml",
        "arrows/invalid_malformed_arrows.puml",
        "arrows/invalid_endpoint_variants.puml",
        "errors/invalid_malformed_note_ref.puml",
        "notes/invalid_note_position_target_required.puml",
        "structure/invalid_malformed_divider_delay.puml",
        "groups/invalid_else_without_open_group.puml",
        "groups/invalid_end_without_open_group.puml",
        "groups/invalid_else_inside_ref.puml",
        "groups/invalid_ref_block_missing_body.puml",
        "errors/invalid_separator_unbalanced_equals.puml",
        "errors/invalid_participant_queue_alias_collision.puml",
        "errors/invalid_arrow_variant_tokenization.puml",
        "errors/invalid_arrow_slash_tokenization.puml",
        "errors/invalid_include_tag_missing.puml",
        "errors/invalid_include_url.puml",
        "errors/invalid_else_inside_loop_group.puml",
        "errors/invalid_group_else_without_alt.puml",
        "errors/invalid_group_mismatched_end_keyword.puml",
        "errors/invalid_group_empty_alt.puml",
        "errors/invalid_group_empty_else_branch.puml",
        "errors/invalid_autonumber_bad_format_token.puml",
        "errors/invalid_preproc_conditional_order.puml",
        "errors/invalid_preproc_unclosed_if.puml",
        "errors/invalid_preproc_procedure_unsupported.puml",
        "errors/invalid_preproc_endwhile_without_while.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1);
    }
}

#[test]
fn else_inside_loop_group_reports_deterministic_normalize_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_else_inside_loop_group.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_GROUP_ELSE_KIND"));
}

#[test]
fn strict_group_semantics_accepts_nested_alt_par_critical_and_group() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/valid_group_nested_mixed_fragments.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn strict_group_semantics_allows_empty_group_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/valid_group_empty_group_block.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn strict_group_semantics_rejects_empty_alt_and_else_branches() {
    for case in [
        "errors/invalid_group_empty_alt.puml",
        "errors/invalid_group_empty_else_branch.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("E_GROUP_EMPTY"));
    }
}

#[test]
fn slash_arrow_variants_are_tokenized_into_message_arrows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("arrows/valid_arrow_slash_portability.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arrows: Vec<&str> = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["arrow"].as_str())
        .collect();

    assert_eq!(arrows, vec!["->", "->", "<->", "->o", "<<--x"]);
}

#[test]
fn malformed_slash_arrow_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_arrow_slash_tokenization.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains(
                    "A -//-> B: malformed-double-slash\n^^^^^^^^",
                ))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn expanded_arrow_variants_are_tokenized_into_message_arrows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("arrows/valid_arrow_variant_tokenization.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arrows: Vec<&str> = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["arrow"].as_str())
        .collect();

    assert_eq!(
        arrows,
        vec!["-/->", "-\\->", "-/->>", "-\\-->>", "o-/->x", "x-\\<<--o"]
    );
}

#[test]
fn malformed_arrow_variant_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_arrow_variant_tokenization.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 4, column 1")
                .and(predicate::str::contains("A -/--> B: malformed\n^^^^^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn check_mode_emits_styling_warnings_but_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_unsupported.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn check_mode_accepts_phase1_supported_skinparam_keys_without_warnings() {
    for fixture_name in [
        "styling/valid_skinparam_maxmessagesize_supported.puml",
        "styling/valid_skinparam_sequence_footbox_supported.puml",
        "styling/valid_skinparam_arrow_color_supported.puml",
        "styling/valid_skinparam_lifeline_border_color_supported.puml",
        "styling/valid_skinparam_participant_background_color_supported.puml",
        "styling/valid_skinparam_participant_border_color_supported.puml",
        "styling/valid_skinparam_note_background_color_supported.puml",
        "styling/valid_skinparam_note_border_color_supported.puml",
        "styling/valid_skinparam_group_background_color_supported.puml",
        "styling/valid_skinparam_group_border_color_supported.puml",
        "styling/valid_skinparam_sequence_alias_colors_supported.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(fixture_name)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn check_mode_skinparam_unsupported_key_and_value_are_both_reported_deterministically() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_unsupported_mixed_deterministic.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr should be valid utf-8");
    let unsupported_key = stderr
        .find("W_SKINPARAM_UNSUPPORTED")
        .expect("missing unsupported-key warning");
    let unsupported_value = stderr
        .find("W_SKINPARAM_UNSUPPORTED_VALUE")
        .expect("missing unsupported-value warning");
    assert!(
        unsupported_key < unsupported_value,
        "warnings should keep source order"
    );
}

#[test]
fn check_mode_skinparam_unsafe_color_value_warns_deterministically() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(
            "@startuml\nskinparam ArrowColor #aabbcc\nskinparam ArrowColor #ff0000\"/><script>\nA -> B\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(
            predicate::str::contains("W_SKINPARAM_UNSUPPORTED_VALUE")
                .and(predicate::str::contains("ArrowColor"))
                .and(predicate::str::contains("line 3, column 1")),
        );
}

#[test]
fn dump_mode_emits_warnings_in_deterministic_order() {
    let input = "@startuml\n!theme spacelab\nskinparam UnknownKey red\nskinparam StillUnknown blue\nA -> B\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "model", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = String::from_utf8(out.stderr).unwrap();
    let first = stderr.find("W_SKINPARAM_UNSUPPORTED").unwrap();
    let second = stderr[first + 1..].find("W_SKINPARAM_UNSUPPORTED").unwrap();
    let theme = stderr.find("W_THEME_UNSUPPORTED").unwrap();
    assert!(first < theme);
    assert!(first + 1 + second < theme);

    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json.get("participants").is_some());
}

#[test]
fn render_mode_emits_styling_warnings_but_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\n!theme plain\nskinparam UnknownKey red\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stderr(
            predicate::str::contains("W_SKINPARAM_UNSUPPORTED")
                .and(predicate::str::contains("W_THEME_UNSUPPORTED")),
        );
}

#[test]
fn source_related_warning_uses_line_column_and_caret_in_all_modes() {
    let input = "@startuml\nskinparam UnknownKey red\nA -> B\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "model", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );
}

#[test]
fn malformed_arrow_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("arrows/invalid_malformed_arrows.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_ARROW_INVALID"));
}

#[test]
fn malformed_endpoint_variant_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("arrows/invalid_endpoint_variants.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_ARROW_INVALID"));
}

#[test]
fn source_related_error_uses_line_column_and_caret_in_all_modes() {
    let invalid = fixture("arrows/invalid_malformed_arrows.puml");

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &invalid])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&invalid)
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn check_mode_reports_unmatched_enduml_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("errors/invalid_unmatched_enduml.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("without a preceding @startuml"));
}

#[test]
fn check_mode_reports_nested_startuml_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("errors/invalid_nested_startuml.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("before closing previous block"));
}

#[test]
fn check_mode_reports_unterminated_second_block_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_unterminated_second_block.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("missing a closing @enduml"));
}

#[test]
fn malformed_note_or_ref_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_malformed_note_ref.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_NOTE_INVALID"));
}

#[test]
fn malformed_ref_block_missing_body_reports_diagnostic_snapshot() {
    let invalid = fixture("groups/invalid_ref_block_missing_body.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_REF_INVALID"));
    assert_snapshot!(
        "malformed_ref_block_missing_body_reports_diagnostic",
        stderr
    );
}

#[test]
fn malformed_group_structure_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/invalid_else_without_open_group.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_GROUP_ELSE_UNMATCHED"));
}

#[test]
fn malformed_group_mismatched_end_keyword_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_group_mismatched_end_keyword.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_GROUP_END_KIND"));
    assert_snapshot!(
        "malformed_group_mismatched_end_keyword_reports_diagnostic",
        stderr
    );
}

#[test]
fn malformed_group_empty_alt_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_group_empty_alt.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_GROUP_EMPTY"));
    assert_snapshot!("malformed_group_empty_alt_reports_diagnostic", stderr);
}

#[test]
fn invalid_autonumber_bad_format_token_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_autonumber_bad_format_token.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_AUTONUMBER_FORMAT_UNSUPPORTED"));
    assert_snapshot!(
        "invalid_autonumber_bad_format_token_reports_diagnostic",
        stderr
    );
}

#[test]
fn dump_mode_requires_kind() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--dump")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "a value is required for '--dump <KIND>'",
        ));
}

#[test]
fn dump_mode_outputs_ast_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("dump_mode_outputs_ast_json", json);
}

#[test]
fn dump_mode_outputs_scene_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "scene", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert!(json.get("size").is_some());
    assert!(json.get("lanes").is_some());
    assert!(json.get("rows").is_some());
}

#[test]
fn dump_mode_scene_is_deterministic_for_same_input() {
    let first = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let first_json: Value = serde_json::from_slice(&first).unwrap();
    let second_json: Value = serde_json::from_slice(&second).unwrap();
    assert_eq!(first_json, second_json);
    assert_json_snapshot!(
        "dump_mode_scene_is_deterministic_for_same_input",
        first_json
    );
}

#[test]
fn dump_mode_scene_preserves_advanced_note_ref_forms_deterministically() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("overflow/overflow_note_ref_advanced_forms_nonoverlap.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_scene_preserves_advanced_note_ref_forms_deterministically",
        json
    );
}

#[test]
fn check_mode_accepts_advanced_note_ref_forms() {
    for case in [
        "notes/valid_note_advanced_forms.puml",
        "groups/valid_ref_advanced_forms.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn multi_mode_outputs_all_diagrams_as_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("multi_valid.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("multi_mode_outputs_all_diagrams_as_json", json);
}

#[test]
fn multi_mode_handles_three_diagrams() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/multi_three.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("multi_mode_handles_three_diagrams", json);
}

#[test]
fn multi_mode_splits_uppercase_start_enduml_blocks() {
    let input = "@STARTUML\nAlice -> Bob: one\n@ENDUML\n@STARTUML\nBob -> Alice: two\n@ENDUML\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected multi-dump array output");
    assert_eq!(arr.len(), 2);
}

#[test]
fn multi_mode_reports_unterminated_trailing_startuml_block() {
    let input = "@startuml\nAlice -> Bob: one\n@enduml\n@startuml\nBob -> Alice: two\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("missing a closing @enduml"));
}

#[test]
fn multi_mode_reports_enduml_without_startuml() {
    let input = "@enduml\n@startuml\nAlice -> Bob: one\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("without a preceding @startuml"));
}

#[test]
fn multi_input_without_flag_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(fs::read_to_string(fixture("multi_valid.puml")).unwrap())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("rerun with --multi"));
}

#[test]
fn stdin_input_is_supported() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!("stdin_input_is_supported", String::from_utf8(out).unwrap());
}

#[test]
fn stdin_dash_path_is_supported() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin("A -> B")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!(
        "stdin_dash_path_is_supported",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn missing_file_maps_to_io_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-12345.puml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn empty_input_maps_to_validation_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg(fixture("empty.txt"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn plain_multi_delimiter_supported_with_multi_flag() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("plain_multi.txt")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!(
        "plain_multi_delimiter_supported_with_multi_flag",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn check_and_dump_are_mutually_exclusive() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn from_markdown_extracts_fenced_blocks_in_source_order() {
    let input = "# doc\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\ntext\n```plantuml\n@startuml\nBob -> Alice: two\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    let first = arr[0]["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    let second = arr[1]["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(first, "one");
    assert_eq!(second, "two");
}

#[test]
fn from_markdown_supports_first_class_fence_frontends_and_aliases() {
    let input = fs::read_to_string(fixture("markdown/mixed_fences.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    let labels = arr
        .iter()
        .map(|doc| {
            doc["statements"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "puml-one",
            "pumlx-two",
            "picouml-three",
            "plantuml-four",
            "mermaid-five",
        ]
    );
}

#[test]
fn from_markdown_supports_legacy_sequence_fence_aliases() {
    let input = "```puml-sequence
@startuml
Alice -> Bob: one
@enduml
```
text
```uml-sequence
@startuml
Bob -> Alice: two
@enduml
```
";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["statements"][0]["kind"]["Message"]["label"], "one");
    assert_eq!(arr[1]["statements"][0]["kind"]["Message"]["label"], "two");
}

#[test]
fn from_markdown_supports_uml_fence_alias() {
    let input = "```uml
@startuml
Alice -> Bob: uml-alias
@enduml
```
";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        json["statements"][0]["kind"]["Message"]["label"],
        "uml-alias"
    );
}

#[test]
fn from_markdown_ignores_non_fence_markdown_content() {
    let input = "# not a diagram\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: ok\n@enduml\n```\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn from_markdown_diagnostics_json_maps_to_markdown_line_column() {
    let input = "# header\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "--diagnostics", "json", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("diagnostics_json_error_contract_shape", json);
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(first["code"], "E_ARROW_INVALID");
    assert_eq!(first["severity"], "error");
    assert_eq!(first["line"], 4);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B: bad");
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("E_ARROW_INVALID"));
}

#[test]
fn stdin_markdown_multi_fences_require_multi_flag() {
    let input = fs::read_to_string(fixture("markdown/mixed_fences.md")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("rerun with --multi"));
}

#[test]
fn stdin_markdown_multi_outputs_snippet_named_json() {
    let input = fs::read_to_string(fixture("markdown/multipage_mixed.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "snippet-1-1.svg");
    assert_eq!(arr[1]["name"], "snippet-1-2.svg");
    assert_eq!(arr[2]["name"], "snippet-2.svg");
}

#[test]
fn diagnostics_default_mode_remains_human_readable() {
    let input = "# header\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("line 4, column 1"));
    assert!(stderr.contains("A -x B: bad\n^^^^^^"));
    assert!(!stderr.trim_start().starts_with("{\"diagnostics\""));
}

#[test]
fn from_markdown_ingests_mixed_fence_edge_cases_deterministically() {
    let input = fs::read_to_string(fixture("markdown/edge_cases.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    let labels = arr
        .iter()
        .map(|doc| {
            doc["statements"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec!["tilde-puml", "uppercase-mermaid", "three-space-indent"]
    );
}

#[test]
fn stdin_markdown_edge_cases_multi_outputs_name_supported_fences_only() {
    let input = fs::read_to_string(fixture("markdown/edge_cases.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "snippet-1.svg");
    assert_eq!(arr[1]["name"], "snippet-2.svg");
    assert_eq!(arr[2]["name"], "snippet-3.svg");
}

#[test]
fn from_markdown_unclosed_supported_fence_ingests_through_eof() {
    let input = fs::read_to_string(fixture("markdown/unclosed_fence_eof.md")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn from_markdown_without_supported_fences_reports_actionable_error() {
    let input = "# heading\n```rust\nfn main() {}\n```\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "no supported markdown diagram fences found",
        ))
        .stderr(predicate::str::contains("mermaid"));
}

#[test]
fn include_cycle_input_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_cycle_self.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_cycle_chain_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_chain_a.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_id_tag_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/include_with_tag_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_many_expands_each_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_many_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 2);
}

#[test]
fn include_once_expands_only_first_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_once_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 1);
}

#[test]
fn includesub_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/includesub_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_id_missing_tag_reports_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_include_tag_missing.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_TAG_NOT_FOUND"))
        .stderr(predicate::str::contains(
            "include tag 'MISSING_TAG' was not found",
        ));
}

#[test]
fn include_url_is_rejected_with_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("errors/invalid_include_url.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_UNSUPPORTED"))
        .stderr(predicate::str::contains(
            "!include URL targets are not supported",
        ));
}

#[test]
fn includesub_without_tag_is_rejected_with_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_includesub_missing_tag.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDESUB_TAG_REQUIRED"))
        .stderr(predicate::str::contains(
            "!includesub requires a target tag",
        ));
}

#[test]
fn include_variants_url_policy_is_rejected_deterministically() {
    for (case, directive) in [
        ("errors/invalid_include_url.puml", "!include"),
        ("errors/invalid_include_once_url.puml", "!include_once"),
        ("errors/invalid_includesub_url.puml", "!includesub"),
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("E_INCLUDE_URL_UNSUPPORTED"))
            .stderr(predicate::str::contains(format!(
                "{directive} URL targets are not supported"
            )));
    }
}

#[test]
fn preprocessor_if_elseif_else_emits_only_selected_branch() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_if_elseif_else.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["primary"]);
}

#[test]
fn preprocessor_while_executes_until_condition_is_false() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_while_define_counter.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["loop 2", "loop 1"]);
}

#[test]
fn preprocessor_function_procedure_assert_log_and_dump_are_minimally_compatible() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/valid_function_procedure_assert_log_dump.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn preprocessor_assert_false_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_assert_false.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_PREPROC_ASSERT"));
    assert_snapshot!("preprocessor_assert_false_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_unclosed_function_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_unclosed_function.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_FUNCTION_UNCLOSED"));
    assert_snapshot!("preprocessor_unclosed_function_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_conditional_and_while_balance_errors_are_deterministic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_conditional_order.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_ORDER"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_unclosed_if.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_UNCLOSED"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_endwhile_without_while.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_WHILE_UNEXPECTED"));
}

#[test]
fn lifecycle_after_destroy_is_rejected() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("lifecycle/valid_destroy_then_message.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("destroyed"));
}

#[test]
fn unsupported_state_inputs_fail_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_state_diagram.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_FAMILY_UNKNOWN"));
}

#[test]
fn class_object_usecase_bootstrap_inputs_pass_check() {
    for case in [
        "families/valid_class_bootstrap.puml",
        "families/valid_object_bootstrap.puml",
        "families/valid_usecase_bootstrap.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn class_object_usecase_bootstrap_render_stubs_are_deterministic() {
    for (case, marker) in [
        (
            "families/valid_class_bootstrap.puml",
            "Bootstrap stub for class diagrams",
        ),
        (
            "families/valid_object_bootstrap.puml",
            "Bootstrap stub for object diagrams",
        ),
        (
            "families/valid_usecase_bootstrap.puml",
            "Bootstrap stub for usecase diagrams",
        ),
    ] {
        let src = fs::read_to_string(fixture(case)).unwrap();
        let first = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let second = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        assert_eq!(
            first, second,
            "stub output should be deterministic for {case}"
        );
        let svg = String::from_utf8(first).unwrap();
        assert!(svg.contains(marker), "missing family marker for {case}");
    }
}

#[test]
fn autonumber_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("autonumber_is_preserved_in_model_dump", json);
}

#[test]
fn autonumber_restart_step_and_format_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_restart_step_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(
        autonumber_raw,
        vec![
            "10 5 \"[000]\"",
            "stop",
            "resume 2 \"R-00\"",
            "3 3 \"S-00\""
        ]
    );
}

#[test]
fn autonumber_raw_is_canonicalized_for_deterministic_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_format_only_and_canonical_spacing.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(autonumber_raw, vec!["\"ID-000\"", "resume \"ID-000\""]);
}

#[test]
fn autonumber_off_and_resume_edges_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_off_resume_edges.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(
        autonumber_raw,
        vec![
            "7 3 \"ID-00\"",
            "off",
            "resume \"R-00\"",
            "resume 5 \"R-00\""
        ]
    );
}

#[test]
fn lifecycle_shortcuts_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_shortcuts_expansion.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("lifecycle_shortcuts_are_preserved_in_model_dump", json);
}

#[test]
fn lifecycle_return_inference_from_shortcut_activation_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_return_inferred_from_shortcut_activation.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "lifecycle_return_inference_from_shortcut_activation_is_preserved_in_model_dump",
        json
    );
}

#[test]
fn lifecycle_return_inference_from_last_message_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_return_inferred_from_last_message.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "lifecycle_return_inference_from_last_message_is_preserved_in_model_dump",
        json
    );
}

#[test]
fn lifecycle_return_without_caller_context_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("lifecycle/invalid_return_without_caller_context.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_RETURN_INFER_CALLER"));
}

#[test]
fn queue_role_and_separator_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("participants/valid_queue_separator.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("queue_role_and_separator_are_preserved_in_model_dump", json);
}

#[test]
fn can_read_tempfile_input() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("sample.puml");
    fs::write(&input, "@startuml\nX -> Y\n@enduml\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .assert()
        .success();

    let output = tmp.path().join("sample.svg");
    assert!(output.exists());
    let svg = fs::read_to_string(output).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn dump_mode_scene_preserves_separator_delay_divider_and_spacer_rows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("structure/valid_separator_delay_divider_spacer.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_scene_preserves_separator_delay_divider_and_spacer_rows",
        json
    );
}

#[test]
fn stdin_include_requires_include_root_or_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("@startuml\n!include include_ok_child.puml\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "!include from stdin requires include_root option",
        ));
}

#[test]
fn stdin_include_with_include_root_passes() {
    let root = format!("{}/tests/fixtures/include", env!("CARGO_MANIFEST_DIR"));
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--include-root", &root, "-"])
        .write_stdin("@startuml\n!include include_ok_child.puml\n@enduml\n")
        .assert()
        .success();
}

#[test]
fn file_multi_output_with_o_writes_numbered_files() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("multi_three.puml");
    fs::copy(fixture("structure/multi_three.puml"), &input).unwrap();
    let out = tmp.path().join("diagram.svg");

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            input.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--multi",
        ])
        .assert()
        .success();

    assert!(tmp.path().join("diagram-1.svg").exists());
    assert!(tmp.path().join("diagram-2.svg").exists());
    assert!(tmp.path().join("diagram-3.svg").exists());
}

#[test]
fn stdin_newpage_without_multi_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("multiple pages detected"));
}

#[test]
fn stdin_newpage_with_multi_outputs_json_array_and_stable_order() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin("@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "stdin_newpage_with_multi_outputs_json_array_and_stable_order",
        json
    );
}

#[test]
fn stdin_newpage_cli_contract_modes_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/newpage_stdin_contract.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("stdin_newpage_cli_contract_modes", json);
}

#[test]
fn stdin_ignore_newpage_without_multi_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
}

#[test]
fn stdin_ignore_newpage_with_multi_still_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
}

#[test]
fn file_newpage_output_without_multi_writes_numbered_files() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::copy(fixture("structure/newpage_stdin_contract.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
}

#[test]
fn file_newpage_output_writes_numbered_files_with_multi_flag() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
}

#[test]
fn multipage_file_output_failure_does_not_leave_partial_writes() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::copy(fixture("structure/newpage_stdin_contract.puml"), &input).unwrap();
    let output = tmp.path().join("diagram.svg");
    let first = tmp.path().join("diagram-1.svg");

    fs::write(&first, "stable-original-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args([
            "--multi",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-content".to_string()
    );
    assert!(!tmp.path().join("diagram-2.svg").exists());
}

#[test]
fn file_ignore_newpage_output_writes_single_default_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("ignore_newpage.puml");
    fs::copy(
        fixture("structure/ignore_newpage_single_output.puml"),
        &input,
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("ignore_newpage.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-1.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-2.svg").exists());
}

#[test]
fn stdin_multi_blocks_with_newpage_flatten_into_stable_named_json_order() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            "@startuml\nA -> B : one\nnewpage Two\nB -> A : two\n@enduml\n\n@startuml\nX -> Y : three\n@enduml\n",
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "diagram-1.svg");
    assert_eq!(arr[1]["name"], "diagram-2.svg");
    assert_eq!(arr[2]["name"], "diagram-3.svg");
}

#[test]
fn stdin_multi_blocks_with_ignore_newpage_requires_multi() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "multiple diagrams detected; rerun with --multi",
        ));
}

#[test]
fn stdin_multi_blocks_with_ignore_newpage_and_multi_outputs_two_items() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "diagram-1.svg");
    assert_eq!(arr[1]["name"], "diagram-2.svg");
}

#[test]
fn file_input_infers_include_root_from_parent_directory() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("child.puml");
    let parent = tmp.path().join("parent.puml");
    fs::write(&include, "Alice -> Bob : from child\n").unwrap();
    fs::write(
        &parent,
        "@startuml\n!include child.puml\nBob -> Alice : from parent\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", parent.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn explicit_output_file_is_overwritten_with_latest_render() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("explicit.svg");
    fs::write(&out, "old-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            fixture("single_valid.puml").as_str(),
            "--output",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let written = fs::read_to_string(&out).unwrap();
    assert!(written.contains("<svg"));
    assert_ne!(written, "old-content");
}

#[test]
fn multi_page_output_with_root_path_reports_invalid_output_stem() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap(), "--output", "/"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot derive output stem"));
}

#[test]
fn explicit_output_with_missing_parent_reports_io_exit_code() {
    let tmp = tempdir().unwrap();
    let missing_parent = tmp.path().join("missing").join("out.svg");

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            fixture("single_valid.puml").as_str(),
            "--output",
            missing_parent.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));
}

#[test]
fn markdown_file_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.md");
    fs::write(
        &input,
        "# heading\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn markdown_file_default_render_output_uses_deterministic_snippet_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(dir.path().join("mixed_snippet_1-1.svg").exists());
    assert!(dir.path().join("mixed_snippet_1-2.svg").exists());
    assert!(dir.path().join("mixed_snippet_2.svg").exists());
    assert!(!dir.path().join("mixed-1.svg").exists());
    assert!(!dir.path().join("mixed-2.svg").exists());
}

#[test]
fn markdown_multi_output_failure_does_not_leave_partial_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    let first = dir.path().join("mixed_snippet_1-1.svg");
    fs::write(&first, "stable-original-snippet").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-snippet".to_string()
    );
    assert!(!dir.path().join("mixed_snippet_1-2.svg").exists());
}

#[test]
fn markdown_file_diagnostics_map_to_original_markdown_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("invalid.markdown");
    fs::write(
        &input,
        "# header\n\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--diagnostics", "json", input.to_str().unwrap()])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(first["line"], 5);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B: bad");
}

#[test]
fn lint_mode_requires_check_flag() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--lint-input", &fixture("single_valid.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--check"));
}

#[test]
fn lint_mode_json_report_supports_repeated_inputs_and_globs_with_stable_order() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("b_valid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("styling/valid_skinparam_unsupported.puml"),
        tmp.path().join("c_warning.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "b_valid.puml",
            "--lint-input",
            "a_invalid.puml",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["summary"]["total_files"], 3);
    assert_eq!(report["summary"]["passed_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 3);
    assert_eq!(report["summary"]["passed_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let files = report["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["path"], "a_invalid.puml");
    assert_eq!(files[1]["path"], "b_valid.puml");
    assert_eq!(files[2]["path"], "c_warning.puml");

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--> a_invalid.puml"));
}

#[test]
fn lint_mode_json_diagnostics_stay_on_stderr_and_report_stays_on_stdout() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("invalid_single.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "invalid_single.puml",
            "--diagnostics",
            "json",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["severity"], "error");
}

#[test]
fn lint_mode_human_report_succeeds_for_all_valid_inputs() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("a_valid.puml"),
    )
    .unwrap();
    fs::copy(fixture("basic/hello.puml"), tmp.path().join("b_valid.puml")).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "--lint-glob", "*.puml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "lint summary: files=2 passed=2 failed=0",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_mode_markdown_docs_glob_runs_end_to_end() {
    let tmp = tempdir().unwrap();
    fs::write(
        tmp.path().join("ok.md"),
        "# ok\n```puml\n@startuml\nAlice -> Bob: hello\n@enduml\n```\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("broken.md"),
        "# broken\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.md",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["line"], 4);
    assert_eq!(diagnostics["diagnostics"][0]["file"], "broken.md");
}

#[test]
fn lint_mode_json_diagnostics_aggregate_deterministically_across_files() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::write(
        tmp.path().join("b_warning.puml"),
        "@startuml\nskinparam SequenceFooColor #123456\nAlice -> Bob: ok\n@enduml\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    let entries = diagnostics["diagnostics"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["file"], "a_invalid.puml");
    assert_eq!(entries[0]["severity"], "error");
    assert_eq!(entries[1]["file"], "b_warning.puml");
    assert_eq!(entries[1]["severity"], "warning");
}

#[test]
fn clap_help_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("PicoUML polymorphic sequence CLI"))
        .stdout(predicate::str::contains(
            "Permit multiple stdin render outputs",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn clap_version_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("puml"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn exit_code_matrix_is_stable_for_success_validation_and_io() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .code(0);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--definitely-invalid-flag")
        .assert()
        .code(1);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-input-12.puml")
        .assert()
        .code(2);
}

#[test]
fn dump_capabilities_outputs_manifest_shape() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg("--dump-capabilities")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(json["server"], "puml-lsp");
    assert_eq!(json["protocol"], "3.17");
    assert!(json["languageFeatures"].is_array());
    assert!(json["customRequests"].is_array());
}

#[test]
fn check_fixture_uses_fixture_loader_and_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check-fixture", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_fixture_missing_file_maps_to_io_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            "/tmp/definitely-not-present-fixture-16.puml",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read fixture"));
}

#[test]
fn check_fixture_with_json_diagnostics_emits_warning_payload() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported.puml"),
            "--diagnostics",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let line = String::from_utf8(out).unwrap();
    let json: Value = serde_json::from_str(line.trim()).expect("valid json warning payload");
    assert_json_snapshot!("diagnostics_json_warning_contract_shape", json);
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(first["code"], "W_SKINPARAM_UNSUPPORTED");
    assert_eq!(first["severity"], "warning");
    assert_eq!(first["line"], 2);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "skinparam TotallyUnknownColor red");
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn diagnostics_json_writes_only_to_stderr_and_not_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            &fixture("invalid_single.puml"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::starts_with(
            "{\n  \"schema\": \"puml.diagnostics\"",
        ));
}

#[test]
fn stdin_empty_input_maps_to_validation_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn markdown_mdown_extension_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.mdown");
    fs::write(
        &input,
        "# heading\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
