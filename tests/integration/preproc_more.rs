use super::*;

#[test]
fn stdrpt_flag_formats_error_as_single_tab_separated_line() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stderr.lines().collect();
    // Exactly one line per diagnostic
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one stdrpt line, got: {stderr:?}"
    );
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(
        parts.len(),
        4,
        "expected 4 tab-separated fields, got: {:?}",
        parts
    );
    assert_eq!(parts[0], "error", "first field should be severity");
    // second field is code, third is location, fourth is message
    assert!(!parts[3].is_empty(), "message field should not be empty");
}

#[test]
fn stdrpt_flag_location_includes_file_and_line_col() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    let line = stderr.lines().next().unwrap_or("");
    let parts: Vec<&str> = line.split('\t').collect();
    // location field (index 2) must contain a colon-separated path:line:col
    let location = parts.get(2).copied().unwrap_or("");
    assert!(
        location.contains(':'),
        "location field should contain colons: {location:?}"
    );
}

#[test]
fn stdrpt_does_not_emit_multiline_source_context() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    // No caret lines (lines starting with spaces + ^^^)
    for line in stderr.lines() {
        assert!(
            !line.trim_start().starts_with('^'),
            "stdrpt should suppress caret lines, found: {line:?}"
        );
    }
}

#[test]
fn stdrpt_exit_code_semantics_unchanged_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--stdrpt", "--check", &fixture("single_valid.puml")])
        .assert()
        .success();
}

// ─── Preprocessor advanced directives ────────────────────────────────────────

#[test]
fn preproc_newline_builtin_returns_newline_char() {
    let src = "@startuml\n!$nl = %newline()\nA -> B : ok\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

// ── Issue #188: Full PicoUML native syntax ────────────────────────────────────

#[test]
fn picouml_full_constructs_passes_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "picouml",
            "--check",
            &fixture("picouml/valid_full_constructs.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn preproc_feature_builtin_returns_false_for_unknown() {
    let src = "@startuml\n!$f = %feature(\"nosuchfeature\")\nA -> B : %feature(\"x\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("feature builtin should work");
    assert!(svg.contains("false"), "expected 'false' from %feature");
}

#[test]
fn preproc_variable_exists_returns_correct_bool() {
    let src = "@startuml\n!$x = hello\nA -> B : %variable_exists(\"x\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("variable_exists should work");
    assert!(
        svg.contains("true"),
        "expected 'true' for existing variable"
    );
}

#[test]
fn preproc_function_exists_detects_defined_function() {
    let src = "@startuml\n!function MyFn($a)\n!return $a\n!endfunction\nA -> B : %function_exists(\"MyFn\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("function_exists should work");
    assert!(svg.contains("true"), "expected 'true' for defined function");
}

#[test]
fn preproc_get_json_attribute_nested_path() {
    // Simple flat key — nested path traversal
    let src = "@startuml\n!$cfg = { \"name\": \"beta\" }\nA -> B : %get_json_attribute($cfg, \"name\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("get_json_attribute should work");
    assert!(svg.contains("beta"), "expected 'beta' from JSON attribute");
}

#[test]
fn preproc_retrieve_procedure_return_is_empty_in_deterministic_model() {
    let src = "@startuml\n!$ret = %retrieve_procedure_return()\nA -> B : done\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

#[test]
fn preproc_while_loop_with_variable_counter_expands_correctly() {
    let src = "@startuml\n!$i = 0\n!while $i < 3\n!$i = $i + 1\nA$i -> B$i\n!endwhile\n@enduml\n";
    let svg = render_source_to_svg(src).expect("while loop should work");
    // Should have produced 3 messages
    assert!(svg.contains("A1"), "expected A1 in output");
    assert!(svg.contains("A2"), "expected A2 in output");
    assert!(svg.contains("A3"), "expected A3 in output");
}

#[test]
fn preproc_expression_word_operators_and_string_builtins_expand() {
    let src = "@startuml\n!$raw = \"  Alpha-Beta  \"\n!assert %contains(%trim($raw), \"Alpha\") and %startswith(%trim($raw), \"Alpha\")\nA -> B : %replace(%lower(%trim($raw)), \"-\", \":\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("string builtins should expand");
    assert!(svg.contains("alpha:beta"), "expected replacement output");
}

#[test]
fn preproc_list_and_map_builtins_are_deterministic_json_strings() {
    let src = "@startuml\n!$items = %list(\"red\", \"blue\")\n!$items = %list_add($items, \"green\")\n!$cfg = %map(\"name\", \"Ada\", \"role\", \"admin\")\n!$cfg = %map_put($cfg, \"team\", %join($items, \"/\"))\n!assert %list_contains($items, \"blue\") and %map_contains_key($cfg, \"team\")\nA -> B : %list_get($items, 2) / %get($cfg, \"team\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("list/map builtins should expand");
    assert!(svg.contains("green /"), "expected list_get output");
    assert!(
        svg.contains("red/blue/green"),
        "expected joined map value output"
    );
}

#[test]
fn preproc_nested_json_mutation_and_projection_helpers_expand() {
    let src = "@startuml\n!$cfg = {\"users\":[{\"name\":\"Ada\",\"role\":\"dev\"}],\"meta\":{\"version\":1}}\n!$cfg = %json_set($cfg, \"users[0].role\", \"admin\")\n!$cfg = %json_set($cfg, \"meta.tags[0]\", \"stable\")\n!$cfg = %json_merge($cfg, {\"meta\":{\"build\":2},\"extra\":true})\n!$cfg = %json_remove($cfg, \"users[0].name\")\n!$items = %list_sort(%list_remove(%list(\"zeta\", \"alpha\", \"beta\"), \"zeta\"))\n!assert %json_key_exists($cfg, \"meta.tags[0]\") and %json_is_valid($cfg)\nA -> B : %get($cfg, \"users[0].role\") / %get($cfg, \"meta.tags[0]\") / %get($cfg, \"meta.build\") / %json_type($cfg) / %join($items, \":\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("nested JSON helpers should expand");
    assert!(
        svg.contains("admin / stable / 2 /") && svg.contains("object / alpha:beta"),
        "expected nested mutation, merge, remove, type, and sorted list output"
    );
    assert!(
        !svg.contains("Ada"),
        "json_remove should delete the nested name before rendering"
    );
}

#[test]
fn preproc_undef_removes_define() {
    // After !undef, the define should no longer expand
    let src = "@startuml\n!define GREETING hello\n!undef GREETING\nA -> B : ok\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

// ─── MindMap / WBS rendering ──────────────────────────────────────────────────
