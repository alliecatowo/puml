use puml::parser::{parse_with_options, ParseOptions};
use puml::{ast::StatementKind, parse};
/// Coverage wave 23 — exercises preprocessor includes, macros, and
/// expression-evaluation paths that were previously uncovered.
use std::fs;
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn msg_labels(src: &str) -> Vec<String> {
    let doc = parse(src).expect("parse failed");
    doc.statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect()
}

// ── expression evaluation (includes.rs paths) ─────────────────────────────────

#[test]
fn expr_compound_or_and() {
    let src = "@startuml
!$a = 1
!$b = 0
!if $a == 1 || $b == 1
A -> B : or-true
!endif
!if $a == 1 && $b == 0
A -> B : and-true
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["or-true", "and-true"]);
}

#[test]
fn expr_xor_word_operators() {
    // Test `xor` and `or` word operators
    let src = "@startuml
!$a = 1
!$b = 0
!if $a xor $b
A -> B : xor-true
!endif
!if $a or $b
A -> B : word-or-true
!endif
!if $a and $b
A -> B : word-and-false
!else
A -> B : word-and-else
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["xor-true", "word-or-true", "word-and-else"]);
}

#[test]
fn expr_not_prefix_and_negation() {
    let src = "@startuml
!if not 0
A -> B : not-zero-true
!endif
!if !0
A -> B : bang-zero-true
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["not-zero-true", "bang-zero-true"]);
}

#[test]
fn expr_comparison_operators() {
    let src = "@startuml
!if 3 > 2
A -> B : gt
!endif
!if 2 < 3
A -> B : lt
!endif
!if 3 >= 3
A -> B : gte
!endif
!if 3 <= 3
A -> B : lte
!endif
!if 2 != 3
A -> B : neq
!endif
!if 2 <> 3
A -> B : diamond-neq
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["gt", "lt", "gte", "lte", "neq", "diamond-neq"]);
}

#[test]
fn expr_defined_and_not_defined() {
    let src = "@startuml
!define FLAG
!if defined(FLAG)
A -> B : is-defined
!endif
!if !defined(NOTSET)
A -> B : not-defined
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["is-defined", "not-defined"]);
}

#[test]
fn expr_string_comparison_true_false_literals() {
    let src = "@startuml
!if \"hello\" == \"hello\"
A -> B : str-eq
!endif
!if \"hello\" != \"world\"
A -> B : str-neq
!endif
!if true
A -> B : literal-true
!endif
!if false
A -> B : literal-false
!else
A -> B : literal-false-else
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(
        labels,
        vec!["str-eq", "str-neq", "literal-true", "literal-false-else"]
    );
}

#[test]
fn expr_numeric_truthiness() {
    // non-zero integer is truthy, zero is falsy
    let src = "@startuml
!if 1
A -> B : one-truthy
!endif
!if 0
A -> B : zero-truthy
!else
A -> B : zero-falsy
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["one-truthy", "zero-falsy"]);
}

// ── include (fs) paths ────────────────────────────────────────────────────────

#[test]
fn include_basic_file() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let child = root.join("child.puml");
    fs::write(&child, "A -> B : from-child\n").unwrap();

    let src = "@startuml\n!include child.puml\n@enduml";
    let options = ParseOptions {
        include_root: Some(root),
        ..ParseOptions::default()
    };
    let labels = {
        let doc = parse_with_options(src, &options).expect("parse failed");
        doc.statements
            .iter()
            .filter_map(|s| match &s.kind {
                StatementKind::Message(m) => m.label.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(labels, vec!["from-child"]);
}

#[test]
fn include_once_deduplicates() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let once = root.join("once.puml");
    fs::write(&once, "A -> B : once-content\n").unwrap();

    let src = "@startuml\n!include_once once.puml\n!include_once once.puml\n@enduml";
    let options = ParseOptions {
        include_root: Some(root),
        ..ParseOptions::default()
    };
    let doc = parse_with_options(src, &options).expect("parse failed");
    let labels: Vec<_> = doc
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect();
    // include_once should only include the file once
    assert_eq!(labels, vec!["once-content"]);
}

#[test]
fn include_nested_with_depth() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let inner = root.join("inner.puml");
    let outer = root.join("outer.puml");
    fs::write(&inner, "A -> B : inner\n").unwrap();
    fs::write(&outer, "!include inner.puml\nA -> B : outer\n").unwrap();

    let src = "@startuml\n!include outer.puml\n@enduml";
    let options = ParseOptions {
        include_root: Some(root),
        ..ParseOptions::default()
    };
    let doc = parse_with_options(src, &options).expect("parse failed");
    let labels: Vec<_> = doc
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec!["inner", "outer"]);
}

