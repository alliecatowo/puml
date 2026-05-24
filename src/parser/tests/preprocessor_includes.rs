use super::*;

#[test]
fn include_resolves_relative_to_root() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();

    let doc = parse_with_options(
        "!include inc.puml",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
}

#[test]
fn include_cycle_errors() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.puml"), "!include b.puml\n").unwrap();
    fs::write(dir.path().join("b.puml"), "!include a.puml\n").unwrap();

    let err = parse_with_options(
        "!include a.puml",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("include cycle detected"));
}

#[test]
fn include_from_stdin_requires_root() {
    let err = parse_with_options("!include x.puml", &ParseOptions::default()).unwrap_err();
    assert!(err.message.contains("E_INCLUDE_ROOT_REQUIRED"));
}

#[test]
fn include_rejects_parent_escape_outside_root() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    let outside = dir.path().join("outside.puml");
    fs::create_dir_all(&root).unwrap();
    fs::write(&outside, "A -> B\n").unwrap();

    let err = parse_with_options(
        "!include ../outside.puml",
        &ParseOptions {
            include_root: Some(root),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDE_ESCAPE"));
}

#[cfg(unix)]
#[test]
fn include_rejects_symlink_target_outside_root() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    let outside = dir.path().join("outside.puml");
    let link = root.join("link_outside.puml");

    fs::create_dir_all(&root).unwrap();
    fs::write(&outside, "A -> B\n").unwrap();
    symlink(&outside, &link).unwrap();

    let err = parse_with_options(
        "!include link_outside.puml",
        &ParseOptions {
            include_root: Some(root),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDE_ESCAPE"));
}

#[test]
fn include_id_extracts_startsub_block() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("inc.puml"),
        "!startsub FLOW\nA -> B : one\n!endsub\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!include inc.puml!FLOW",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
}

#[test]
fn include_id_missing_tag_errors() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("inc.puml"),
        "!startsub FLOW\nA -> B : one\n!endsub\n",
    )
    .unwrap();

    let err = parse_with_options(
        "!include inc.puml!MISSING",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDE_TAG_NOT_FOUND"));
}

