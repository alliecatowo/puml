//! Coverage rescue lane F for preprocessor builtins and include handling.
//!
//! These tests intentionally drive the public parser/preprocessor APIs so they
//! cover `src/preproc/builtins.rs` and `src/preproc/includes.rs` without adding
//! CLI/theme/sprite overlap.
use httpmock::prelude::*;
use puml::ast::StatementKind;
use puml::parser::{parse_with_options, preprocess_with_options, ParseOptions};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;

fn msg_labels_with_options(src: &str, options: &ParseOptions) -> Vec<String> {
    let doc = parse_with_options(src, options).expect("parse failed");
    doc.statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::Message(message) => message.label.clone(),
            _ => None,
        })
        .collect()
}

fn msg_labels(src: &str) -> Vec<String> {
    msg_labels_with_options(src, &ParseOptions::default())
}

fn options_with_root(root: impl Into<PathBuf>) -> ParseOptions {
    ParseOptions {
        include_root: Some(root.into()),
        ..ParseOptions::default()
    }
}

fn url_cache_path(cache_home: &Path, url: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());
    cache_home.join("puml").join("includes").join(hash)
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn builtin_string_numeric_and_path_edges_are_deterministic() {
    let labels = msg_labels(
        r#"@startuml
!$substr = %substr("abcdef", -4, 2)
!$chr_bad = %strlen(%chr(1114112))
!$hex_bad = %hex2dec("not-hex")
!$quoted = %quote(hello)
!$rootless = %dirpath("leaf.puml")
A -> B : $substr
A -> B : $chr_bad
A -> B : $hex_bad
A -> B : $quoted
A -> B : %filename("/tmp/archive.tar.gz")
A -> B : %filenameroot("/tmp/archive.tar.gz")
A -> B : %strlen($rootless)
@enduml"#,
    );

    assert_eq!(
        labels,
        vec![
            "ab",
            "0",
            "0",
            "\"hello\"",
            "archive.tar.gz",
            "archive.tar",
            "0"
        ]
    );
}

#[test]
fn builtin_json_path_set_remove_and_merge_edges() {
    let labels = msg_labels(
        r#"@startuml
!$from_null = %set(null, "user.names[1]", "Ada")
!$created = %get_json_attribute($from_null, "user.names[1]")
!$bad_set = %set(["a"], "[oops]", "b")
!$bad_len = %list_size($bad_set)
!$nested = {"outer": {"items": ["a", "b"]}}
!$removed = %remove($nested, "outer.items[0]")
!$first_after_remove = %get_json_attribute($removed, "outer.items[0]")
!$merged_arrays = %map_merge(["a"], ["b", "c"])
!$array_size = %list_size($merged_arrays)
!$scalar_merge = %map_merge("old", "new")
A -> B : $created
A -> B : $bad_len
A -> B : $first_after_remove
A -> B : $array_size
A -> B : $scalar_merge
@enduml"#,
    );

    assert_eq!(labels, vec!["Ada", "1", "b", "3", "new"]);
}

#[test]
fn builtin_json_lookup_fallback_and_regex_class_edges() {
    let labels = msg_labels(
        r#"@startuml
!$loose = {"name": "Ada", "items": ["first", {"name": "second"}], "flag": true}
!$name = %get_json_attribute($loose, "items[1].name")
!$bad_index = %strlen(%get_json_attribute($loose, "items[notanumber]"))
!$contains = %map_contains_value($loose, "second")
!$neg_class = %splitstr_regex("a1b2c", "[^0-9]+")
!$digit_class = %splitstr_regex("a1b2c", "[0-9]")
!$optional = %splitstr_regex("abXac", "a.?c")
A -> B : $name
A -> B : $bad_index
A -> B : $contains
A -> B : $neg_class
A -> B : $digit_class
A -> B : $optional
@enduml"#,
    );

    assert_eq!(
        labels,
        vec!["second", "0", "true", ",1,2,", "a,b,c", "abX,"]
    );
}

