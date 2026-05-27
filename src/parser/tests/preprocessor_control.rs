use super::*;

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
    let err =
        parse_with_options("!totallynew thing\nA -> B\n", &ParseOptions::default()).unwrap_err();
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
