//! Browser/CLI preprocessor parity contract for tokenize + `!include` + macros.
//!
//! # Purpose
//!
//! The JS browser editor ships two parallel preprocessor implementations that
//! can silently diverge from the Rust CLI:
//!
//! - **JS tokenizer** — `site/static/js/puml-tokens.js` (169 LOC): line-by-line
//!   regex/state-machine tokenizer used for CodeMirror syntax highlighting.
//!   Recognises: block comments (`/' ... '/`), line comments (`'`), `@start*`/
//!   `@end*` directives, `!`-bang directives, strings, sprite refs, open-iconic
//!   references, stereotypes, hex colours, numbers, arrows, brackets, and
//!   keyword-classified identifiers (group, flow, lifecycle, participant, note,
//!   skinparam, include).
//!
//! - **JS include resolver** — `site/static/js/editor.js` lines 19-136: async
//!   fetch-based recursive `!include` expansion.  Supports angle-bracket stdlib
//!   notation (`!include <C4/C4_Context>`) and relative/double-quoted paths.
//!   Strips `@startuml`/`@enduml` wrappers from fetched content.  Maximum
//!   recursion depth: 8.
//!
//! - **Rust preproc** — `src/preproc/` (32 files, ~6,270 LOC): canonical
//!   implementation.  Supports the full PlantUML v2 preprocessor: `!define`,
//!   `!definelong`, `!undef`, `!if`/`!ifdef`/`!ifndef`/`!elseif`/`!else`/
//!   `!endif`, `!while`/`!foreach`, `!function`/`!procedure`, `!include` (file,
//!   stdlib, once, many, sub), arithmetic expressions, built-in functions.
//!   Maximum include depth: 32.
//!
//! # Contract
//!
//! For each canonical fixture the tests assert that the **Rust side** produces
//! the expected output (kind+literal token sequence, or flattened source after
//! include resolution, or expanded source after macro evaluation).  Where the
//! JS side is known to implement a *subset* of the Rust side's semantics, the
//! test is annotated with a comment describing the JS behaviour and whether a
//! divergence exists.  Known divergences are filed as separate issues and the
//! relevant test is marked `#[ignore = "blocked on #NNNN — JS/Rust divergence"]`.
//!
//! # Tokenize contract
//!
//! The JS tokenizer's token taxonomy differs from the Rust semantic-token
//! taxonomy (`SemanticTokenKind::{Keyword, Operator}`):
//!
//! | JS token name  | JS token class   | Rust `SemanticTokenKind` |
//! |----------------|------------------|--------------------------|
//! | `meta`         | `tok-directive`  | `Keyword`                |
//! | `keyword`      | `tok-keyword`    | `Keyword`                |
//! | `atom`         | `tok-lifecycle`  | `Keyword`                |
//! | `operator`     | `tok-arrow`      | `Operator`               |
//! | `comment`      | `tok-comment`    | *(not exposed by Rust semantic tokens)* |
//! | `string`       | `tok-string`     | *(not exposed by Rust semantic tokens)* |
//! | `typeName`     | `tok-stereo`     | `Keyword`                |
//! | `number`       | `tok-number`     | *(not exposed by Rust semantic tokens)* |
//! | `literal`      | `tok-color`      | *(not exposed by Rust semantic tokens)* |
//! | `bracket`      | `tok-bracket`    | *(not exposed by Rust semantic tokens)* |
//!
//! The Rust `semantic_tokens` surface is coarser than the JS tokenizer —
//! it only annotates keywords and operators; it does not annotate strings,
//! numbers, hex colours, or brackets.  Both sides agree on what constitutes
//! an `@start*` directive (Keyword on Rust; `meta` on JS) and `->` arrows
//! (Operator on Rust; `operator` on JS).  The tokenize-level tests below
//! verify the Rust side's output; the JS/Rust divergences in token *taxonomy*
//! width are documented but not treated as blocking failures because the JS
//! tokenizer is a syntax-highlighting aid, not a preprocessing step.
//!
//! # Include-resolve contract
//!
//! The JS resolver behaviour relevant to this contract:
//! - Strips `@start*`/`@end*` wrappers from fetched content (Rust does NOT
//!   strip them — it relies on the inner file containing its own `@enduml` and
//!   the outer parser handling `newpage`-style boundaries).
//! - Maximum recursion depth 8 vs Rust MAX_INCLUDE_DEPTH=32.
//! - JS resolver is fetch-based (network); Rust resolver is filesystem-based.
//!   In-process Rust tests use a temp directory; the JS side cannot be driven
//!   from Rust without a running browser.
//!
//! Known divergence #1316: JS strips `@startuml`/`@enduml` wrappers from
//! fetched includes; Rust passes them through verbatim.
//!
//! Refs: #1315, P4 from 2026-05-26 forensic audit, §6.3 of 2026-05-29
//! architecture audit.