#[test]
fn builtin_dynamic_invocation_error_branches_are_stable() {
    let empty = parse_with_options(
        "@startuml\nA -> B : %call_user_func(\"\")\n@enduml\n",
        &ParseOptions::default(),
    )
    .expect_err("empty callable should error");
    assert!(empty.message.contains("E_PREPROC_DYNAMIC_UNSUPPORTED"));
    assert!(empty.message.contains("non-empty callable name"));

    let procedure = parse_with_options(
        r#"@startuml
!procedure Say()
A -> B : hi
!endprocedure
A -> B : %call_user_func("Say")
@enduml"#,
        &ParseOptions::default(),
    )
    .expect_err("procedure expression invocation should error");
    assert!(procedure.message.contains("E_PREPROC_DYNAMIC_UNSUPPORTED"));
    assert!(procedure.message.contains("only supports functions"));
}

#[test]
fn include_resolves_nested_paths_relative_to_current_file() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("sub/nested")).unwrap();
    fs::write(root.join("sub/main.puml"), "!include nested/child.puml\n").unwrap();
    fs::write(
        root.join("sub/nested/child.puml"),
        "A -> B : nested-child\n",
    )
    .unwrap();

    let labels = msg_labels_with_options(
        "@startuml\n!include sub/main.puml\n@enduml\n",
        &options_with_root(root),
    );

    assert_eq!(labels, vec!["nested-child"]);
}

#[test]
fn include_many_glob_order_question_mark_and_no_match_are_deterministic() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("part_b.puml"), "A -> B : b\n").unwrap();
    fs::write(root.join("part_a.puml"), "A -> B : a\n").unwrap();
    fs::write(root.join("part_aa.puml"), "A -> B : aa\n").unwrap();
    fs::create_dir(root.join("part_c.puml")).unwrap();

    let expanded = preprocess_with_options(
        "@startuml\n!include_many part_?.puml\n!include_many missing_*.puml\n@enduml\n",
        &options_with_root(root),
    )
    .expect("glob include_many should preprocess");

    assert!(expanded.contains("A -> B : a\nA -> B : b"));
    assert!(!expanded.contains("aa"));
}

#[test]
fn include_many_glob_requires_root_and_reports_parent_errors() {
    let no_root = parse_with_options(
        "@startuml\n!include_many *.puml\n@enduml\n",
        &ParseOptions::default(),
    )
    .expect_err("stdin glob should require root");
    assert!(no_root.message.contains("E_INCLUDE_ROOT_REQUIRED"));

    let tmp = tempdir().unwrap();
    let missing_parent = parse_with_options(
        "@startuml\n!include_many missing/*.puml\n@enduml\n",
        &options_with_root(tmp.path()),
    )
    .expect_err("missing glob parent should error");
    assert!(missing_parent.message.contains("E_INCLUDE_READ"));
    assert!(missing_parent.message.contains("include glob parent"));
}

#[test]
fn include_angle_stdlib_tags_include_once_and_invalid_forms() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("stdlib/Local")).unwrap();
    fs::write(
        root.join("stdlib/Local/Tagged.puml"),
        "A -> B : whole\n!startsub PUBLIC\nA -> B : public\n!endsub\n",
    )
    .unwrap();

    let labels = msg_labels_with_options(
        "@startuml\n!include <Local/Tagged>!PUBLIC\n!include <Local/Tagged>!PUBLIC\n@enduml\n",
        &options_with_root(root),
    );
    assert_eq!(labels, vec!["public"]);

    let empty = parse_with_options(
        "@startuml\n!include <>\n@enduml\n",
        &options_with_root(root),
    )
    .expect_err("empty stdlib include should error");
    assert!(empty.message.contains("E_INCLUDE_PATH_REQUIRED"));

    let bad_suffix = parse_with_options(
        "@startuml\n!include <Local/Tagged> extra\n@enduml\n",
        &options_with_root(root),
    )
    .expect_err("bad suffix should error");
    assert!(bad_suffix.message.contains("E_INCLUDE_INVALID_FORM"));

    let missing_tag = parse_with_options(
        "@startuml\n!include <Local/Tagged>!MISSING\n@enduml\n",
        &options_with_root(root),
    )
    .expect_err("missing stdlib tag should error");
    assert!(missing_tag.message.contains("E_INCLUDE_TAG_NOT_FOUND"));
}

