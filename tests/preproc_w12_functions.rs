/// Wave-12 batch A — preprocessor `!function`, `!procedure`, arithmetic
/// expressions, and built-in functions.
///
/// These tests verify the parsing and evaluation layer that powers every
/// PlantUML stdlib library (AWS, C4, Azure, Material Icons …).  Each stdlib
/// uses `!function`/`!procedure` for macro-like reusable definitions plus
/// arithmetic in `!if` expressions.  Wave-4 added `!definelong`; this is the
/// next tier.
///
/// Scope: preproc-only.  We do not touch parser/, render/, or normalize/.
use puml::{ast::StatementKind, parse};

// ── helpers ────────────────────────────────────────────────────────────────────

/// Extract all message labels from the parsed document.
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

/// Assert that parsing succeeds without caring about content.
// Used by some tests; suppress the dead_code lint for completeness.
#[allow(dead_code)]
fn preprocess_ok(src: &str) {
    parse(src).expect("parse should succeed");
}

// ── !function — basic arithmetic return ────────────────────────────────────────

/// A `!function` that returns twice its input should produce 14 when called
/// with 7.  The result is stored in a `!$` variable and used as a message label.
#[test]
fn preproc_function_double_returns_2x_input() {
    let src = "@startuml
!function $double($x)
  !return $x * 2
!endfunction

!$result = $double(7)
A -> B : $result
@enduml";
    assert_eq!(msg_labels(src), vec!["14"]);
}

// ── !procedure — inline PUML source emission ──────────────────────────────────

/// A `!procedure` expands its body as PUML source.  Calling `$logf("hello")`
/// should emit the statement `A -> B : hello` into the output.
#[test]
fn preproc_procedure_logf_inlines_note_statement() {
    let src = "@startuml
!procedure $logf($msg)
A -> B : $msg
!endprocedure

$logf(hello)
@enduml";
    assert_eq!(msg_labels(src), vec!["hello"]);
}

// ── %intval built-in ──────────────────────────────────────────────────────────

