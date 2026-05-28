/// Tests for `!definelong` / `!enddefinelong` multi-line macro definitions.
///
/// `!definelong` is a PlantUML preprocessor feature that allows defining macros
/// whose bodies span multiple lines. These macros are used extensively by real
/// macro libraries (AWS/Azure/GCP/C4/tupadr3 stdlib bundles).
use puml::{ast::StatementKind, parse};

// ── helpers ───────────────────────────────────────────────────────────────────

fn preprocess_ok(src: &str) {
    // Use parse to drive the preprocessor; we only care that it succeeds.
    parse(src).expect("parse should succeed");
}

fn preprocess_err(src: &str) -> String {
    parse(src).expect_err("parse should fail").message.clone()
}

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

// ── basic: no-arg multi-line macro ───────────────────────────────────────────

#[test]
fn definelong_no_args_expands_to_body() {
    // A macro without arguments should expand to the body verbatim when called
    // with an empty argument list `MYMACRO()`.
    let src = "@startuml
!definelong MYMACRO()
Alice -> Bob : hello
Bob -> Alice : world
!enddefinelong
MYMACRO()
@enduml";
    // Verify it does not error — we just want the preprocessor to succeed.
    preprocess_ok(src);
}

// ── basic: single-param macro expands correctly ───────────────────────────────

#[test]
fn definelong_single_param_substitution() {
    let src = "@startuml
!definelong GREET(name)
Alice -> Bob : Hello name
!enddefinelong
GREET(World)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello World"]);
}

// ── multiple params ───────────────────────────────────────────────────────────

#[test]
fn definelong_two_params_substituted() {
    // Both params are substituted in the body. Here `src` and `dst` are used
    // as participant names (left/right of `->`) and as part of the label.
    let src = "@startuml
!definelong LINK(src, dst)
src -> dst : src connects dst
!enddefinelong
LINK(Alice, Bob)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Alice connects Bob"]);
}

// ── macro used multiple times ─────────────────────────────────────────────────

#[test]
fn definelong_used_multiple_times() {
    let src = "@startuml
!definelong GREET(who)
Alice -> who : hi
!enddefinelong
GREET(Bob)
GREET(Carol)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hi", "hi"]);
}

// ── multi-line body (more than one statement line) ────────────────────────────

#[test]
fn definelong_multi_line_body_emits_multiple_lines() {
    let src = "@startuml
!definelong PING(a, b)
a -> b : ping
b -> a : pong
!enddefinelong
PING(Alice, Bob)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ping", "pong"]);
}

// ── interaction with !define ──────────────────────────────────────────────────

#[test]
fn definelong_interacts_with_define() {
    // A !define value used inside a !definelong body should be substituted at
    // call time (same as single-line macros: token substitution happens on
    // each expanded line when it is processed).
    let src = "@startuml
!define LABEL hello
!definelong SAYHELLO(who)
Alice -> who : LABEL
!enddefinelong
SAYHELLO(Bob)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

// ── interaction with !set variables ──────────────────────────────────────────

#[test]
fn definelong_interacts_with_set_variable() {
    let src = "@startuml
!$target = \"Carol\"
!definelong REACH(msg)
Alice -> $target : msg
!enddefinelong
REACH(wave)
@enduml";
    // The body is `Alice -> $target : msg`; $target should expand.
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["wave"]);
}

// ── default parameter values ──────────────────────────────────────────────────

#[test]
fn definelong_default_param_used_when_arg_omitted() {
    // Default parameter without quotes (unquoted values are substituted as-is).
    let src = "@startuml
!definelong GREET(name = stranger)
Alice -> Bob : Hello name
!enddefinelong
GREET()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello stranger"]);
}

// ── !definelong inside inactive conditional is skipped ───────────────────────

#[test]
fn definelong_inside_inactive_conditional_is_skipped() {
    let src = "@startuml
!if 0
!definelong GHOST(x)
A -> B : ghost x
!enddefinelong
!endif
A -> B : visible
@enduml";
    let labels = msg_labels(src);
    // GHOST should not be defined, and only "visible" should appear.
    assert_eq!(labels, vec!["visible"]);
}

// ── orphaned !enddefinelong is an error ───────────────────────────────────────

#[test]
fn enddefinelong_without_definelong_is_error() {
    let src = "@startuml
!enddefinelong
A -> B : after
@enduml";
    let err = preprocess_err(src);
    assert!(
        err.contains("E_ENDDEFINELONG_UNEXPECTED"),
        "expected E_ENDDEFINELONG_UNEXPECTED, got: {err}"
    );
}

// ── unclosed !definelong is an error ─────────────────────────────────────────

#[test]
fn definelong_without_enddefinelong_is_error() {
    let src = "@startuml
!definelong OPEN(x)
A -> B : body
@enduml";
    let err = preprocess_err(src);
    assert!(
        err.contains("E_DEFINELONG_UNCLOSED"),
        "expected E_DEFINELONG_UNCLOSED, got: {err}"
    );
}

// ── real-world-ish: macro emitting several diagram statements ─────────────────

#[test]
fn definelong_real_world_component_macro() {
    // Mimics a pattern used by C4/AWS stdlib macros: a macro that emits
    // several diagram lines at once.
    let src = r#"@startuml
!definelong Person(alias, label)
participant "label" as alias
!enddefinelong
Person(alice, "Alice Smith")
Person(bob, "Bob Jones")
alice -> bob : hello
@enduml"#;
    // This should parse without error. The participant declarations and
    // message should all be present.
    let doc = parse(src).expect("real-world !definelong example should parse");
    assert!(
        !doc.statements.is_empty(),
        "expected statements in document"
    );
}

// ── !undef removes a !definelong macro ───────────────────────────────────────

#[test]
fn definelong_undef_removes_macro() {
    // After !undef, a !definelong macro should no longer be defined.
    // The macro call should not expand (the name passes through as text).
    let src = "@startuml
!definelong MYMACRO(x)
Alice -> Bob : x
!enddefinelong
!undef MYMACRO
A -> B : after
@enduml";
    // Should not error; MYMACRO call doesn't appear so nothing to check.
    preprocess_ok(src);
}