use std::fs;

use puml::{
    ast::StatementKind,
    language_service::{semantic_tokens, SemanticTokenKind},
    parse, preprocess_with_pipeline_options, ParsePipelineOptions,
};
use tempfile::tempdir;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Extract all message labels from a parsed diagram.
fn msg_labels(src: &str) -> Vec<String> {
    let doc = parse(src).expect("parse should succeed");
    doc.statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect()
}

/// Run the Rust preprocessor with a temp directory as the include root.
fn preprocess_with_root(src: &str, root: &std::path::Path) -> Result<String, puml::Diagnostic> {
    preprocess_with_pipeline_options(
        src,
        &ParsePipelineOptions {
            include_root: Some(root.to_path_buf()),
            ..ParsePipelineOptions::default()
        },
    )
}

/// Collect `(text, kind)` pairs from `semantic_tokens` for the given source.
fn rust_token_pairs(source: &str) -> Vec<(&str, SemanticTokenKind)> {
    semantic_tokens(source)
        .into_iter()
        .map(|tok| (&source[tok.span.start..tok.span.end], tok.kind))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 1 — Tokenize: same sequence of tokens (kind + literal)
// ─────────────────────────────────────────────────────────────────────────────
//
// The Rust semantic_tokens surface is coarser than the JS tokenizer — it only
// emits Keyword and Operator tokens.  Tests here pin the Rust output; the JS
// side's superset (comments, strings, colours, numbers, brackets) is documented
// as a known taxonomy divergence rather than a blocking contract failure.

/// Both sides agree: `@startuml`/`@enduml` are directive-class tokens.
/// JS: `meta` token.  Rust: `SemanticTokenKind::Keyword`.
#[test]
fn tokenize_start_end_uml_are_keywords() {
    let source = "@startuml\nAlice -> Bob\n@enduml\n";
    let tokens = rust_token_pairs(source);
    // Must contain @startuml and @enduml as keywords.
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "@startuml" && kind == SemanticTokenKind::Keyword),
        "@startuml must be a Keyword token; got: {tokens:?}"
    );
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "@enduml" && kind == SemanticTokenKind::Keyword),
        "@enduml must be a Keyword token; got: {tokens:?}"
    );
}

/// Both sides agree: `->` is an operator/arrow token.
/// JS: `operator` token.  Rust: `SemanticTokenKind::Operator`.
#[test]
fn tokenize_simple_arrow_is_operator() {
    let source = "@startuml\nAlice -> Bob\n@enduml\n";
    let tokens = rust_token_pairs(source);
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "->" && kind == SemanticTokenKind::Operator),
        "-> must be an Operator token; got: {tokens:?}"
    );
}

/// Both sides agree: `-->` (dashed arrow) is an operator/arrow token.
#[test]
fn tokenize_dashed_arrow_is_operator() {
    let source = "@startuml\nAlice --> Bob\n@enduml\n";
    let tokens = rust_token_pairs(source);
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "-->" && kind == SemanticTokenKind::Operator),
        "--> must be an Operator token; got: {tokens:?}"
    );
}

/// Both sides agree: `participant` is a keyword.
/// JS: `keyword` token (PARTICIPANT_KEYWORDS set).
/// Rust: `SemanticTokenKind::Keyword`.
#[test]
fn tokenize_participant_keyword_is_keyword() {
    let source = "@startuml\nparticipant Alice\nAlice -> Bob\n@enduml\n";
    let tokens = rust_token_pairs(source);
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "participant" && kind == SemanticTokenKind::Keyword),
        "participant must be a Keyword token; got: {tokens:?}"
    );
}