/// `%intval` coerces a string to an integer.  "42" → 42, "0x1F" → 0 (non-hex
/// prefix returns 0 for the non-numeric suffix), and a bare integer is
/// returned as-is.
#[test]
fn preproc_intval_function_evaluates_to_integer() {
    let src = "@startuml
!$a = %intval(\"42\")
!$b = %intval(\"7\")
!$c = %eval($a + $b)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["49"]);
}

// ── %strlen built-in ──────────────────────────────────────────────────────────

/// `%strlen` returns the number of characters in a string.
#[test]
fn preproc_strlen_function_returns_string_length() {
    let src = "@startuml
!$n = %strlen(\"hello\")
A -> B : $n
@enduml";
    assert_eq!(msg_labels(src), vec!["5"]);
}

// ── !if with arithmetic comparison ───────────────────────────────────────────

/// An `!if` whose condition involves arithmetic (`%strlen("hello") > 3`)
/// should route into the true branch and emit the expected message.
#[test]
fn preproc_if_with_arithmetic_comparison_routes_branch() {
    let src = "@startuml
!if %strlen(\"hello\") > 3
A -> B : long
!else
A -> B : short
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["long"]);
}

// ── !if with boolean && and || ────────────────────────────────────────────────

/// Compound boolean `&&`/`||` in `!if` conditions should short-circuit
/// correctly.  We test three cases: pure &&, pure ||, and a mix.
#[test]
fn preproc_if_with_boolean_and_or_routes_branch() {
    let src = "@startuml
!$x = 14
!if %intval($x) >= 10 && %intval($x) <= 20
A -> B : in-range
!endif
!if %intval($x) < 5 || %intval($x) > 10
A -> B : out-of-5
!endif
!if %intval($x) < 5 || %intval($x) <= 20
A -> B : or-second-true
!endif
@enduml";
    assert_eq!(
        msg_labels(src),
        vec!["in-range", "out-of-5", "or-second-true"]
    );
}

// ── !$var assignment persists ─────────────────────────────────────────────────

/// A variable assigned with `!$var = EXPR` on one line should be visible on
/// all subsequent lines, including inside `!if` conditions.
#[test]
fn preproc_var_assignment_persists_across_lines() {
    let src = "@startuml
!$counter = 0
!$counter = %eval($counter + 1)
!$counter = %eval($counter + 1)
!$counter = %eval($counter + 1)
!if $counter == 3
A -> B : three
!else
A -> B : wrong
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["three"]);
}

// ── additional built-ins ──────────────────────────────────────────────────────

/// `%substr(s, i, n)` returns `n` characters of `s` starting at index `i`.
#[test]
fn preproc_substr_extracts_slice() {
    let src = "@startuml
!$s = %substr(\"hello world\", 6, 5)
A -> B : $s
@enduml";
    assert_eq!(msg_labels(src), vec!["world"]);
}

/// `%strpos(s, t)` returns the first character index of `t` in `s`, or -1.
#[test]
fn preproc_strpos_finds_substring_index() {
    let src = "@startuml
!$i = %strpos(\"foobar\", \"bar\")
A -> B : $i
@enduml";
    assert_eq!(msg_labels(src), vec!["3"]);
}

/// `%abs(x)` returns the absolute value.
#[test]
fn preproc_abs_returns_positive() {
    let src = "@startuml
!$v = %abs(-7)
A -> B : $v
@enduml";
    assert_eq!(msg_labels(src), vec!["7"]);
}

/// `%min(a, b)` and `%max(a, b)` return the smaller / larger value.
#[test]
fn preproc_min_max_select_extremes() {
    let src = "@startuml
!$lo = %min(3, 8)
!$hi = %max(3, 8)
A -> B : $lo
A -> B : $hi
@enduml";
    assert_eq!(msg_labels(src), vec!["3", "8"]);
}

/// `%dec2hex(n)` converts a decimal integer to lowercase hex.
#[test]
fn preproc_dec2hex_converts_decimal_to_hex() {
    let src = "@startuml
!$h = %dec2hex(255)
A -> B : $h
@enduml";
    assert_eq!(msg_labels(src), vec!["ff"]);
}

/// `%hex2dec(h)` converts a hex string to decimal.
#[test]
fn preproc_hex2dec_converts_hex_to_decimal() {
    let src = "@startuml
!$n = %hex2dec(\"ff\")
A -> B : $n
@enduml";
    assert_eq!(msg_labels(src), vec!["255"]);
}

/// `%ifempty(s, default)` returns the default when `s` is empty.
#[test]
fn preproc_ifempty_returns_default_when_empty() {
    let src = "@startuml
!$val = %ifempty(\"\", \"fallback\")
A -> B : $val
@enduml";
    assert_eq!(msg_labels(src), vec!["fallback"]);
}

/// `%ifempty(s, default)` returns `s` unchanged when `s` is non-empty.
#[test]
fn preproc_ifempty_returns_value_when_nonempty() {
    let src = "@startuml
!$val = %ifempty(\"present\", \"fallback\")
A -> B : $val
@enduml";
    assert_eq!(msg_labels(src), vec!["present"]);
}

/// `%datetime` returns a non-empty deterministic timestamp string.
#[test]
fn preproc_datetime_returns_nonempty_string() {
    let src = "@startuml
!$ts = %datetime()
!if %strlen($ts) > 0
A -> B : ok
!else
A -> B : fail
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["ok"]);
}

/// `%lineno` is defined and returns a numeric string (deterministically 0 in
/// our offline renderer, which is acceptable for stdlib compatibility).
#[test]
fn preproc_lineno_returns_numeric_string() {
    let src = "@startuml
!$ln = %lineno()
!if %intval($ln) >= 0
A -> B : ok
!else
A -> B : fail
!endif
@enduml";
    assert_eq!(msg_labels(src), vec!["ok"]);
}

// ── function calling another function ─────────────────────────────────────────

/// A `!function` can call another `!function` in its `!return` expression.
/// `$quad(x)` = `$double($double(x))` should return 4x input.
#[test]
fn preproc_function_calls_another_function() {
    let src = "@startuml
!function $double($x)
  !return $x * 2
!endfunction
!function $quad($x)
  !return $double($double($x))
!endfunction
!$r = $quad(3)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["12"]);
}

// ── procedure with conditional body ──────────────────────────────────────────

/// A `!procedure` body may contain `!if` blocks.  Only the active branch
/// emits output.
#[test]
fn preproc_procedure_body_with_conditional() {
    let src = "@startuml
!procedure $tagged($label, $flag)
!if $flag
A -> B : yes-$label
!else
A -> B : no-$label
!endif
!endprocedure

$tagged(alpha, 1)
$tagged(beta, 0)
@enduml";
    assert_eq!(msg_labels(src), vec!["yes-alpha", "no-beta"]);
}

// ── default parameter value on !function ──────────────────────────────────────

/// When a `!function` parameter has a default value and no argument is passed,
/// the default should be used.
#[test]
fn preproc_function_default_param_used_when_omitted() {
    let src = "@startuml
!function $add($a, $b = 10)
  !return $a + $b
!endfunction
!$r = $add(5)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["15"]);
}

// ── stdlib-style pattern: function + procedure together ───────────────────────

/// Mimics a C4/AWS-style stdlib pattern: a `!function` computes a derived
/// value and a `!procedure` emits diagram source using that function's result.
#[test]
fn preproc_stdlib_style_function_and_procedure_combined() {
    let src = "@startuml
!function $label($base, $suffix)
  !return $base + $suffix
!endfunction

!procedure $box($name, $kind)
A -> B : $name is $kind
!endprocedure

!$tag = $label(\"aws-\", \"s3\")
$box($tag, storage)
@enduml";
    assert_eq!(msg_labels(src), vec!["aws-s3 is storage"]);
}