#[test]
fn include_url_disabled_errors() {
    let err = parse_with_options(
        "!include https://example.com/a.puml",
        &ParseOptions {
            allow_url_includes: false,
            ..ParseOptions::default()
        },
    )
    .unwrap_err();
    assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn import_resolves_stdlib_module_from_include_root() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("nested")).unwrap();
    fs::write(stdlib.join("core.puml"), "A -> B : core\n").unwrap();
    fs::write(
        stdlib.join("nested").join("extra.puml"),
        "B -> A : nested\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!import core\n!import nested/extra\n!import core\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn include_angle_bracket_targets_resolve_from_stdlib_catalog() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("C4")).unwrap();
    fs::write(
        stdlib.join("C4").join("C4_Container.puml"),
        "!procedure Container($alias,$label)\n$alias -> $alias : [C4] $label\n!endprocedure\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!include <C4/C4_Container>\nContainer(Api, \"API\")\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn include_angle_bracket_targets_support_startsub_tags() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("C4")).unwrap();
    fs::write(
        stdlib.join("C4").join("C4_Context.puml"),
        "!startsub CORE\nAlice -> Bob : tagged\n!endsub\nCharlie -> Dana : untagged\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!include <C4/C4_Context>!CORE\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 1);
    match &doc.statements[0].kind {
        StatementKind::Message(message) => {
            assert_eq!(message.from, "Alice");
            assert_eq!(message.to, "Bob");
            assert_eq!(message.label.as_deref(), Some("tagged"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn include_angle_bracket_missing_tag_errors_with_stdlib_context() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("C4")).unwrap();
    fs::write(
        stdlib.join("C4").join("C4_Context.puml"),
        "!startsub CORE\nAlice -> Bob : tagged\n!endsub\n",
    )
    .unwrap();

    let err = parse_with_options(
        "!include <C4/C4_Context>!MISSING\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDE_TAG_NOT_FOUND"));
    assert!(err.message.contains("stdlib include"));
    assert!(err.message.contains("MISSING"));
}

#[test]
fn import_and_include_catalog_support_aws_shape_stub_surface() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("awslib14").join("Compute")).unwrap();
    fs::write(
        stdlib.join("awslib14").join("AWSCommon.puml"),
        "!procedure AWSIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AWS $service] $label\n!endprocedure\n",
    )
    .unwrap();
    fs::write(
        stdlib.join("awslib14").join("Compute").join("EC2.puml"),
        "!include <awslib14/AWSCommon>\n!procedure EC2($alias,$label=\"\")\nAWSIcon($alias,EC2,$label)\n!endprocedure\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!import awslib14/AWSCommon\n!include <awslib14/Compute/EC2>\nEC2(NodeA, \"ingress\")\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn import_and_include_catalog_support_awslib_official_slug_alias() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(stdlib.join("awslib14").join("Compute")).unwrap();
    fs::write(
        stdlib.join("awslib14").join("AWSCommon.puml"),
        "!procedure AWSIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AWS $service] $label\n!endprocedure\n",
    )
    .unwrap();
    fs::write(
        stdlib.join("awslib14").join("Compute").join("EC2.puml"),
        "!include <awslib/AWSCommon>\n!procedure EC2($alias,$label=\"\")\nAWSIcon($alias,EC2,$label)\n!endprocedure\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!import awslib/AWSCommon\n!include <awslib/Compute/EC2>\nEC2(NodeA, \"ingress\")\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn import_and_include_catalog_support_azure_and_gcp_shape_stub_surface() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");

    fs::create_dir_all(stdlib.join("azure")).unwrap();
    fs::write(
        stdlib.join("azure").join("AzureCommon.puml"),
        "!procedure AzureIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AZURE $service] $label\n!endprocedure\n",
    )
    .unwrap();
    fs::write(
        stdlib.join("azure").join("StorageAccount.puml"),
        "!include <azure/AzureCommon>\n!procedure AzureStorageAccount($alias,$label=\"\")\nAzureIcon($alias,StorageAccount,$label)\n!endprocedure\n",
    )
    .unwrap();

    fs::create_dir_all(stdlib.join("gcp")).unwrap();
    fs::write(
        stdlib.join("gcp").join("GCPCommon.puml"),
        "!procedure GCPIcon($alias,$service,$label=\"\")\n$alias -> $alias : [GCP $service] $label\n!endprocedure\n",
    )
    .unwrap();
    fs::write(
        stdlib.join("gcp").join("ComputeEngine.puml"),
        "!include <gcp/GCPCommon>\n!procedure GCPComputeEngine($alias,$label=\"\")\nGCPIcon($alias,ComputeEngine,$label)\n!endprocedure\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!import azure/AzureCommon\n!include <azure/StorageAccount>\nAzureStorageAccount(AzStore, \"assets\")\n!import gcp/GCPCommon\n!include <gcp/ComputeEngine>\nGCPComputeEngine(GceNode, \"ingress\")\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn import_requires_stdlib_root_when_no_include_root_is_available() {
    let err = parse_with_options("!import core\n", &ParseOptions::default()).unwrap_err();
    assert!(err.message.contains("E_IMPORT_ROOT_REQUIRED"));
}

#[test]
fn import_security_and_shape_errors_are_deterministic() {
    let dir = tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(&stdlib).unwrap();
    fs::write(stdlib.join("ok.puml"), "A -> B\n").unwrap();

    let cases = [
        ("!import\n", "E_IMPORT_PATH_REQUIRED"),
        ("!import /tmp/abs.puml\n", "E_IMPORT_ABSOLUTE_PATH"),
        ("!import bad!TAG\n", "E_IMPORT_INVALID_FORM"),
        ("!import ../outside\n", "E_IMPORT_ESCAPE"),
        ("!import does/not/exist\n", "E_IMPORT_STDLIB_NOT_FOUND"),
    ];

    for (src, code) in cases {
        let err = parse_with_options(
            src,
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();
        assert!(
            err.message.contains(code),
            "missing {code}: {}",
            err.message
        );
    }
}

#[test]
fn import_url_disabled_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stdlib = dir.path().join("stdlib");
    fs::create_dir_all(&stdlib).unwrap();
    let err = parse_with_options(
        "!import https://example.com/lib.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            allow_url_includes: false,
            ..ParseOptions::default()
        },
    )
    .unwrap_err();
    assert!(
        err.message.contains("E_INCLUDE_URL_DISABLED"),
        "missing E_INCLUDE_URL_DISABLED: {}",
        err.message
    );
}

#[test]
fn include_once_only_expands_first_occurrence() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

    let doc = parse_with_options(
        "!include_once inc.puml\n!include_once inc.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn include_many_expands_each_occurrence() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B : many\n").unwrap();

    let doc = parse_with_options(
        "!include_many inc.puml\n!include_many inc.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn include_many_glob_expands_files_in_deterministic_order() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("b.puml"), "A -> B : b\n").unwrap();
    fs::write(dir.path().join("a.puml"), "A -> B : a\n").unwrap();
    fs::write(dir.path().join("ignore.txt"), "A -> B : txt\n").unwrap();
    fs::create_dir_all(dir.path().join("nested")).unwrap();
    fs::write(
        dir.path().join("nested").join("skip.puml"),
        "A -> B : nested\n",
    )
    .unwrap();

    let doc = parse_with_options(
        "!include_many *.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 2);
    match &doc.statements[0].kind {
        StatementKind::Message(msg) => assert_eq!(msg.label.as_deref(), Some("a")),
        other => panic!("unexpected first statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Message(msg) => assert_eq!(msg.label.as_deref(), Some("b")),
        other => panic!("unexpected second statement: {other:?}"),
    }
}

#[test]
fn include_many_glob_from_stdin_requires_root() {
    let err = parse_with_options("!include_many *.puml\n", &ParseOptions::default()).unwrap_err();
    assert!(err.message.contains("E_INCLUDE_ROOT_REQUIRED"));
}

#[test]
fn include_many_glob_missing_parent_reports_read_error() {
    let dir = tempdir().unwrap();
    let err = parse_with_options(
        "!include_many missing/*.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();
    assert!(err.message.contains("E_INCLUDE_READ"));
    assert!(err.message.contains("include glob parent"));
}

#[test]
fn include_many_glob_rejects_parent_escape() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(dir.path().join("outside.puml"), "A -> B : outside\n").unwrap();

    let err = parse_with_options(
        "!include_many ../*.puml\n",
        &ParseOptions {
            include_root: Some(root),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDE_ESCAPE"));
}

#[test]
fn include_once_deduplicates_canonical_path_aliases() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("nested")).unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

    let doc = parse_with_options(
        "!include_once ./inc.puml\n!include_once nested/../inc.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn includesub_requires_tag() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B : body\n").unwrap();

    let err = parse_with_options(
        "!includesub inc.puml\n",
        &ParseOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParseOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.message.contains("E_INCLUDESUB_TAG_REQUIRED"));
}

#[test]
fn include_many_url_disabled_errors() {
    let err = parse_with_options(
        "!include_many https://example.com/a.puml",
        &ParseOptions {
            allow_url_includes: false,
            ..ParseOptions::default()
        },
    )
    .unwrap_err();
    assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn include_url_directive_disabled_errors_deterministically() {
    let err = parse_with_options(
        "!includeurl https://example.com/a.puml",
        &ParseOptions {
            allow_url_includes: false,
            ..ParseOptions::default()
        },
    )
    .unwrap_err();
    assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
    assert!(err
        .message
        .contains("!includeurl URL includes are disabled"));
}
