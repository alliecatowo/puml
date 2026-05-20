use super::*;

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
fn nonuml_family_fixtures_render_nonempty_svg_depth_smoke() {
    let fixtures = [
        "non_sequence/valid_sdl.puml",
        "families/valid_sdl_shapes.puml",
        "non_sequence/valid_archimate.puml",
        "non_sequence/valid_nwdiag.puml",
        "non_sequence/valid_json.puml",
        "non_sequence/valid_yaml.puml",
        "non_sequence/valid_regex.puml",
        "non_sequence/valid_ebnf.puml",
        "non_sequence/valid_chart_bar.puml",
        "non_sequence/valid_chart_pie.puml",
        "non_sequence/valid_math.puml",
        "non_sequence/valid_ditaa.puml",
        "families/valid_math_complex.puml",
        "families/valid_ditaa_complex.puml",
    ];

    for case in fixtures {
        let src = fs::read_to_string(fixture(case)).expect("fixture should load");
        let svg = render_source_to_svg(&src).expect("render should succeed");
        assert!(svg.starts_with("<svg"), "expected svg root for {case}");
        assert!(
            svg.len() > 120,
            "expected non-trivial svg for {case}, got {} bytes",
            svg.len()
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
fn component_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_component_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn deployment_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_deployment_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn state_diagram_basic_check_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_state_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn activity_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/valid_activity_oldstyle_baseline.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn old_style_activity_renders_flow_nodes_instead_of_raw_source() {
    let svg = render_source_to_svg(include_str!(
        "../fixtures/families/valid_activity_old_style.puml"
    ))
    .expect("old-style activity should render");

    assert!(svg.contains("data-activity-kind=\"Start\""));
    assert!(svg.contains("data-activity-kind=\"Action\""));
    assert!(svg.contains("data-activity-kind=\"Stop\""));
    assert!(svg.contains(">Step1<"));
    assert!(svg.contains(">Step2<"));
    assert!(svg.contains("<line "));
    assert!(!svg.contains("(*) --&gt;"));
}

#[test]
fn timing_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_timing_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn non_sequence_mindmap_check_now_succeeds_with_baseline_renderer() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_mindmap_diagram.puml"),
        ])
        .assert()
        .code(0);
}

#[test]
fn non_sequence_wbs_check_now_succeeds_with_baseline_renderer() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/invalid_wbs_diagram.puml")])
        .assert()
        .code(0);
}

#[test]
fn gantt_and_chronology_baseline_inputs_pass_check() {
    for case in [
        "timeline/valid_gantt_baseline.puml",
        "timeline/valid_chronology_baseline.puml",
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
fn gantt_and_chronology_dump_model_is_stable() {
    for case in [
        "timeline/valid_gantt_baseline.puml",
        "timeline/valid_chronology_baseline.puml",
    ] {
        let out = Command::cargo_bin("puml")
            .expect("binary")
            .args(["--dump", "model", &fixture(case)])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json: Value = serde_json::from_slice(&out).unwrap();
        assert_json_snapshot!(
            format!("timeline_dump_model__{}", case.replace('/', "__")),
            json
        );
    }
}

#[test]
fn gantt_and_chronology_unsupported_baseline_syntax_is_deterministic() {
    for (case, code) in [
        (
            "errors/invalid_gantt_unsupported_baseline.puml",
            "E_GANTT_UNSUPPORTED",
        ),
        (
            "errors/invalid_chronology_unsupported_baseline.puml",
            "E_CHRONOLOGY_UNSUPPORTED",
        ),
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
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
        "arrows/valid_rare_arrow_styles.puml",
        "arrows/valid_dotted_parallel_sequence_edges.puml",
        "arrows/valid_teoz_overlapping_routes.puml",
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
        "structure/valid_autonumber_dotted_and_hash_padding.puml",
        "include/include_with_tag_ok.puml",
        "include/include_many_ok.puml",
        "include/include_once_ok.puml",
        "include/includesub_ok.puml",
        "preprocessor/valid_if_elseif_else.puml",
        "preprocessor/valid_ifdef_ifndef.puml",
        "preprocessor/valid_while_define_counter.puml",
        "preprocessor/valid_variable_assignment_reference.puml",
        "preprocessor/valid_function_call_args_defaults_keywords.puml",
        "preprocessor/valid_function_return_indented.puml",
        "preprocessor/valid_procedure_call_args.puml",
        "preprocessor/valid_import_stdlib_core.puml",
        "preprocessor/valid_import_stdlib_nested_no_ext.puml",
        "preprocessor/valid_builtin_strlen.puml",
        "preprocessor/valid_builtin_boolval.puml",
        "preprocessor/valid_builtin_chain.puml",
        "preprocessor/valid_builtin_list_map_stringification_assert_log.puml",
        "include/valid_include_once.puml",
        "include/valid_include_many.puml",
        "include/valid_includesub.puml",
        "include/valid_c4_context.puml",
        "include/valid_awslib_ec2.puml",
        "stdlib_include_tag/valid_stdlib_tagged_angle_include.puml",
        // preprocessor advanced directives
        "preprocessor/valid_while_variable_loop.puml",
        "preprocessor/valid_undef.puml",
        "preprocessor/valid_assert_true.puml",
        "preprocessor/valid_log_directive.puml",
        "preprocessor/valid_get_json_attribute.puml",
        "preprocessor/valid_get_variable_value.puml",
        "preprocessor/valid_feature_builtin.puml",
        "preprocessor/valid_newline_builtin.puml",
        "preprocessor/valid_retrieve_procedure_return.puml",
        "preprocessor/valid_function_exists.puml",
        "preprocessor/valid_variable_exists.puml",
        "preprocessor/valid_json_dot_bracket_access.puml",
        "preprocessor/valid_splitstr_regex.puml",
        "preprocessor/valid_macro_concat_body.puml",
        "preprocessor/valid_macro_expr_collection_depth.puml",
        "preprocessor/valid_unsafe_builtin_policy.puml",
        // MindMap/WBS hardening fixtures
        "families/valid_mindmap_palette.puml",
        "families/valid_wbs_progress.puml",
        "families/valid_mindmap_orientation.puml",
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
