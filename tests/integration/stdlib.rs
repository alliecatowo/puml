use super::*;

#[test]
fn stdlib_c4_context_check_passes_and_ast_has_object_declarations() {
    // --check must succeed (requires normalize_family to accept Object diagram).
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/valid_c4_context.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // AST dump must show ObjectDecl nodes with macro-expanded names and aliases.
    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/valid_c4_context.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");

    // Diagram must be Object kind (C4 stubs emit `object` declarations).
    assert_eq!(
        ast["kind"], "Object",
        "C4 context fixture must produce Object diagram"
    );

    let stmts = ast["statements"].as_array().expect("statements array");

    // Person(u, "User") -> ObjectDecl { name: "User", alias: "u <<person>>" }
    let user_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "User")
        .expect("User ObjectDecl from Person() macro");
    assert!(
        user_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<person>>"),
        "Person macro alias must contain <<person>> stereotype"
    );

    // System(s, "Software System") -> ObjectDecl { name: "Software System", alias: "s <<system>>" }
    let sys_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "Software System")
        .expect("Software System ObjectDecl from System() macro");
    assert!(
        sys_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<system>>"),
        "System macro alias must contain <<system>> stereotype"
    );

    // Rel(u, s, "Uses") -> FamilyRelation { from: "u", to: "s" }
    let rel = stmts
        .iter()
        .find(|s| {
            s["kind"]["FamilyRelation"]["from"] == "u" && s["kind"]["FamilyRelation"]["to"] == "s"
        })
        .expect("Rel(u, s) FamilyRelation");
    assert_eq!(rel["kind"]["FamilyRelation"]["arrow"], "-->");
}

#[test]
fn stdlib_awslib_ec2_check_passes_and_ast_has_object_declarations() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/valid_awslib_ec2.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/valid_awslib_ec2.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");

    assert_eq!(
        ast["kind"], "Object",
        "AWS EC2 fixture must produce Object diagram"
    );

    let stmts = ast["statements"].as_array().expect("statements array");

    // EC2(server, "App Server") -> ObjectDecl { name: "App Server", alias: "server <<aws-ec2>>" }
    let server_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "App Server")
        .expect("App Server ObjectDecl from EC2() macro");
    assert!(
        server_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<aws-ec2>>"),
        "EC2 macro alias must contain <<aws-ec2>> stereotype"
    );

    // Rel(server, cache, "reads from") -> FamilyRelation
    let rel = stmts
        .iter()
        .find(|s| s["kind"]["FamilyRelation"]["from"] == "server")
        .expect("Rel(server, cache) FamilyRelation");
    assert_eq!(rel["kind"]["FamilyRelation"]["to"], "cache");
}

#[test]
fn c4_multiple_rel_on_same_pair_coalesces_labels_with_newline_not_concatenation() {
    // Regression test for #425: multiple Rel() calls between the same source→target
    // pair must NOT produce "Uses HTTPSSends emails" (concatenated without separator).
    // They must coalesce into one relation whose label is "Uses HTTPS\nSends emails",
    // rendered as stacked tspan elements in the SVG output.
    // Inline the C4 Rel() procedure via stdin so SVG goes to stdout.
    // The C4 Rel() macro expands to `$from --> $to : $label`.
    let puml_src = "\
        @startuml\n\
        !procedure Rel($from, $to, $label, $tech=\"\")\n\
        $from --> $to : $label\n\
        !endprocedure\n\
        object User as user <<person>>\n\
        object API as api <<system>>\n\
        !Rel(user, api, \"Uses HTTPS\")\n\
        !Rel(user, api, \"Sends emails\")\n\
        @enduml\n";

    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args(["-"])
        .write_stdin(puml_src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(output).expect("UTF-8 SVG");

    // Must NOT contain concatenated label.
    assert!(
        !svg.contains("Uses HTTPSSends"),
        "labels must not be concatenated without separator"
    );
    assert!(
        !svg.contains("Uses HTTPS\nSends"),
        "raw newline in SVG text is invisible — must be converted to tspan"
    );
    // Must contain each label text.
    assert!(svg.contains("Uses HTTPS"), "first label must appear");
    assert!(svg.contains("Sends emails"), "second label must appear");
    // Must use tspan for multi-line rendering (#425).
    assert!(
        svg.contains("<tspan") && svg.contains("Uses HTTPS") && svg.contains("Sends emails"),
        "multiline label must use <tspan> elements"
    );
    // Must have exactly ONE polyline between user and api (merged relation, not two overlapping).
    let user_api_arrow_count = svg
        .matches("data-uml-from=\"user\" data-uml-to=\"api\"")
        .count();
    assert_eq!(
        user_api_arrow_count, 1,
        "duplicate Rel() on same pair must coalesce to a single arrow, got {user_api_arrow_count}"
    );
}

#[test]
fn stdlib_angle_bracket_include_is_idempotent_when_included_twice() {
    // Including the same stdlib file twice must not cause duplicate procedure errors.
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("double_include.puml");
    fs::write(
        &input,
        "@startuml\n!include <C4/C4_Context>\n!include <C4/C4_Context>\n!Person(u, User)\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_angle_bracket_include_supports_tagged_fixture() {
    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("stdlib_include_tag/valid_stdlib_tagged_angle_include.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");
    let stmts = ast["statements"].as_array().expect("statements array");
    assert_eq!(
        stmts.len(),
        1,
        "tagged stdlib include must omit untagged body lines"
    );
    assert_eq!(stmts[0]["kind"]["Message"]["from"], "Alice");
    assert_eq!(stmts[0]["kind"]["Message"]["to"], "Bob");
    assert_eq!(
        stmts[0]["kind"]["Message"]["label"],
        "from tagged stdlib include"
    );
}