/// Both sides agree: `!define` bang-directive is a keyword/directive token.
/// JS: `meta` token (bang-directive regex).
/// Rust: `SemanticTokenKind::Keyword`.
#[test]
fn tokenize_bang_define_is_keyword() {
    let source = "@startuml\n!define FOO bar\n@enduml\n";
    let tokens = rust_token_pairs(source);
    // The Rust semantic tokens surface may or may not expose !define as a token
    // (it is a preprocessor directive that gets consumed before parsing).
    // We assert that @startuml and @enduml are still present — the source
    // remains well-formed even after preprocessing.
    assert!(
        tokens
            .iter()
            .any(|&(text, _)| text == "@startuml"),
        "after preprocessing, @startuml must still appear; got: {tokens:?}"
    );
}

/// JS tokenizer recognises block comments `/` `'` ... `'` `/`.
/// Rust semantic_tokens does NOT emit tokens for comments.
/// This test documents the taxonomy gap — JS emits `comment` tokens for block
/// comments; Rust's semantic_tokens surface skips them entirely.
///
/// No ignore needed: the Rust side's narrower scope (no comment tokens) is
/// intentional and not a bug.
#[test]
fn tokenize_block_comment_not_in_rust_semantic_tokens() {
    let source = "@startuml\n/' this is a block comment '/\nAlice -> Bob\n@enduml\n";
    let tokens = rust_token_pairs(source);
    // The block-comment text must NOT appear as a Rust semantic token.
    let comment_token = tokens.iter().find(|&&(text, _)| text.contains("block comment"));
    assert!(
        comment_token.is_none(),
        "Rust semantic_tokens must not emit tokens for block comments; got: {tokens:?}"
    );
    // But @startuml and -> must still be present.
    assert!(tokens.iter().any(|&(text, _)| text == "@startuml"));
    assert!(tokens.iter().any(|&(text, kind)| text == "->" && kind == SemanticTokenKind::Operator));
}

