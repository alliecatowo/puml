use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn stdlib_flag_lists_reachable_local_paths_and_aliases() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("-stdlib")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "# PUML local stdlib inventory (deterministic shim subset",
        ))
        .stdout(predicate::str::contains("# alias: awslib -> awslib14"))
        .stdout(predicate::str::contains("# alias: material2 -> material"))
        .stdout(predicate::str::contains(
            "# alias: material2.1.19 -> material",
        ))
        .stdout(predicate::str::contains("C4/C4_Context.puml\n"))
        .stdout(predicate::str::contains("material/folder.puml\n"))
        .stdout(predicate::str::contains("openiconic/all.puml\n"))
        .stdout(predicate::str::contains("openiconic/folder.puml\n"))
        .stdout(predicate::str::contains(
            "awslib/Compute/EC2.puml -> awslib14/Compute/EC2.puml\n",
        ))
        .stdout(predicate::str::contains("awslib14/Compute/EC2.puml\n"))
        .stdout(predicate::str::contains(
            "material2.1.19/folder_move.puml -> material/folder_move.puml\n",
        ))
        .stdout(predicate::str::contains("bootstrap"))
        .stdout(predicate::str::contains("openiconic"))
        .stdout(predicate::str::contains("# missing upstream packs:"))
        .stdout(predicate::str::contains("# builtin packs: openiconic"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_flag_output_paths_are_sorted() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .arg("--stdlib")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("utf8 stdout");
    let paths = stdout
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>();
    let mut sorted = paths.clone();
    sorted.sort();

    assert_eq!(paths, sorted);
}

#[test]
fn stdlib_catalog_diagram_renders_from_registry_path() {
    let source = std::fs::read_to_string(fixture(
        "stdlib_catalog/valid_stdlib_inventory_diagram.puml",
    ))
    .expect("fixture");

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-", "--format", "svg"])
        .write_stdin(source)
        .assert()
        .success()
        .stdout(predicate::str::contains("data-stdlib-catalog=\"true\""))
        .stdout(predicate::str::contains("Local stdlib inventory"))
        .stdout(predicate::str::contains(">C4<"))
        .stdout(predicate::str::contains(">material<"))
        .stdout(predicate::str::contains(">bootstrap<"))
        .stdout(predicate::str::contains(">openiconic<"))
        .stdout(predicate::str::contains("unavailable"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_catalog_diagram_text_output_is_deterministic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-", "--format", "txt"])
        .write_stdin("@startuml\ntitle Local stdlib inventory\nstdlib\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdlib\n"))
        .stdout(predicate::str::contains("available C4"))
        .stdout(predicate::str::contains("builtin openiconic"))
        .stdout(predicate::str::contains("unavailable bootstrap"))
        .stdout(predicate::str::contains("material2.1.19 -> material"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn unavailable_stdlib_pack_diagnostic_lists_available_and_missing_packs() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            &fixture("stdlib_catalog/invalid_bootstrap_include.puml"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"code\": \"E_INCLUDE_STDLIB_PACK_UNAVAILABLE\"",
        ))
        .stderr(predicate::str::contains(
            "stdlib pack 'bootstrap' is not bundled",
        ))
        .stderr(predicate::str::contains("available packs: C4"))
        .stderr(predicate::str::contains("material2.1.19"))
        .stderr(predicate::str::contains(
            "known unavailable upstream packs:",
        ));
}

#[test]
fn openiconic_stdlib_include_resolves_to_builtin_sprite_contract() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_openiconic_include.puml"))
        .expect("fixture");
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-", "--format", "svg"])
        .write_stdin(source)
        .assert()
        .success()
        .stdout(predicate::str::contains("data-creole-sprites=\"true\""))
        .stdout(predicate::str::contains("data-sprite=\"folder\""))
        .stdout(predicate::str::contains("data-sprite=\"cloud-upload\""))
        .stdout(predicate::str::contains("fill=\"#2563eb\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn openiconic_stdlib_pack_include_resolves_all_builtin_sprites() {
    let source = "@startuml
!include <openiconic/all>
Alice -> Bob : Pack <$folder,scale=2,color=#1d4ed8> and <$cloud-upload>
@enduml
";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-", "--format", "svg"])
        .write_stdin(source)
        .assert()
        .success()
        .stdout(predicate::str::contains("data-creole-sprites=\"true\""))
        .stdout(predicate::str::contains("data-sprite=\"folder\""))
        .stdout(predicate::str::contains("data-sprite=\"cloud-upload\""))
        .stdout(predicate::str::contains("fill=\"#1d4ed8\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn openiconic_stdlib_import_resolves_to_same_builtin_sprite_contract() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_openiconic_import.puml"))
        .expect("fixture");
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-", "--format", "svg"])
        .write_stdin(source)
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported"))
        .stdout(predicate::str::contains("data-sprite=\"folder\""))
        .stdout(predicate::str::contains("fill=\"#0f766e\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn unavailable_stdlib_import_uses_same_pack_diagnostic_contract() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            "--include-root",
            env!("CARGO_MANIFEST_DIR"),
            &fixture("stdlib_catalog/invalid_bootstrap_import.puml"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"code\": \"E_INCLUDE_STDLIB_PACK_UNAVAILABLE\"",
        ))
        .stderr(predicate::str::contains(
            "stdlib pack 'bootstrap' is not bundled",
        ))
        .stderr(predicate::str::contains("available packs: C4"))
        .stderr(predicate::str::contains("openiconic"));
}

#[test]
fn unknown_stdlib_pack_diagnostic_is_distinct_from_unavailable_pack() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            &fixture("stdlib_catalog/invalid_unknown_pack_include.puml"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"code\": \"E_INCLUDE_STDLIB_NOT_FOUND\"",
        ))
        .stderr(
            predicate::str::contains(
                "stdlib include '&lt;does-not-exist/example&gt;' was not found",
            )
            .or(predicate::str::contains(
                "stdlib include '<does-not-exist/example>' was not found",
            )),
        )
        .stderr(predicate::str::contains(
            "known unavailable upstream packs:",
        ));
}
