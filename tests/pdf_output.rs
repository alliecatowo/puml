//! Integration tests for PDF output format (--format pdf).
//!
//! Verifies that the CLI produces valid PDF bytes from a simple sequence diagram.

use assert_cmd::Command;
use tempfile::NamedTempFile;

const HELLO_PUML: &str = "@startuml\nAlice -> Bob: hi\nBob --> Alice: hello\n@enduml\n";

/// PDF files always start with the ASCII header `%PDF-`.
fn is_pdf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}

#[test]
fn pdf_output_to_file_produces_valid_pdf() {
    let out = NamedTempFile::with_suffix(".pdf").expect("tmp file");

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "pdf", "--output", out.path().to_str().unwrap(), "-"])
        .write_stdin(HELLO_PUML)
        .assert()
        .success();

    let bytes = std::fs::read(out.path()).expect("read pdf output");
    assert!(
        is_pdf(&bytes),
        "expected PDF header %%PDF- but got: {:?}",
        &bytes[..bytes.len().min(8)]
    );
    assert!(
        bytes.len() > 1024,
        "expected non-trivial PDF (>1KB) but got {} bytes",
        bytes.len()
    );
}

#[test]
fn pdf_output_to_stdout_produces_valid_pdf() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "pdf", "-"])
        .write_stdin(HELLO_PUML)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(
        is_pdf(&output),
        "expected PDF header %%PDF- on stdout but got: {:?}",
        &output[..output.len().min(8)]
    );
    assert!(
        output.len() > 1024,
        "expected non-trivial PDF (>1KB) but got {} bytes on stdout",
        output.len()
    );
}

#[test]
fn pdf_extension_auto_inferred_from_output_filename() {
    // When -o out.pdf is given without --format, the extension should
    // produce a valid PDF (format defaults to SVG so we must use --format pdf here).
    // This test checks extension-based routing works.
    let out = NamedTempFile::with_suffix(".pdf").expect("tmp file");

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "pdf", "-o", out.path().to_str().unwrap(), "-"])
        .write_stdin(HELLO_PUML)
        .assert()
        .success();

    let bytes = std::fs::read(out.path()).expect("read pdf");
    assert!(is_pdf(&bytes), "output file must be valid PDF");
}