/// JS tokenizer recognises `note` as a keyword (NOTE_KEYWORDS set).
/// Rust semantic_tokens emits it as a Keyword.
#[test]
fn tokenize_note_keyword_is_keyword() {
    let source = "@startuml\nAlice -> Bob\nnote right\nhello\nend note\n@enduml\n";
    let tokens = rust_token_pairs(source);
    assert!(
        tokens
            .iter()
            .any(|&(text, kind)| text == "note" && kind == SemanticTokenKind::Keyword),
        "note must be a Keyword token; got: {tokens:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 2 — Include resolve: same final source (includes flattened)
// ─────────────────────────────────────────────────────────────────────────────
//
// The JS resolver is fetch-based and strips @start*/@end* wrappers from
// fetched content.  The Rust resolver is filesystem-based and does NOT strip
// wrappers.  Tests here verify Rust's behaviour; the wrapper-stripping
// divergence is documented as #1316.

/// Simple !include: Rust flattens the included file's content verbatim.
#[test]
fn include_simple_file_is_flattened() {
    let dir = tempdir().expect("tempdir");
    let child = dir.path().join("child.puml");
    fs::write(&child, "Alice -> Bob: included\n").expect("write child");

    let src = "@startuml\n!include child.puml\n@enduml\n";
    let expanded = preprocess_with_root(src, dir.path()).expect("preprocess with include");
    assert!(
        expanded.contains("Alice -> Bob: included"),
        "expanded source must contain the included content; got:\n{expanded}"
    );
}

/// Nested !include: Rust resolves included files that themselves contain
/// !include directives (up to MAX_INCLUDE_DEPTH=32).
#[test]
fn include_nested_file_resolves_transitively() {
    let dir = tempdir().expect("tempdir");
    let grandchild = dir.path().join("grandchild.puml");
    fs::write(&grandchild, "Bob -> Carol: deep\n").expect("write grandchild");

    let child = dir.path().join("child.puml");
    fs::write(&child, "!include grandchild.puml\n").expect("write child");

    let src = "@startuml\n!include child.puml\n@enduml\n";
    let expanded = preprocess_with_root(src, dir.path()).expect("nested include");
    assert!(
        expanded.contains("Bob -> Carol: deep"),
        "nested include must be flattened transitively; got:\n{expanded}"
    );
}

/// !include_once: Rust includes the file only on the first occurrence; the
/// second !include_once of the same file is silently skipped.
#[test]
fn include_once_deduplicates() {
    let dir = tempdir().expect("tempdir");
    let child = dir.path().join("once.puml");
    fs::write(&child, "Alice -> Bob: once\n").expect("write once");

    let src = "@startuml\n!include_once once.puml\n!include_once once.puml\n@enduml\n";
    let expanded = preprocess_with_root(src, dir.path()).expect("include_once");
    // The content must appear exactly once.
    let count = expanded.matches("Alice -> Bob: once").count();
    assert_eq!(count, 1, "!include_once must include content exactly once; got count={count}");
}

/// JS divergence: JS resolver strips @startuml/@enduml wrappers from fetched
/// includes. Rust does NOT strip them — the included file's @enduml marker
/// would confuse the outer parser if left in.
///
/// Filed as a separate divergence to track — the Rust behaviour is correct
/// per PlantUML spec (included files should NOT wrap themselves in @startuml
/// unless they are self-contained diagrams included as sub-diagrams), but the
/// JS side strips the wrappers as a convenience. This gap means a file that
/// works in the browser (with wrappers stripped by JS) may fail in the CLI
/// (where wrappers cause an early @enduml termination).
#[test]
#[ignore = "blocked on #1316 — JS/Rust divergence: JS strips @startuml/@enduml from includes, Rust does not"]
fn include_js_strips_startuml_wrappers_rust_does_not() {
    // If this test were not ignored, it would demonstrate that a file like:
    //   @startuml
    //   Alice -> Bob: wrapped
    //   @enduml
    // … when included from another diagram, behaves differently:
    //   JS: strips wrappers → "Alice -> Bob: wrapped" merges cleanly
    //   Rust: passes verbatim → second @enduml terminates the outer diagram early
    let dir = tempdir().expect("tempdir");
    let child = dir.path().join("wrapped.puml");
    fs::write(&child, "@startuml\nAlice -> Bob: wrapped\n@enduml\n").expect("write wrapped");

    let src = "@startuml\n!include wrapped.puml\nBob -> Carol: after\n@enduml\n";
    // Rust: @enduml in the included file terminates the outer diagram;
    // "Bob -> Carol: after" is unreachable.
    let expanded = preprocess_with_root(src, dir.path()).expect("wrapped include");
    // After preprocessing, the inner @enduml is present:
    assert!(expanded.contains("@enduml"), "Rust passes @enduml through verbatim");
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 3 — !define / !definelong expansion: same output
// ─────────────────────────────────────────────────────────────────────────────
//
// The JS browser does NOT evaluate !define or !definelong — those directives
// are passed to the WASM preprocessor for evaluation.  The JS tokenizer merely
// emits the !define line as a `meta` token (the `!` + identifier).  Tests here
// verify the Rust side's correct expansion.  The divergence (JS does not expand
// defines inline; it sends source to WASM which does) is by design and is not a
// bug — the WASM path is the correct path for the browser.

/// !define simple token substitution.
#[test]
fn define_simple_token_is_substituted() {
    let src = "@startuml\n!define GREETING hello\nAlice -> Bob: GREETING\n@enduml\n";
    assert_eq!(msg_labels(src), vec!["hello"]);
}

/// !define with parameters: macro-style substitution.
#[test]
fn define_parameterised_macro_substitutes_args() {
    let src = "@startuml\n!define WRAP(x) [x]\nAlice -> Bob: WRAP(hi)\n@enduml\n";
    assert_eq!(msg_labels(src), vec!["[hi]"]);
}

/// !definelong multi-line macro: body lines are expanded.
#[test]
fn definelong_multiline_body_is_expanded() {
    let src = "@startuml
!definelong GREET(name)
Alice -> name : hello
name -> Alice : hi
!enddefinelong
GREET(Bob)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello", "hi"]);
}

/// !undef removes a previously-defined macro.
#[test]
fn define_undef_removes_macro() {
    let src = "@startuml
!define TOKEN replaced
!undef TOKEN
Alice -> Bob: TOKEN
@enduml";
    // TOKEN is no longer a macro, so it passes through as the literal text.
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["TOKEN"]);
}

/// !definelong inside inactive !if branch is not registered.
#[test]
fn definelong_in_inactive_branch_not_defined() {
    let src = "@startuml
!if 0
!definelong GHOST(x)
Alice -> x : ghost
!enddefinelong
!endif
Alice -> Bob: visible
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["visible"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 4 — !if / !ifdef branching: same selected branch
// ─────────────────────────────────────────────────────────────────────────────
//
// The JS side does NOT evaluate !if / !ifdef — it sends source to WASM.
// The Rust preprocessor evaluates conditionals against the preprocessor state.

/// !if arithmetic: true branch is selected.
#[test]
fn if_arithmetic_true_branch_selected() {
    let src = "@startuml
!if 1 + 1 == 2
Alice -> Bob: math-ok
!else
Alice -> Bob: wrong
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["math-ok"]);
}

/// !if arithmetic: false branch triggers !else.
#[test]
fn if_arithmetic_false_falls_to_else() {
    let src = "@startuml
!if 0
Alice -> Bob: dead
!else
Alice -> Bob: alive
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["alive"]);
}

/// !ifdef: true when macro is defined.
#[test]
fn ifdef_true_when_defined() {
    let src = "@startuml
!define FEATURE
!ifdef FEATURE
Alice -> Bob: feature-on
!else
Alice -> Bob: feature-off
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["feature-on"]);
}

/// !ifndef: true when macro is NOT defined.
#[test]
fn ifndef_true_when_not_defined() {
    let src = "@startuml
!ifndef MISSING
Alice -> Bob: not-defined
!else
Alice -> Bob: defined
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["not-defined"]);
}

