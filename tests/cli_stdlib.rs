use assert_cmd::Command;
use predicates::prelude::*;

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
        .stdout(predicate::str::contains("C4/C4_Context.puml\n"))
        .stdout(predicate::str::contains(
            "awslib/Compute/EC2.puml -> awslib14/Compute/EC2.puml\n",
        ))
        .stdout(predicate::str::contains("awslib14/Compute/EC2.puml\n"))
        .stdout(predicate::str::contains("# missing upstream packs:"))
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
