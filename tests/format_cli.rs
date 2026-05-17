use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn format_command_formats_file_in_place_and_is_idempotent() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("messy.puml");
    fs::write(
        &input,
        "@startuml\r\nAlice → Bob: hi   \r\nalt ok\r\nBob ← Alice: yes\r\nelse no\r\nAlice ⇒ Bob: retry\r\nend\r\n@enduml\r\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["format", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let expected = "@startuml\nAlice -> Bob: hi\nalt ok\n  Bob <- Alice: yes\nelse no\n  Alice --> Bob: retry\nend\n@enduml\n";
    assert_eq!(fs::read_to_string(&input).unwrap(), expected);

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["format", "--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn format_check_exits_nonzero_without_modifying_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("needs-format.puml");
    let messy = "@startuml\nalt ok\nAlice → Bob: hi  \nend\n@enduml\n";
    fs::write(&input, messy).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["format", "--check", input.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("formatting changes needed"));

    assert_eq!(fs::read_to_string(&input).unwrap(), messy);
}

#[test]
fn format_diff_prints_readable_diff_without_modifying_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("needs-diff.puml");
    let messy = "@startuml\nalt ok\nAlice → Bob: hi  \nend\n@enduml\n";
    fs::write(&input, messy).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["format", "--diff", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("@@ -1,5 +1,5 @@"))
        .stdout(predicate::str::contains("-Alice → Bob: hi"))
        .stdout(predicate::str::contains("+  Alice -> Bob: hi"))
        .stderr(predicate::str::is_empty());

    assert_eq!(fs::read_to_string(&input).unwrap(), messy);
}
