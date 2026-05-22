use std::fs;

use assert_cmd::Command;

fn puml() -> Command {
    Command::cargo_bin("puml").expect("puml binary")
}

#[test]
fn hash_defaults_to_fnv_hex_for_raw_bytes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("input.puml");
    fs::write(&fixture, b"hello\n").expect("write fixture");

    let out = puml()
        .args(["hash", fixture.to_str().expect("utf-8 path")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(stdout, "a9bc80cca21f28b3\n");
}

#[test]
fn hash_base64_encodes_digest_bytes_not_hex_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("input.puml");
    fs::write(&fixture, b"hello\n").expect("write fixture");

    let out = puml()
        .args([
            "hash",
            "--format",
            "base64",
            fixture.to_str().expect("utf-8 path"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(stdout, "qbyAzKIfKLM=\n");
}

#[test]
fn hash_rejects_unknown_algorithm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("input.puml");
    fs::write(&fixture, b"hello\n").expect("write fixture");

    puml()
        .args([
            "hash",
            "--algo",
            "sha256",
            fixture.to_str().expect("utf-8 path"),
        ])
        .assert()
        .failure();
}

#[test]
fn hash_missing_file_exits_with_code_2() {
    puml()
        .args(["hash", "does_not_exist_at_all.puml"])
        .assert()
        .code(2);
}