#[test]
fn include_and_import_security_diagnostics_are_stable() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("stdlib")).unwrap();
    fs::write(root.join("outside.puml"), "A -> B : outside\n").unwrap();

    let stdlib_escape = parse_with_options(
        "@startuml\n!include <../outside>\n@enduml\n",
        &options_with_root(root),
    )
    .expect_err("angle stdlib include should not escape stdlib root");
    assert!(stdlib_escape.message.contains("E_IMPORT_ESCAPE"));

    let import_escape = parse_with_options(
        "@startuml\n!import ../outside\n@enduml\n",
        &options_with_root(root),
    )
    .expect_err("import should not escape stdlib root");
    assert!(import_escape.message.contains("E_IMPORT_ESCAPE"));

    let invalid_root = tempdir().unwrap();
    let file_root = invalid_root.path().join("missing-root");
    let root_error = parse_with_options(
        "@startuml\n!include child.puml\n@enduml\n",
        &options_with_root(file_root),
    )
    .expect_err("file include_root should fail canonical root handling");
    assert!(root_error.message.contains("E_INCLUDE_ROOT_INVALID"));
}

#[test]
fn url_include_cache_is_used_before_network_and_file_url_errors_are_stable() {
    let _guard = env_lock().lock().unwrap();
    let tmp = tempdir().unwrap();
    let cache_home = tmp.path().join("cache-home");
    let server = MockServer::start();
    let network = server.mock(|when, then| {
        when.method(GET).path("/cached.puml");
        then.status(500).body("should not be fetched");
    });
    let url = server.url("/cached.puml");
    let cache_path = url_cache_path(&cache_home, &url);
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    fs::write(&cache_path, "A -> B : from-cache\n").unwrap();

    std::env::set_var("XDG_CACHE_HOME", &cache_home);
    let options = ParseOptions {
        allow_url_includes: true,
        ..Default::default()
    };
    let labels =
        msg_labels_with_options(&format!("@startuml\n!include {url}\n@enduml\n"), &options);
    std::env::remove_var("XDG_CACHE_HOME");

    assert_eq!(labels, vec!["from-cache"]);
    network.assert_calls(0);

    let file_url = "file:///definitely/not/a/puml-file.puml";
    let err = parse_with_options(
        &format!("@startuml\n!include {file_url}\n@enduml\n"),
        &options,
    )
    .expect_err("missing file URL should error deterministically");
    assert!(err.message.contains("E_INCLUDE_URL_FETCH"));
    assert!(err.message.contains("failed to read file URL"));
}

#[test]
fn url_include_content_length_limit_is_checked_before_body_read() {
    let _guard = env_lock().lock().unwrap();
    let server = MockServer::start();
    let body = "A".repeat(1024 * 1024 + 1);
    let mock = server.mock(|when, then| {
        when.method(GET).path("/too-large-by-header.puml");
        then.status(200)
            .header("content-length", "1048577")
            .body(body);
    });
    let url = server.url("/too-large-by-header.puml");
    let cache_home = tempdir().unwrap();
    let cache_path = url_cache_path(cache_home.path(), &url);
    let _ = fs::remove_file(&cache_path);

    std::env::set_var("XDG_CACHE_HOME", cache_home.path());
    let options = ParseOptions {
        allow_url_includes: true,
        ..Default::default()
    };
    let err = parse_with_options(
        &format!("@startuml\n!includeurl {url}\n@enduml\n"),
        &options,
    )
    .expect_err("oversized content-length should error");
    std::env::remove_var("XDG_CACHE_HOME");

    assert!(err.message.contains("E_INCLUDE_URL_TOO_LARGE"));
    assert!(err.message.contains("1048577 bytes"));
    mock.assert_calls(1);
}
