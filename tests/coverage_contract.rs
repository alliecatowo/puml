use assert_cmd::Command;

#[test]
fn exit_code_contract() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .code(0);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--definitely-invalid-flag")
        .assert()
        .code(2);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-12345.puml")
        .assert()
        .code(3);

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("")
        .assert()
        .code(4);

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("invalid")
        .assert()
        .code(5);
}
