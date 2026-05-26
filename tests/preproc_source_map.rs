use puml::parser::{parse_with_options, ParseOptions};
use std::fs;
use tempfile::tempdir;

#[test]
fn parser_diagnostic_inside_include_reports_included_file_origin() {
    let dir = tempdir().unwrap();
    let child = dir.path().join("broken.puml");
    fs::write(&child, "A -x B\n").unwrap();

    let err = parse_with_options(
        "!include broken.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    let origin = err.source.as_ref().expect("include origin");
    assert!(err.message.contains("E_ARROW_INVALID"), "{err:?}");
    assert!(origin.file.as_ref().unwrap().ends_with("broken.puml"));
    assert_eq!(origin.line, 1);
    assert_eq!(origin.column, 1);
    assert_eq!(origin.snippet, "A -x B");
    assert!(origin
        .include_stack
        .first()
        .unwrap()
        .ends_with("broken.puml"));
}

#[test]
fn preprocessor_diagnostic_inside_include_reports_included_file_origin() {
    let dir = tempdir().unwrap();
    let child = dir.path().join("unsupported.puml");
    fs::write(&child, "!return nope\n").unwrap();

    let err = parse_with_options(
        "!include unsupported.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    let origin = err.source.as_ref().expect("include origin");
    assert!(err.message.contains("E_PREPROC_UNSUPPORTED"), "{err:?}");
    assert!(origin.file.as_ref().unwrap().ends_with("unsupported.puml"));
    assert_eq!(origin.line, 1);
    assert_eq!(origin.column, 1);
    assert_eq!(origin.snippet, "!return nope");
}