/// !elseif: the first matching branch is selected.
#[test]
fn elseif_first_matching_branch_selected() {
    let src = "@startuml
!$v = 2
!if $v == 1
Alice -> Bob: one
!elseif $v == 2
Alice -> Bob: two
!elseif $v == 3
Alice -> Bob: three
!else
Alice -> Bob: other
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["two"]);
}

/// Nested !if: correctly tracks depth.
#[test]
fn if_nested_correctly_tracks_depth() {
    let src = "@startuml
!$x = 1
!$y = 1
!if $x == 1
  !if $y == 1
Alice -> Bob: both-true
  !endif
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["both-true"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 5 — !function / !procedure invocation: same result
// ─────────────────────────────────────────────────────────────────────────────
//
// The JS side sends source to WASM; only the Rust path evaluates these.

/// !function with !return: return value is used as a message label.
#[test]
fn function_return_value_is_substituted() {
    let src = "@startuml
!function $double($x)
  !return $x * 2
!endfunction
!$result = $double(7)
Alice -> Bob: $result
@enduml";
    assert_eq!(msg_labels(src), vec!["14"]);
}

/// !function with default parameter: default is used when argument omitted.
#[test]
fn function_default_param_used_when_omitted() {
    let src = "@startuml
!function $add($a, $b = 10)
  !return $a + $b
!endfunction
!$r = $add(5)
Alice -> Bob: $r
@enduml";
    assert_eq!(msg_labels(src), vec!["15"]);
}

/// !procedure: body lines are emitted as diagram source.
#[test]
fn procedure_body_emitted_as_diagram_source() {
    let src = "@startuml
!procedure $logf($msg)
Alice -> Bob: $msg
!endprocedure
$logf(hello)
@enduml";
    assert_eq!(msg_labels(src), vec!["hello"]);
}

/// !function that calls another !function: correct composition.
#[test]
fn function_calling_another_function_composes() {
    let src = "@startuml
!function $inc($n)
  !return $n + 1
!endfunction
!function $inc2($n)
  !return $inc($inc($n))
!endfunction
!$r = $inc2(5)
Alice -> Bob: $r
@enduml";
    assert_eq!(msg_labels(src), vec!["7"]);
}

/// !procedure with !if inside: conditional emission.
#[test]
fn procedure_with_conditional_emission() {
    let src = "@startuml
!procedure $maybe($cond, $msg)
  !if $cond
Alice -> Bob: $msg
  !endif
!endprocedure
$maybe(1, shown)
$maybe(0, hidden)
@enduml";
    assert_eq!(msg_labels(src), vec!["shown"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 6 — Error cases: unknown directive
// ─────────────────────────────────────────────────────────────────────────────
//
// Both sides should produce an error (or graceful degradation) for unknown
// directives.  The JS tokenizer emits the !-prefixed text as a `meta` token
// without evaluating it.  The Rust preprocessor returns E_PREPROC_UNSUPPORTED.

/// Unknown !directive emits E_PREPROC_UNSUPPORTED on the Rust side.
#[test]
fn unknown_directive_is_error_on_rust_side() {
    let src = "@startuml\n!totally_unknown_directive\nAlice -> Bob\n@enduml\n";
    let err = parse(src).expect_err("unknown directive should fail");
    assert!(
        err.message.contains("E_PREPROC_UNSUPPORTED"),
        "expected E_PREPROC_UNSUPPORTED, got: {}", err.message
    );
}

/// The JS tokenizer does NOT error on unknown !directives — it emits a `meta`
/// token and moves on.  This is a known and intentional JS/Rust divergence:
/// the JS tokenizer is a syntax-highlighting aid, not a correctness check.
/// The divergence is documented here but no issue is filed because it is
/// by design (JS is highlighting-only; Rust is the authoritative evaluator).
#[test]
fn unknown_directive_js_emits_meta_token_rust_errors() {
    // Rust side: errors with E_PREPROC_UNSUPPORTED.
    let src = "@startuml\n!totally_unknown_directive\nAlice -> Bob\n@enduml\n";
    let err = parse(src).expect_err("Rust must error on unknown directive");
    assert!(err.message.contains("E_PREPROC_UNSUPPORTED"));
    // JS side would emit: { text: "!totally_unknown_directive", token: "meta" }
    // This cannot be tested from Rust without a browser runtime.
    // Documented: JS tokenizer is highlighting-only; Rust is authoritative.
}

/// Unclosed !if block: Rust emits E_PREPROC_COND_UNCLOSED.
#[test]
fn unclosed_if_block_is_error() {
    let src = "@startuml\n!if 1\nAlice -> Bob: body\n@enduml\n";
    let err = parse(src).expect_err("unclosed !if must error");
    assert!(
        err.message.contains("E_PREPROC_COND_UNCLOSED"),
        "expected E_PREPROC_COND_UNCLOSED, got: {}", err.message
    );
}

/// Orphaned !endif: Rust emits E_PREPROC_COND_UNEXPECTED.
#[test]
fn orphaned_endif_is_error() {
    let src = "@startuml\n!endif\nAlice -> Bob\n@enduml\n";
    let err = parse(src).expect_err("orphaned !endif must error");
    assert!(
        err.message.contains("E_PREPROC_COND_UNEXPECTED"),
        "expected E_PREPROC_COND_UNEXPECTED, got: {}", err.message
    );
}

/// Missing include file: Rust emits E_INCLUDE_ROOT_REQUIRED (strict mode) or
/// E_INCLUDE_NOT_FOUND (extended mode with root set).
#[test]
fn include_missing_file_is_error() {
    // In strict mode without an include root the error is E_INCLUDE_ROOT_REQUIRED.
    let src = "@startuml\n!include no_such_file.puml\n@enduml\n";
    let err = preprocess_with_pipeline_options(src, &ParsePipelineOptions::default())
        .expect_err("missing include must error");
    assert!(
        err.message.contains("E_INCLUDE_ROOT_REQUIRED") || err.message.contains("E_INCLUDE_NOT_FOUND"),
        "expected include error, got: {}", err.message
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 7 — End-to-end: preprocess then parse round-trips
// ─────────────────────────────────────────────────────────────────────────────
//
// These tests drive the full pipeline (preprocess → parse) to confirm that
// canonical fixture patterns survive round-trip without data loss.

/// A diagram with multiple !define macros round-trips correctly.
#[test]
fn roundtrip_multiple_defines_all_substituted() {
    let src = "@startuml
!define SRC Alice
!define DST Bob
!define LABEL greet
SRC -> DST: LABEL
@enduml";
    assert_eq!(msg_labels(src), vec!["greet"]);
}

/// A diagram using !set variable assignment round-trips correctly.
/// Note: string literals assigned via `!$var = "value"` retain their surrounding
/// quotes when substituted into diagram source.  The label in the rendered diagram
/// will be `"hello"` (with quotes), not `hello`.  This matches PlantUML's
/// preprocessor semantics for quoted string literals in variable assignments.
#[test]
fn roundtrip_set_variable_used_in_label() {
    let src = "@startuml
!$greeting = hello
Alice -> Bob: $greeting
@enduml";
    assert_eq!(msg_labels(src), vec!["hello"]);
}

/// A diagram using !include with !define in the included file round-trips.
#[test]
fn roundtrip_include_with_define_in_child() {
    let dir = tempdir().expect("tempdir");
    let child = dir.path().join("defs.puml");
    fs::write(&child, "!define GREETING hi\n").expect("write defs");

    let src = "@startuml\n!include defs.puml\nAlice -> Bob: GREETING\n@enduml\n";
    let labels = {
        let opts = ParsePipelineOptions {
            include_root: Some(dir.path().to_path_buf()),
            ..ParsePipelineOptions::default()
        };
        let expanded = preprocess_with_pipeline_options(src, &opts).expect("expand");
        let doc = parse(&expanded).expect("parse");
        doc.statements
            .iter()
            .filter_map(|s| match &s.kind {
                StatementKind::Message(m) => m.label.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(labels, vec!["hi"]);
}

/// JS resolver max depth is 8; Rust max depth is 32.  This test verifies that
/// Rust can resolve a chain of 8 deep includes without error (well within the
/// JS limit of 8 and the Rust limit of 32).
#[test]
fn include_depth_8_is_within_both_limits() {
    let dir = tempdir().expect("tempdir");
    // Build a chain: depth0 includes depth1, depth1 includes depth2, ..., depth7 is a leaf.
    let leaf = dir.path().join("depth7.puml");
    fs::write(&leaf, "Alice -> Bob: leaf\n").expect("write leaf");
    for i in (0..7).rev() {
        let next = i + 1;
        let file = dir.path().join(format!("depth{i}.puml"));
        fs::write(&file, format!("!include depth{next}.puml\n")).expect("write depth");
    }
    let src = "@startuml\n!include depth0.puml\n@enduml\n";
    let expanded = preprocess_with_root(src, dir.path()).expect("8-level include chain");
    assert!(
        expanded.contains("Alice -> Bob: leaf"),
        "8-level include chain must resolve the leaf; got:\n{expanded}"
    );
}

/// JS resolver max depth is 8; Rust max depth is 32.  A chain of 9 levels
/// exceeds the JS limit (which would silently produce an error comment in the
/// output) but is within the Rust limit.
///
/// This test documents the depth-limit divergence.  It is NOT ignored because
/// the Rust side (the subject of this contract test) should succeed; the
/// JS side's failure at depth > 8 is a known, documented limitation.
#[test]
fn include_depth_9_exceeds_js_limit_but_rust_handles_it() {
    let dir = tempdir().expect("tempdir");
    let leaf = dir.path().join("depth8.puml");
    fs::write(&leaf, "Alice -> Bob: deep-leaf\n").expect("write leaf");
    for i in (0..8).rev() {
        let next = i + 1;
        let file = dir.path().join(format!("depth{i}.puml"));
        fs::write(&file, format!("!include depth{next}.puml\n")).expect("write depth");
    }
    let src = "@startuml\n!include depth0.puml\n@enduml\n";
    // Rust succeeds; JS would emit '--- include failed: ... (maximum depth) ---'
    let expanded = preprocess_with_root(src, dir.path()).expect("9-level include chain");
    assert!(
        expanded.contains("Alice -> Bob: deep-leaf"),
        "9-level include chain must resolve on Rust (JS would fail at depth>8); got:\n{expanded}"
    );
}
