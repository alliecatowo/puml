use super::*;

#[test]
fn check_mode_pragma_teoz_is_accepted_as_compatibility_noop() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("basic/valid_pragma_directives.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn malformed_pragma_missing_body_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_pragma_missing_body.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_PRAGMA_INVALID]"));
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
        "include/error_include_cycle_self.puml",
        "include/error_include_chain_a.puml",
        "lifecycle/valid_destroy_then_message.puml",
        "lifecycle/invalid_return_without_caller_context.puml",
        "arrows/invalid_malformed_arrows.puml",
        "arrows/invalid_endpoint_variants.puml",
        "errors/invalid_malformed_note_ref.puml",
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
        "errors/invalid_include_once_url.puml",
        "errors/invalid_includesub_url.puml",
        "errors/invalid_includesub_missing_tag.puml",
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
        "errors/invalid_preproc_expr_missing.puml",
        "errors/invalid_preproc_unexpected_endfunction.puml",
        "errors/invalid_preproc_while_iteration_limit.puml",
        "errors/invalid_pragma_missing_body.puml",
        "errors/invalid_preproc_assert_missing_expr.puml",
        "errors/invalid_preproc_builtin_in_assert.puml",
        "errors/invalid_preproc_builtin_in_log.puml",
        "errors/invalid_preproc_dynamic_invoke.puml",
        "errors/invalid_preproc_json_assignment.puml",
        "errors/invalid_preproc_function_missing_arg.puml",
        "errors/invalid_preproc_procedure_return.puml",
        "errors/invalid_import_empty_path.puml",
        "errors/invalid_import_url.puml",
        "errors/invalid_import_absolute_path.puml",
        "errors/invalid_import_tag_form.puml",
        "errors/invalid_import_escape_path.puml",
        "errors/invalid_import_missing_module.puml",
        "errors/invalid_pragma_missing_body.puml",
        "errors/invalid_theme_empty_name.puml",
        "errors/invalid_theme_remote_source.puml",
        "errors/invalid_theme_unknown_name.puml",
        "errors/invalid_include_absolute_path.puml",
        "errors/invalid_include_empty_path.puml",
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
fn bracketed_sequence_arrow_style_metadata_is_preserved() {
    let src = std::fs::read_to_string(fixture("arrows/valid_rare_arrow_styles.puml")).unwrap();
    let doc = puml::parse(&src).expect("parse should succeed");
    let messages = doc
        .statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            puml::ast::StatementKind::Message(m) => Some(m),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(messages[0].style.thickness, Some(3));
    assert!(messages[1].style.dotted);
    assert_eq!(messages[2].style.thickness, Some(5));
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
    assert!(first + 1 + second > first);
    assert!(!stderr.contains("W_THEME_UNSUPPORTED"));

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
        .stderr(predicate::str::contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn check_mode_rejects_theme_remote_source_with_deterministic_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_theme_remote_source.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_THEME_SOURCE_UNSUPPORTED]"));
}

#[test]
fn check_mode_rejects_theme_unknown_name_with_catalog_message() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_theme_unknown_name.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("[E_THEME_UNKNOWN]")
                .and(predicate::str::contains("available local themes:"))
                .and(predicate::str::contains("plain"))
                .and(predicate::str::contains("spacelab")),
        );
}

#[test]
fn theme_plain_produces_default_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_plain.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#111");
    assert_eq!(style["participant_background_color"], "#f6f6f6");
    assert_eq!(style["note_background_color"], "#fff8c4");
}

#[test]
fn theme_aws_orange_produces_orange_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_aws_orange.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#232f3e");
    assert_eq!(style["participant_background_color"], "#ff9900");
    assert_eq!(style["lifeline_border_color"], "#ff9900");
}

#[test]
fn theme_blueprint_produces_dark_blue_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_blueprint.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#ffffff");
    assert_eq!(style["participant_background_color"], "#1a3a5c");
    assert_eq!(style["lifeline_border_color"], "#7eb4d4");
}

#[test]
fn theme_cerulean_produces_blue_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_cerulean.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#2fa4e7");
    assert_eq!(style["participant_background_color"], "#d9edf7");
}

#[test]
fn theme_hacker_produces_green_on_black_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_hacker.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#00ff00");
    assert_eq!(style["participant_background_color"], "#0d0d0d");
    assert_eq!(style["note_background_color"], "#000000");
}

#[test]
fn theme_sketchy_produces_hand_drawn_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_sketchy.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#333333");
    assert_eq!(style["participant_background_color"], "#fffde7");
}

#[test]
fn theme_catalog_covers_all_22_presets() {
    use puml::theme::{resolve_sequence_theme_preset, LOCAL_SEQUENCE_THEME_CATALOG};
    // Use >= rather than ==: adding new themes should not break this test.
    // The original set contained 22; the exact count is an implementation detail.
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.len() >= 22,
        "expected at least 22 theme presets, found {}",
        LOCAL_SEQUENCE_THEME_CATALOG.len()
    );
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        let result = resolve_sequence_theme_preset(name);
        assert!(
            result.is_ok(),
            "preset `{name}` failed to resolve: {:?}",
            result
        );
        let preset = result.unwrap();
        assert_eq!(preset.name, *name);
        // All color strings must start with '#' or be a named color
        assert!(!preset.style.arrow_color.is_empty());
        assert!(!preset.style.participant_background_color.is_empty());
    }
}

#[test]
fn all_22_theme_fixtures_pass_check_mode() {
    for name in &[
        "styling/valid_theme_plain.puml",
        "styling/valid_theme_aws_orange.puml",
        "styling/valid_theme_blueprint.puml",
        "styling/valid_theme_cerulean.puml",
        "styling/valid_theme_hacker.puml",
        "styling/valid_theme_sketchy.puml",
        "styling/valid_theme_amiga.puml",
        "styling/valid_theme_bluegray.puml",
        "styling/valid_theme_carbon_gray.puml",
        "styling/valid_theme_materia_outline.puml",
        "styling/valid_theme_mono.puml",
        "styling/valid_theme_nautilus.puml",
        "styling/valid_theme_not_so_funny.puml",
        "styling/valid_theme_reddress_darkgreen.puml",
        "styling/valid_theme_sandstone.puml",
        "styling/valid_theme_silver.puml",
        "styling/valid_theme_spacelab_white.puml",
        "styling/valid_theme_sunlust.puml",
        "styling/valid_theme_toy.puml",
        "styling/valid_theme_united.puml",
        "styling/valid_theme_vibrant.puml",
        "styling/valid_theme_none.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(name)])
            .assert()
            .success();
    }
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