#[test]
fn include_missing_file_gives_error() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();

    let src = "@startuml\n!include nonexistent.puml\n@enduml";
    let options = ParseOptions {
        include_root: Some(root),
        ..ParseOptions::default()
    };
    let err = parse_with_options(src, &options).expect_err("missing file should error");
    assert!(
        err.message.contains("E_INCLUDE")
            || err.message.contains("not found")
            || err.message.contains("No such"),
        "unexpected error: {}",
        err.message
    );
}

// ── macros: define and expand ─────────────────────────────────────────────────

#[test]
fn macro_define_and_expand_with_args() {
    let src = "@startuml
!define GREET(name) Hello name
A -> B : GREET(Alice)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello Alice"]);
}

#[test]
fn macro_define_no_args_simple_substitution() {
    let src = "@startuml
!define MYVAL 42
A -> B : MYVAL
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["42"]);
}

#[test]
fn macro_concat_operator() {
    // ## operator concatenates tokens
    let src = "@startuml
!function MkName($a, $b)
!return $a ## $b
!endfunction
A -> B : %MkName(Hello, World)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["HelloWorld"]);
}

#[test]
fn macro_with_default_param() {
    let src = "@startuml
!function Greet($name = \"World\")
!return \"Hello \" + $name
!endfunction
A -> B : %Greet(\"Alice\")
A -> B : %Greet()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello Alice", "Hello World"]);
}

#[test]
fn macro_recursive_function_depth() {
    // simple inline function that uses another function
    let src = "@startuml
!function Double($n)
!return %eval($n + $n)
!endfunction
!function Quad($n)
!return %Double(%Double($n))
!endfunction
A -> B : %Quad(3)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["12"]);
}

#[test]
fn macro_procedure_with_output() {
    let src = "@startuml
!procedure MsgPair($a, $b)
$a -> $b : forward
$b -> $a : reverse
!endprocedure
!MsgPair(Alice, Bob)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["forward", "reverse"]);
}

// ── macros: variable scoping ──────────────────────────────────────────────────

#[test]
fn macro_var_global_scope() {
    // !global $varname = val assigns a global variable
    let src = "@startuml
!global $g = globalval
!procedure ShowGlobal()
A -> B : $g
!endprocedure
!ShowGlobal()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["globalval"]);
}

// ── macros: split_args and parameter parsing ──────────────────────────────────

#[test]
fn macro_split_args_with_nested_parens() {
    // %substr has 3 args, the third uses a nested expression
    let src = "@startuml
!$s = %substr(\"hello world\", 0, %strlen(\"hello\"))
A -> B : $s
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

#[test]
fn macro_expand_function_returns_unknown_is_error() {
    // Calling unknown function gives a parse error
    let src = "@startuml\nA -> B : %__unknown_func__()\n@enduml";
    let err = parse(src).expect_err("unknown function should error");
    assert!(
        err.message.contains("unknown") || err.message.contains("E_PREPROC"),
        "unexpected error: {}",
        err.message
    );
}

// ── macros: eval_string_concat ────────────────────────────────────────────────

#[test]
fn string_concat_with_plus_operator() {
    let src = "@startuml
!$a = \"Hello\"
!$b = \" World\"
!$c = $a + $b
A -> B : $c
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello World"]);
}

// ── macros: json path suffix in variables ─────────────────────────────────────

#[test]
fn variable_json_dot_path_access() {
    let src = "@startuml
!$obj = {\"name\": \"Alice\", \"score\": 99}
A -> B : $obj.name
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Alice"]);
}

#[test]
fn variable_json_array_index_access() {
    let src = "@startuml
!$arr = [\"first\", \"second\", \"third\"]
A -> B : $arr[1]
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["second"]);
}

// ── preproc: assert directive ─────────────────────────────────────────────────

#[test]
fn assert_true_passes_through() {
    let src = "@startuml
!assert 1 == 1
A -> B : ok
@enduml";
    // Should parse without error
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ok"]);
}

#[test]
fn assert_false_gives_error() {
    let src = "@startuml\n!assert 1 == 2\n@enduml";
    let err = parse(src).expect_err("assert false should fail");
    assert!(
        err.message.contains("assert") || err.message.contains("E_PREPROC"),
        "unexpected error: {}",
        err.message
    );
}

// ── include_many ──────────────────────────────────────────────────────────────

#[test]
fn include_many_expands_glob_files() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let a = root.join("part_a.puml");
    let b = root.join("part_b.puml");
    fs::write(&a, "A -> B : from-a\n").unwrap();
    fs::write(&b, "A -> B : from-b\n").unwrap();

    let src = "@startuml\n!include_many part_*.puml\n@enduml";
    let options = ParseOptions {
        include_root: Some(root),
        ..ParseOptions::default()
    };
    let doc = parse_with_options(src, &options).expect("parse failed");
    let labels: Vec<_> = doc
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect();
    // Both files should be included (order may vary)
    assert!(labels.contains(&"from-a".to_string()));
    assert!(labels.contains(&"from-b".to_string()));
}
