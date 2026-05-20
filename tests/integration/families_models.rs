use super::*;

#[test]
fn import_url_disabled_produces_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--no-url-includes",
            &fixture("errors/invalid_import_url.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn include_path_shape_errors_are_deterministic() {
    let cases = [
        (
            "errors/invalid_include_absolute_path.puml",
            "E_INCLUDE_ABSOLUTE_PATH",
        ),
        (
            "errors/invalid_include_empty_path.puml",
            "E_INCLUDE_PATH_REQUIRED",
        ),
    ];

    for (path, code) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(path)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
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
fn non_sequence_inputs_fail_validation() {
    let (case, code) = (
        "errors/invalid_salt_block_mismatch.puml",
        "E_BLOCK_MISMATCH",
    );
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture(case)])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(code));
}

#[test]
fn extended_family_fixtures_pass_check_and_render_svg() {
    let cases = [
        // Wave 3-A (#490 #494) suppressed the leaky "<family> diagram" canvas
        // text. Each marker now asserts on a structural element actually emitted
        // by the renderer for that family.
        ("families/valid_component.puml", "«component»"),
        ("families/valid_deployment.puml", "<polygon"),
        ("families/valid_state.puml", "<rect"),
        ("families/valid_activity.puml", "<rect"),
        ("families/valid_timing.puml", "<text"),
        ("families/valid_timing_waveform.puml", "<polyline"),
    ];
    for (case, marker) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stderr(predicate::str::is_empty());

        let src = fs::read_to_string(fixture(case)).unwrap();
        let out = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let svg = String::from_utf8(out).expect("svg utf8");
        assert!(svg.contains("<svg"), "missing svg envelope for {case}");
        assert!(svg.contains("</svg>"), "missing svg close for {case}");
        assert!(
            svg.contains(marker),
            "expected marker `{marker}` for {case}"
        );
    }
}

#[test]
fn class_object_usecase_bootstrap_inputs_pass_check() {
    for case in [
        "families/valid_class_bootstrap.puml",
        "families/valid_object_bootstrap.puml",
        "families/valid_usecase_bootstrap.puml",
        "families/valid_salt_bootstrap.puml",
        "families/valid_class_members_block.puml",
        "families/valid_object_members_block.puml",
        "families/valid_usecase_members_block.puml",
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
        ("families/valid_class_bootstrap.puml", "User"),
        ("families/valid_object_bootstrap.puml", "Order"),
        ("families/valid_usecase_bootstrap.puml", "Authenticate"),
        ("families/valid_salt_bootstrap.puml", "submit_button"),
        ("families/valid_class_members_block.puml", "+id: UUID"),
        ("families/valid_object_members_block.puml", "token = abc123"),
        ("families/valid_usecase_members_block.puml", "Authenticate"),
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
fn family_member_block_render_snapshot_is_deterministic() {
    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(
            fs::read_to_string(fixture("families/valid_class_members_block.puml")).unwrap(),
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_snapshot!(
        "family_member_block_render_snapshot_is_deterministic",
        String::from_utf8(svg).unwrap()
    );
}

#[test]
fn family_member_blocks_are_preserved_in_ast_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("families/valid_class_members_block.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let members = json["statements"][0]["kind"]["ClassDecl"]["members"]
        .as_array()
        .expect("members should be present");
    assert_eq!(members.len(), 3);
    // Members are now objects with "text" and "modifier" fields
    assert_eq!(members[0]["text"], "+id: UUID");
    assert_eq!(members[0]["modifier"], serde_json::Value::Null);
}

#[test]
fn unclosed_family_declaration_block_reports_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_FAMILY_DECL_BLOCK_UNCLOSED"))
        .stderr(predicate::str::contains("missing `}`"));
}

#[test]
fn extended_families_render_to_deterministic_svg() {
    let cases = [
        ("non_sequence/valid_regex.puml", "<svg"),
        ("non_sequence/valid_ebnf.puml", "<svg"),
        ("non_sequence/valid_math.puml", "<svg"),
        ("non_sequence/valid_sdl.puml", "<svg"),
        ("non_sequence/valid_ditaa.puml", "<svg"),
        ("non_sequence/valid_chart_bar.puml", "<svg"),
        ("non_sequence/valid_chart_pie.puml", "<svg"),
    ];
    for (case, marker) in cases {
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
        assert_eq!(first, second, "render must be deterministic for {case}");
        let svg = String::from_utf8(first).unwrap();
        assert!(
            svg.contains(marker),
            "missing marker `{marker}` for {case}; got: {svg}"
        );
    }
}

#[test]
fn extended_families_pass_check() {
    for case in [
        "non_sequence/valid_regex.puml",
        "non_sequence/valid_ebnf.puml",
        "non_sequence/valid_math.puml",
        "non_sequence/valid_sdl.puml",
        "non_sequence/valid_ditaa.puml",
        "non_sequence/valid_chart_bar.puml",
        "non_sequence/valid_chart_pie.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success();
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
fn autonumber_dotted_and_hash_padding_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_dotted_and_hash_padding.puml"),
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
        vec!["1.02.003", "7 2 \"ID-###\"", "stop", "resume 5 \"R-###\""]
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
