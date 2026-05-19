#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{ActivityStepKind, DiagramKind, StatementKind};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn define_substitution_skips_quoted_strings() {
        let doc = parse_with_options(
            "!define NAME Alice\nparticipant NAME\nnote over NAME: \"NAME\"\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::Participant(_)
        ));
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.target.as_deref(), Some("Alice"));
                assert_eq!(n.text, "\"NAME\"");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn pragma_directives_with_arguments_are_preserved_as_statements() {
        let doc = parse_with_options(
            "!pragma teoz true\nparticipant A\nparticipant B\nA -> B: hi\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 4);
        assert!(matches!(doc.statements[0].kind, StatementKind::Pragma(_)));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
    }

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

    #[test]
    fn conditional_if_elseif_else_selects_first_matching_branch() {
        let doc = parse_with_options(
            "!define FLAG 1\n!if FLAG == 1\nA -> B: first\n!elseif 1\nA -> B: second\n!else\nA -> B: third\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("first")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn ifdef_and_ifndef_follow_define_state() {
        let doc = parse_with_options(
            "!define ENABLED 1\n!ifdef ENABLED\nA -> B: yes\n!endif\n!ifndef ENABLED\nA -> B: no\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("yes")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn while_loops_execute_with_define_updates() {
        let doc = parse_with_options(
            "!define COUNT 2\n!while COUNT != 0\nA -> B: loop\n!if COUNT == 2\n!define COUNT 1\n!elseif COUNT == 1\n!define COUNT 0\n!endif\n!endwhile\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_function_and_procedure_blocks_are_accepted() {
        let doc = parse_with_options(
            "@startuml\n!function Echo($x)\n!return $x\n!endfunction\n!procedure Emit($x)\n!log $x\n!endprocedure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_variables_and_callable_args_are_applied() {
        let doc = parse_with_options(
            "@startuml\n!$from = Alice\n!$to ?= Bob\n!function F($x,$y=\"B\")\n!return $x + $y\n!endfunction\n!procedure P($a,$b=\"B\")\n$a -> $b: via-proc\n!endprocedure\n!P($from,$to)\n$from -> $to: %F(\"A\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_concat_signature_and_arg_errors_are_deterministic() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a##$b)\n!return $a ## $b\n!endfunction\nA -> B: %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }

        let missing = parse_with_options(
            "@startuml\n!function Need($a,$b)\n!return $a\n!endfunction\nA -> B: %Need(\"x\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(missing.message.contains("E_PREPROC_ARG_REQUIRED"));
    }

    #[test]
    fn preprocessor_assert_false_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!assert false : expected failure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT"));
    }

    #[test]
    fn preprocessor_assert_requires_non_empty_expression() {
        let err = parse_with_options(
            "@startuml\n!assert\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT_EXPR_REQUIRED"));
    }

    #[test]
    fn preprocessor_unknown_builtin_is_rejected_deterministically() {
        // Truly-unknown `%xyz(...)` invocations must surface a deterministic
        // diagnostic so that drift in PlantUML's builtin surface fails fast
        // instead of silently going through.
        let err = parse_with_options(
            "@startuml\n!assert %nosuchbuiltin() : no\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(
            err.message.contains("E_PREPROC_BUILTIN_UNSUPPORTED"),
            "expected E_PREPROC_BUILTIN_UNSUPPORTED, got: {}",
            err.message
        );
    }

    #[test]
    fn preprocessor_builtin_basics_expand_inline() {
        // strlen, upper/lower, substr, intval, boolval — these used to error
        // out via E_PREPROC_BUILTIN_UNSUPPORTED. They now expand inline.
        let doc = parse_with_options(
            "@startuml\nA -> B : %strlen(\"hello\")=%upper(\"ab\")/%substr(\"plantuml\", 0, 5)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("5=AB/plant"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_json_variable_round_trips_via_get_json_attribute() {
        // JSON variable assignment is now accepted; `%get_json_attribute`
        // reads a single top-level string value.
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"alpha\", \"v\": 2 }\nA -> B : %get_json_attribute($cfg, \"name\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("alpha")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_invoke_procedure_dynamically_dispatches_to_callable() {
        // `%invoke_procedure("$Say", ...)` resolves to a previously declared
        // `!procedure` and executes its body deterministically.
        let doc = parse_with_options(
            "@startuml\n!procedure $Say($who)\nA -> $who : hi\n!endprocedure\n%invoke_procedure(\"$Say\", \"Bob\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.to, "Bob");
                assert_eq!(m.label.as_deref(), Some("hi"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_call_user_func_supports_dynamic_function_invocation() {
        let doc = parse_with_options(
            "@startuml\n!function F($x,$y)\n!return $x + $y\n!endfunction\nA -> B : %call_user_func(\"F\", \"A\", \"B\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                // `+` is the string concatenation operator in PlantUML preprocessor (#582).
                // `!return $x + $y` with $x="A" and $y="B" should produce "AB".
                assert_eq!(m.label.as_deref(), Some("AB"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_unclosed_function_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!function Echo($x)\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FUNCTION_UNCLOSED"));
    }

    #[test]
    fn unknown_preprocessor_directive_errors_deterministically() {
        let err = parse_with_options("!totallynew thing\nA -> B\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_PREPROC_UNSUPPORTED"));
        assert!(err.message.contains("!totallynew"));
    }

    #[test]
    fn conditional_requires_balancing_and_order() {
        let err = parse_with_options("!endif\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNEXPECTED"));

        let err = parse_with_options("!if 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNCLOSED"));

        let err = parse_with_options(
            "!if 1\n!else\n!elseif 1\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_ORDER"));
    }

    #[test]
    fn preprocessor_parenthesized_logical_conditions_are_supported() {
        let doc = parse_with_options(
            "@startuml\n!if (1 && (0 || 1))\nA -> B : yes\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_conditions_support_nested_integer_arithmetic() {
        let doc = parse_with_options(
            "@startuml\n!if (2 + 3 * (4 - 1)) == 11\nA -> B : math\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("math")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_macro_concat_collapses_expanded_function_body_tokens() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a,$b)\n!return $a ## $b\n!endfunction\nA -> B : %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_function_like_define_and_collection_aliases_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!define EDGE(a,b,label) a -> b : %upper(label)\n!$items = %list(\"red\", %map(\"name\", \"blue\"))\n!$items = %list_set($items, 0, \"green\")\n!$cfg = %map(\"items\", $items)\n!assert not %map_is_empty($cfg) and %map_contains_value($cfg, \"blue\")\nEDGE(Alice, Bob, ok)\nA -> B : %eval_int(\"2 + 3 * 4\")/%get($cfg, \"items[1].name\")/%list_get(%get($cfg, \"items\"), 0)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["OK", "14/blue/green"]);
    }

    #[test]
    fn preprocessor_json_helpers_return_nested_objects_and_empty_keys() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"users\": [{ \"name\": \"Ada\", \"meta\": { \"team\": \"core\" }}], \"empty\": \"\" }\n!if %json_key_exists($cfg, \"empty\")\nA -> B : %get_json_attribute($cfg, \"users[0].meta\")\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("{\"team\":\"core\"}"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_list_map_helpers_and_modulo_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"Ada\", \"role\": \"core\" }\n!foreach $item in %split(\"red|blue\", \"|\")\nA -> B : $item\n!endfor\n!if 7 % 4 == 3\nA -> B : %get($cfg, \"name\")/%join([\"x\",\"y\"], \"-\")/%quote(ok)\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["red", "blue", "Ada/x-y/\"ok\""]);
    }

    #[test]
    fn preprocessor_foreach_binds_map_pairs_and_array_indices() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"Ada\", \"role\": \"core\" }\n!foreach $key, $value in $cfg\nA -> B : $key=$value\n!endfor\n!foreach $idx, $color in [\"red\",\"blue\"]\nA -> B : $idx:$color\n!endfor\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["name=Ada", "role=core", "0:red", "1:blue"]);
    }

    #[test]
    fn preprocessor_list_and_map_builtin_aliases_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!$list = %list_insert([\"a\",\"c\"], 1, \"b\")\n!$map = %map(\"name\", \"Ada\", \"role\", \"core\")\nA -> B : %join(%list_reverse($list), \"\")/%list_indexof($list, \"b\")/%first($list)/%last($list)\nA -> B : %json_type(%str2json($map))/%get_json_type($map)/%map_entries($map)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels[0], "cba/1/a/c");
        assert!(labels[1].starts_with("object/object/[[\"name\",\"Ada\"],[\"role\",\"core\"]]"));
    }

    #[test]
    fn while_requires_balancing() {
        let err = parse_with_options("!endwhile\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNEXPECTED"));

        let err = parse_with_options("!while 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNCLOSED"));
    }

    #[test]
    fn parses_multiline_title_and_legend_blocks() {
        let doc = parse_with_options(
            "title\nLine 1\nLine 2\nend title\nlegend\nAlpha\nBeta\nend legend\nA -> B\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Title(v) => assert_eq!(v, "Line 1\nLine 2"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Legend(v) => assert_eq!(v, "Alpha\nBeta"),
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(doc.statements[2].kind, StatementKind::Message(_)));
    }

    #[test]
    fn parses_multiline_note_block() {
        let doc = parse_with_options(
            "A -> B\nnote right of B\nline 1\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("B"));
                assert_eq!(n.text, "line 1\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_note_across_without_target() {
        let doc =
            parse_with_options("note across: shared context\n", &ParseOptions::default()).unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "across");
                assert!(n.target.is_none());
                assert_eq!(n.text, "shared context");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_note_with_inline_head_text() {
        let doc = parse_with_options(
            "note over A, B: summary\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A, B"));
                assert_eq!(n.text, "summary\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_aliases_as_note() {
        let doc = parse_with_options(
            "hnote over A: alias form\nrnote right of A: rounded alias\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "alias form");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "rounded alias");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_multiline_terminators() {
        let doc = parse_with_options(
            "hnote over A\nhex body\nendhnote\nrnote over B\nrect body\nendrnote\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.text, "hex body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.text, "rect body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_ref_with_inline_head_text() {
        let doc = parse_with_options(
            "ref over A, B: summary\nline 2\nend ref\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "ref");
                assert_eq!(g.label.as_deref(), Some("over A, B\nsummary\nline 2"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_arrow_syntax() {
        let err = parse_with_options("A -x B", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_ARROW_INVALID"));
    }

    #[test]
    fn parses_lifecycle_shortcut_suffixes() {
        let doc = parse_with_options("A -> B++: inc", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->@R++");
                assert_eq!(m.to, "B");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_expanded_slanted_arrow_tokens() {
        let doc = parse_with_options("A -/-> B\nB -\\\\->> A\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-/->"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-\\-->>"),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_filled_virtual_endpoint_side_from_message_context() {
        let doc = parse_with_options("[*] -> A\nA -> [*]\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                let from_virtual = m.from_virtual.expect("from virtual");
                assert_eq!(from_virtual.side, crate::ast::VirtualEndpointSide::Left);
                assert_eq!(from_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => {
                let to_virtual = m.to_virtual.expect("to virtual");
                assert_eq!(to_virtual.side, crate::ast::VirtualEndpointSide::Right);
                assert_eq!(to_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_queue_participant_and_separator() {
        let doc = parse_with_options(
            "queue Jobs as Q\n== Processing ==\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Participant(p) => {
                assert_eq!(p.name, "Jobs");
                assert_eq!(p.alias.as_deref(), Some("Q"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Separator(v) => assert_eq!(v.as_deref(), Some("Processing")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_typed_group_end_keyword() {
        let doc =
            parse_with_options("alt branch\nA -> B\nend alt\n", &ParseOptions::default()).unwrap();

        match &doc.statements[2].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "end");
                assert_eq!(g.label.as_deref(), Some("alt"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_class_bootstrap_declarations_and_relations() {
        let doc = parse_with_options(
            "class User\nclass Account as Acct\nUser --> Acct : owns\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ClassDecl(_)
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::ClassDecl(_)
        ));
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "User");
                assert_eq!(rel.to, "Acct");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("owns"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_object_and_usecase_bootstrap_kinds() {
        let object_doc =
            parse_with_options("object Order\nobject Customer\n", &ParseOptions::default())
                .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);

        let usecase_doc = parse_with_options(
            "usecase Authenticate\nusecase Authorize\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
    }

    #[test]
    fn parses_core_uml_broad_partial_declaration_forms() {
        let class_doc = parse_with_options(
            "interface Gateway\nabstract class Shape\nannotation Trace\nstruct Payload\nGateway -[#blue,dashed]-> Shape : adapts\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(class_doc.kind, DiagramKind::Class);
        match &class_doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Gateway");
                assert_eq!(decl.members[0].text, "<<interface>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &class_doc.statements[1].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Shape");
                assert_eq!(decl.members[0].text, "<<abstract class>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(
            class_doc.statements[4].kind,
            StatementKind::FamilyRelation(_)
        ));
        match &class_doc.statements[4].kind {
            StatementKind::FamilyRelation(rel) => assert_eq!(rel.arrow, "-->"),
            other => panic!("unexpected statement: {other:?}"),
        }

        let object_doc = parse_with_options(
            "map Settings {\n  theme => light\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);
        match &object_doc.statements[0].kind {
            StatementKind::ObjectDecl(decl) => {
                assert_eq!(decl.name, "Settings");
                assert_eq!(decl.members[0].text, "<<map>>");
                assert_eq!(decl.members[1].text, "theme => light");
            }
            other => panic!("unexpected statement: {other:?}"),
        }

        let usecase_doc = parse_with_options(
            "actor Customer as C\nusecase (Login) as UC1\nC ..> UC1 : <<include>>\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
        match &usecase_doc.statements[0].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Customer");
                assert_eq!(decl.alias.as_deref(), Some("C"));
                assert_eq!(decl.members[0].text, "<<actor>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[1].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Login");
                assert_eq!(decl.alias.as_deref(), Some("UC1"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.arrow, "..>");
                assert_eq!(rel.label.as_deref(), None);
                assert_eq!(rel.stereotype.as_deref(), Some("include"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_family_relations_with_tight_labels_quotes_and_cardinality() {
        let doc = parse_with_options(
            "class \"Order-Service\"\nclass \"Line-Item\"\nclass \"Price-List\"\n\"Order-Service\" \"1\" --> \"0..*\" \"Line-Item\": contains\nLine-Item --> \"Price-List\": priced by\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        match &doc.statements[3].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Order-Service");
                assert_eq!(rel.to, "Line-Item");
                assert_eq!(rel.label.as_deref(), Some("contains"));
                assert_eq!(rel.left_cardinality.as_deref(), Some("1"));
                assert_eq!(rel.right_cardinality.as_deref(), Some("0..*"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[4].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Line-Item");
                assert_eq!(rel.to, "Price-List");
                assert_eq!(rel.label.as_deref(), Some("priced by"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_component_namespace_groups_and_lollipop_endpoint_cleanup() {
        let doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface \"Orders\" as Orders\n}\nAPI --() Orders: provides\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Component);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ClassGroup { .. }
        ));
        match &doc.statements[1].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "API");
                assert_eq!(rel.to, "Orders");
                assert_eq!(rel.label.as_deref(), Some("provides"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_scoped_core_uml_relations_and_lollipop_endpoints() {
        let doc = parse_with_options(
            "@startuml\npackage Domain {\n  namespace Core {\n    class Api\n    class Repo\n    Api \"1\" -[#green,dashed]-> \"0..*\" Repo : owns\n  }\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::ClassGroup {
                members, relations, ..
            } => {
                assert!(members.iter().any(|m| m == "Domain::Core::Api"));
                assert_eq!(relations.len(), 1);
                assert_eq!(relations[0].from, "Domain::Core::Api");
                assert_eq!(relations[0].to, "Domain::Core::Repo");
                assert_eq!(relations[0].left_cardinality.as_deref(), Some("1"));
                assert_eq!(relations[0].right_cardinality.as_deref(), Some("0..*"));
                assert_eq!(relations[0].line_color.as_deref(), Some("#008000"));
                assert!(relations[0].dashed);
            }
            other => panic!("unexpected statement: {other:?}"),
        }

        let component_doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface Orders\n  API --() Orders : provides\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &component_doc.statements[0].kind {
            StatementKind::ClassGroup { relations, .. } => {
                assert_eq!(relations.len(), 1);
                assert_eq!(relations[0].from, "Edge::API");
                assert_eq!(relations[0].to, "Edge::Orders");
                assert!(relations[0].right_lollipop);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_sequence_decorated_arrow_styles_as_portable_arrow_core() {
        let doc = parse_with_options(
            "participant A\nparticipant B\nA -[#red,dashed]> B : styled\nB ->[#blue,dashed]> A : open styled\nA -[hidden]-> B : hidden\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        match &doc.statements[2].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->");
                assert_eq!(m.style.color.as_deref(), Some("red"));
                assert!(m.style.dashed);
                assert!(!m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[3].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->>");
                assert_eq!(m.style.color.as_deref(), Some("blue"));
                assert!(m.style.dashed);
                assert!(!m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[4].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "-->");
                assert!(m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_sequence_participants_in_theme_fixture_context() {
        let fixture = fs::read_to_string(format!(
            "{}/docs/examples/themes/07_no_theme_default.puml",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("theme fixture");

        let doc = parse_with_options(&fixture, &ParseOptions::default()).unwrap();

        assert_eq!(doc.kind, DiagramKind::Sequence);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[4].kind, StatementKind::Message(_)));
    }

    #[test]
    fn parses_activity_switch_split_goto_and_terminal_controls() {
        let doc = parse_with_options(
            "@startuml\nstart\nswitch (kind?)\ncase (A)\n:Do A;\ncase (B)\ngoto retry\nendswitch\nif (ready?) then (yes)\nelseif (warm?) then (maybe)\nendif\nrepeat\ncontinue\nbreak\nrepeat while (again?)\nend repeat\nsplit\n:one;\nsplit again\n:two;\nend split\nlabel retry\nbackward: retry path;\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        let steps = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::ActivityStep(step) => Some(step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::IfStart
                && step.label.as_deref() == Some("switch kind?")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Else && step.label.as_deref() == Some("A")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Fork
                && step.label.as_deref() == Some("split")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("goto retry")));
        assert!(steps.iter().any(|step| step.kind == ActivityStepKind::Else
            && step.label.as_deref() == Some("elseif warm? / maybe")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("continue")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("break")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("backward retry path")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Stop
                && step.label.as_deref() == Some("detach")));
    }

    #[test]
    fn parses_family_declaration_blocks_with_members() {
        let doc = parse_with_options(
            "class User {\n  +id: UUID\n  +name: String\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        match &doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "User");
                assert_eq!(decl.members.len(), 2);
                assert_eq!(decl.members[0].text, "+id: UUID");
                assert_eq!(decl.members[0].modifier, None);
                assert_eq!(decl.members[1].text, "+name: String");
                assert_eq!(decl.members[1].modifier, None);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn unclosed_family_declaration_block_reports_deterministic_error() {
        let err = parse_with_options(
            "object Config {\nkey = \"value\"\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FAMILY_DECL_BLOCK_UNCLOSED"));
    }

    #[test]
    fn parses_gantt_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\n[Build]\n[Milestone] happens on 2026-05-01\n[Build] starts 2026-04-01\n[Build] requires [Design]\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttTaskDecl { .. }
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttMilestoneDecl {
                happens_on: Some(_),
                ..
            }
        ));
        assert!(doc
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, StatementKind::GanttConstraint { .. })));
    }

    #[test]
    fn parses_gantt_dates_and_duration_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n[Build] lasts 5 days\n[Test] starts 2026-05-06 and lasts 2 weeks\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttConstraint {
                ref subject,
                ref kind,
                ref target
            } if subject == "Project" && kind == "starts" && target == "2026-05-01"
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                duration_days: Some(5),
                ..
            } if name == "Build"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                start_date: Some(ref d),
                duration_days: Some(14),
                ..
            } if name == "Test" && d == "2026-05-06"
        ));
    }

    #[test]
    fn parses_gantt_closed_weekday_calendar_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\nsaturday are closed\nsundays are closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "saturday"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "sunday"
        ));
    }

    #[test]
    fn parses_gantt_closed_date_range_calendar_statement() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n2026-05-04 to 2026-05-05 is closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosedDateRange {
                ref start_date,
                ref end_date
            } if start_date == "2026-05-04" && end_date == "2026-05-05"
        ));
    }

    #[test]
    fn parses_chronology_happens_on_baseline_statement() {
        let doc = parse_with_options(
            "@startchronology\nRelease happens on 2026-05-15\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Chronology);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ChronologyHappensOn { .. }
        ));
    }

    #[test]
    fn parses_usecase_relations_with_alias_and_label() {
        let doc = parse_with_options(
            "usecase Authenticate as Auth\nusecase User\nAuth --> User : validates\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::UseCase);
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Auth");
                assert_eq!(rel.to, "User");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("validates"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn malformed_family_relation_is_preserved_as_unknown_statement() {
        let doc = parse_with_options("class User\nUser -->\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(doc.statements[1].kind, StatementKind::Unknown(_)));
    }

    #[test]
    fn state_keyword_is_parsed_as_state_decl() {
        let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::State);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::StateDecl(_)
        ));
    }

    #[test]
    fn mixed_family_input_reports_deterministic_error() {
        let err = parse_with_options("class A\nnewpage\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_FAMILY_MIXED"));
    }

    #[test]
    fn start_enduml_markers_accept_optional_block_suffixes() {
        let doc = parse_with_options(
            "@startuml \"Primary\"\nA -> B: one\n@enduml anything\n@startuml Second\nB -> A: two\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        let labels = doc
            .statements
            .iter()
            .filter_map(|s| match &s.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["one", "two"]);
    }

    #[test]
    fn start_end_timeline_markers_accept_optional_block_suffixes() {
        let gantt = parse_with_options(
            "@startgantt \"Gantt\"\n[2026-01] : one\n@endgantt anything\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\nEvent\n@endchronology now\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn startmindmap_and_startwbs_markers_set_family_kind() {
        let mindmap = parse_with_options(
            "@startmindmap\n* Root\n** Child\n@endmindmap\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(mindmap.kind, DiagramKind::MindMap);

        let wbs =
            parse_with_options("@startwbs\n* Scope\n@endwbs\n", &ParseOptions::default()).unwrap();
        assert_eq!(wbs.kind, DiagramKind::Wbs);

        let gantt = parse_with_options(
            "@startgantt\n[2026-01-01] : Kickoff\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\n2026-01-01 : Event\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn parses_activity_oldstyle_baseline_statements() {
        let doc = parse_with_options(
            "@startuml\n|Build|\n(*) --> \"Init\"\n#gold:Compile;\n-->[next] right of \"Test\"\n\"Test\" --> (*)\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        assert!(!doc.statements.is_empty());
    }

    #[test]
    fn parses_old_activity_edges_as_canonical_steps() {
        let doc = parse_with_options(
            "@startuml\n(*) --> \"Step1\"\n\"Step1\" -->[ok] \"Step2\"\n\"Step2\" --> (*)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        let steps: Vec<_> = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::ActivityStep(step) => Some((step.kind.clone(), step.label.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            steps,
            vec![
                (ActivityStepKind::Start, None),
                (ActivityStepKind::Action, Some("Step1".to_string())),
                (ActivityStepKind::Action, Some("Step2".to_string())),
                (ActivityStepKind::Stop, None),
            ]
        );
    }

    #[test]
    fn mismatched_start_end_family_markers_report_deterministic_error() {
        let err = parse_with_options("@startmindmap\n* Root\n@endwbs\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_BLOCK_MISMATCH"));
    }

    #[test]
    fn apostrophe_comments_are_ignored_but_preserved_inside_quotes() {
        let doc = parse_with_options(
            "@startuml\n' full line comment\nA -> B: \"don't split\" ' trailing comment\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("\"don't split\""));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    /// Regression: `actor` + `alt` combination must not trigger component family
    /// misdetection (issue #776).  `actor` is a valid sequence participant role and
    /// should not cause the diagram to be classified as a component diagram when
    /// sequence-specific keywords (`alt`, `activate`, sequence arrows) appear.
    #[test]
    fn actor_alt_combination_is_sequence_not_component() {
        let src = "\
@startuml
actor User
participant Browser
participant Server
User -> Browser: click
Browser -> Server: request
activate Server
alt success
  Server --> Browser: 200 OK
else failure
  Server --> Browser: 500
end
deactivate Server
@enduml
";
        let doc = parse_with_options(src, &ParseOptions::default()).unwrap();
        assert_eq!(
            doc.kind,
            DiagramKind::Sequence,
            "actor+alt diagram must be detected as sequence, not component"
        );
        // Verify the actor participant is present
        assert!(
            doc.statements.iter().any(|s| matches!(
                &s.kind,
                StatementKind::Participant(p) if p.name == "User"
            )),
            "actor participant 'User' must be parsed"
        );
    }

    /// Regression: `par..also..end` must not trigger component family misdetection
    /// and `also` must be recognized as a valid parallel-branch continuation keyword
    /// for `par` groups (issue #780).
    #[test]
    fn par_also_end_is_valid_sequence_group() {
        let src = "\
@startuml
participant A
participant B
participant C
A -> B: start
par branch 1
  B -> C: query
  C --> B: result
also branch 2
  B -> C: notify
  C --> B: ack
end
@enduml
";
        let doc = parse_with_options(src, &ParseOptions::default()).unwrap();
        assert_eq!(
            doc.kind,
            DiagramKind::Sequence,
            "par..also..end diagram must be detected as sequence"
        );
        // Verify `also` parsed as a Group statement
        let also_stmt = doc.statements.iter().find(|s| {
            matches!(&s.kind, StatementKind::Group(g) if g.kind == "also")
        });
        assert!(
            also_stmt.is_some(),
            "`also` keyword must produce a Group statement"
        );
    }
}
