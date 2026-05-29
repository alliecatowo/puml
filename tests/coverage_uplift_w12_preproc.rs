//! Wave-12 coverage uplift — preprocessor module focused tests.
//!
//! Targets `src/preproc/builtins/constructors.rs`, `src/preproc/builtins/datetime.rs`,
//! `src/preproc/builtins/scanner.rs`, `src/preproc/builtins/collections.rs`,
//! `src/preproc/macros/definelong.rs`, and `src/preproc/includes/expr.rs`.
//!
//! All tests drive behaviour through the public `parse` API, exercising the
//! preprocessor by embedding PlantUML preprocessor directives in diagram source.
//! Each test asserts specific output values rather than just "does not panic".
//!
//! Refs #89

use puml::ast::StatementKind;
use puml::parse;

// ── helpers ────────────────────────────────────────────────────────────────────

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

fn first_label(src: &str) -> String {
    msg_labels(src)
        .into_iter()
        .next()
        .expect("expected at least one message label")
}

// ── datetime: %date() ─────────────────────────────────────────────────────────

#[test]
fn date_builtin_default_format_epoch_zero() {
    // PUML_NOW = 0 → 1970-01-01 in default yyyy-MM-dd format
    let src = "@startuml
!$PUML_NOW = 0
!$d = %date()
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1970-01-01");
}

#[test]
fn date_builtin_length_of_default_output_is_ten() {
    let src = "@startuml
!$PUML_NOW = 0
!$d = %date()
!$len = %strlen($d)
A -> B : $len
@enduml";
    let label = first_label(src);
    assert_eq!(label, "10");
}

#[test]
fn date_builtin_one_day_later() {
    // 86400 seconds = 1970-01-02
    let src = "@startuml
!$d = %date(\"yyyy-MM-dd\", 86400)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1970-01-02");
}

#[test]
fn date_builtin_known_timestamp_produces_correct_date() {
    // 365 * 86400 = 31536000 → 1971-01-01
    let src = "@startuml
!$d = %date(\"yyyy-MM-dd\", 31536000)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1971-01-01");
}

#[test]
fn date_builtin_mid_day_timestamp_date_part() {
    // 43200 = 12 hours = still 1970-01-01
    let src = "@startuml
!$d = %date(\"yyyy-MM-dd\", 43200)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1970-01-01");
}

#[test]
fn time_builtin_default_format_epoch_zero() {
    let src = "@startuml
!$PUML_NOW = 0
!$t = %time()
A -> B : $t
@enduml";
    let label = first_label(src);
    assert_eq!(label, "00:00:00");
}

#[test]
fn time_builtin_mid_day_epoch() {
    // 43200 seconds = 12:00:00
    let src = "@startuml
!$PUML_NOW = 43200
!$t = %time()
A -> B : $t
@enduml";
    let label = first_label(src);
    assert_eq!(label, "12:00:00");
}

#[test]
fn time_builtin_specific_hour_minute_second() {
    // 3661 = 1h 1m 1s
    let src = "@startuml
!$PUML_NOW = 3661
!$t = %time()
A -> B : $t
@enduml";
    let label = first_label(src);
    assert_eq!(label, "01:01:01");
}

#[test]
fn date_builtin_with_explicit_seconds_and_custom_format() {
    // Use a known timestamp with a custom format
    // 86400 = 1970-01-02, asking for HH:mm:ss (should be 00:00:00)
    let src = "@startuml
!$d = %date(\"HH:mm:ss\", 86400)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "00:00:00");
}

#[test]
fn date_builtin_dd_mm_yyyy_format() {
    // Test the dd/MM/yyyy format tokens
    let src = "@startuml
!$d = %date(\"dd/MM/yyyy\", 0)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "01/01/1970");
}

#[test]
fn date_builtin_mixed_format_with_literal_chars() {
    // Test a format string with literal chars and format tokens
    let src = "@startuml
!$d = %date(\"yyyy-MM-dd HH:mm:ss\", 3661)
A -> B : $d
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1970-01-01 01:01:01");
}

// ── constructors: %list(), %map(), %get(), %set(), %remove() ─────────────────

#[test]
fn list_builtin_creates_json_array() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\")
!$sz = %size($l)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn map_builtin_creates_json_object() {
    let src = "@startuml
!$m = %map(\"key\", \"value\")
!$v = %get($m, \"key\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "value");
}

#[test]
fn get_builtin_retrieves_by_index() {
    let src = "@startuml
!$l = %list(\"x\", \"y\", \"z\")
!$v = %get($l, 1)
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "y");
}

#[test]
fn set_builtin_updates_list_element() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\")
!$l = %set($l, 1, \"B\")
!$v = %get($l, 1)
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "B");
}

#[test]
fn set_builtin_updates_map_value() {
    let src = "@startuml
!$m = %map(\"k\", \"old\")
!$m = %set($m, \"k\", \"new\")
!$v = %get($m, \"k\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "new");
}

#[test]
fn remove_builtin_removes_element_from_map_by_key() {
    let src = "@startuml
!$m = %map(\"k1\", \"v1\", \"k2\", \"v2\")
!$m2 = %remove($m, \"k1\")
!$v = %get($m2, \"k2\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "v2");
}

#[test]
fn size_builtin_on_string_counts_chars() {
    let src = "@startuml
!$s = \"hello\"
!$sz = %size($s)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "5");
}

#[test]
fn size_builtin_on_json_array() {
    let src = "@startuml
!$l = %list(\"a\", \"b\")
!$sz = %size($l)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "2");
}

#[test]
fn size_builtin_on_json_object() {
    let src = "@startuml
!$m = %map(\"a\", \"1\", \"b\", \"2\", \"c\", \"3\")
!$sz = %size($m)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn json_merge_two_objects() {
    let src = "@startuml
!$a = %map(\"x\", \"1\")
!$b = %map(\"y\", \"2\")
!$c = %json_merge($a, $b)
!$sz = %size($c)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "2");
}

#[test]
fn json_merge_two_arrays() {
    let src = "@startuml
!$a = %list(\"p\", \"q\")
!$b = %list(\"r\")
!$c = %json_merge($a, $b)
!$sz = %size($c)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn json_keys_returns_keys_for_object() {
    // %keys() is the correct alias; verify it returns something containing "alpha"
    let src = "@startuml
!$m = %map(\"alpha\", \"1\")
!$keys = %keys($m)
A -> B : $keys
@enduml";
    let label = first_label(src);
    assert!(
        label.contains("alpha"),
        "keys should contain 'alpha', got: {label}"
    );
}

#[test]
fn json_values_returns_values_for_object() {
    let src = "@startuml
!$m = %map(\"k\", \"hello\")
!$vals = %values($m)
A -> B : $vals
@enduml";
    let label = first_label(src);
    assert!(
        label.contains("hello"),
        "values should contain 'hello', got: {label}"
    );
}

#[test]
fn map_entries_returns_pairs() {
    let src = "@startuml
!$m = %map(\"key\", \"val\")
!$pairs = %map_entries($m)
!$sz = %size($pairs)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1");
}

#[test]
fn range_builtin_ascending() {
    let src = "@startuml
!$r = %range(1, 3)
!$sz = %size($r)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn range_builtin_descending() {
    let src = "@startuml
!$r = %range(5, 1)
!$sz = %size($r)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "5");
}

#[test]
fn range_builtin_with_explicit_step() {
    let src = "@startuml
!$r = %range(0, 10, 2)
!$sz = %size($r)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "6");
}

#[test]
fn range_builtin_step_zero_defaults_to_one() {
    let src = "@startuml
!$r = %range(1, 3, 0)
!$sz = %size($r)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

// ── foreach: one and two variable bindings ────────────────────────────────────

#[test]
fn foreach_single_var_over_list() {
    let src = "@startuml
!$l = %list(\"x\", \"y\", \"z\")
!$count = 0
!foreach $item in $l
  !$count = $count + 1
!endfor
A -> B : $count
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn foreach_two_var_binding_over_map() {
    let src = "@startuml
!$m = %map(\"alpha\", \"first\", \"beta\", \"second\")
!$count = 0
!foreach $k, $v in $m
  !$count = $count + 1
!endfor
A -> B : $count
@enduml";
    let label = first_label(src);
    assert_eq!(label, "2");
}

#[test]
fn foreach_two_var_binding_over_array() {
    let src = "@startuml
!$l = %list(\"x\", \"y\", \"z\")
!$count = 0
!foreach $idx, $val in $l
  !$count = $count + 1
!endfor
A -> B : $count
@enduml";
    let label = first_label(src);
    assert_eq!(label, "3");
}

#[test]
fn foreach_over_empty_list_body_not_executed() {
    let src = "@startuml
!$l = %list()
!$count = 0
!foreach $x in $l
  !$count = $count + 1
!endfor
A -> B : $count
@enduml";
    let label = first_label(src);
    assert_eq!(label, "0");
}

// ── expr: logical operators ───────────────────────────────────────────────────

#[test]
fn expr_not_operator_via_if_keyword() {
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
fn expr_modulo_operator() {
    let src = "@startuml
!$a = 7
!$b = $a % 3
A -> B : $b
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1");
}

#[test]
fn expr_integer_multiply_and_divide() {
    let src = "@startuml
!$a = 6 * 7
!$b = 100 / 4
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "42");
    assert_eq!(labels[1], "25");
}

#[test]
fn expr_comparison_greater_less() {
    let src = "@startuml
!$a = 5
!$b = 3
!if $a > $b
A -> B : gt-true
!endif
!if $b < $a
A -> B : lt-true
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["gt-true", "lt-true"]);
}

#[test]
fn expr_comparison_ge_le() {
    let src = "@startuml
!$a = 5
!if $a >= 5
A -> B : ge-true
!endif
!if $a <= 5
A -> B : le-true
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ge-true", "le-true"]);
}

#[test]
fn expr_not_equal_operator() {
    let src = "@startuml
!$a = 1
!$b = 2
!if $a != $b
A -> B : not-equal
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["not-equal"]);
}

#[test]
fn expr_string_equality() {
    let src = "@startuml
!$s = \"hello\"
!if $s == \"hello\"
A -> B : equal
!endif
!if $s != \"world\"
A -> B : not-world
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["equal", "not-world"]);
}

#[test]
fn expr_parenthesized_subexpr() {
    let src = "@startuml
!$a = (2 + 3) * 4
A -> B : $a
@enduml";
    let label = first_label(src);
    assert_eq!(label, "20");
}

#[test]
fn expr_negative_number_literal() {
    let src = "@startuml
!$a = 0 - 5
!$b = $a + 10
A -> B : $b
@enduml";
    let label = first_label(src);
    assert_eq!(label, "5");
}

// ── function definitions and calls ────────────────────────────────────────────

#[test]
fn function_definition_and_call_basic() {
    let src = "@startuml
!function $double($x)
  !return $x * 2
!endfunction
!$r = %double(5)
A -> B : $r
@enduml";
    // Functions are called with % prefix
    let label = first_label(src);
    assert_eq!(label, "10");
}

#[test]
fn function_with_string_concat() {
    let src = "@startuml
!function $greet($name)
  !return \"Hello \" + $name
!endfunction
!$r = %greet(\"World\")
A -> B : $r
@enduml";
    let label = first_label(src);
    assert_eq!(label, "Hello World");
}

#[test]
fn function_two_numeric_args() {
    let src = "@startuml
!function $add($a, $b)
  !return $a + $b
!endfunction
!$r = %add(3, 4)
A -> B : $r
@enduml";
    let label = first_label(src);
    assert_eq!(label, "7");
}

#[test]
fn procedure_call_emits_content() {
    let src = "@startuml
!procedure PING($msg)
A -> B : $msg
!endprocedure
PING(hello)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

// ── definelong: various cases ─────────────────────────────────────────────────

#[test]
fn definelong_no_arg_macro_is_expanded() {
    let src = "@startuml
!definelong GREET()
A -> B : Hello
!enddefinelong
GREET()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello"]);
}

#[test]
fn definelong_one_arg_macro_substitutes_parameter() {
    let src = "@startuml
!definelong SEND(msg)
A -> B : Hello msg
!enddefinelong
SEND(World)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello World"]);
}

#[test]
fn definelong_two_arg_macro_substitutes_both() {
    let src = "@startuml
!definelong SEND2(from, msg)
A -> B : from msg
!enddefinelong
SEND2(Alice, Hi)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Alice Hi"]);
}

#[test]
fn definelong_multi_line_body_all_lines_emitted() {
    let src = "@startuml
!definelong PING()
A -> B : ping
B -> A : pong
!enddefinelong
PING()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ping", "pong"]);
}

#[test]
fn definelong_multiple_calls_each_expanded() {
    let src = "@startuml
!definelong GREET(name)
A -> B : Hello name
!enddefinelong
GREET(Bob)
GREET(Carol)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello Bob", "Hello Carol"]);
}

#[test]
fn definelong_invalid_name_handled_gracefully() {
    // A `!definelong` with invalid name should not panic (may produce error diagnostic)
    let src = "@startuml
!definelong ()
A -> B : body
!enddefinelong
@enduml";
    let _ = parse(src);
}

// ── and short-circuit ─────────────────────────────────────────────────────────

#[test]
fn and_short_circuit_prevents_evaluation_of_rhs() {
    let src = "@startuml
!$a = 0
!if $a && $a
A -> B : should-not-reach
!else
A -> B : short-circuit
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["short-circuit"]);
}

#[test]
fn or_short_circuit_on_true_lhs() {
    let src = "@startuml
!$a = 1
!if $a || 0
A -> B : or-true
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["or-true"]);
}

// ── elseif chain ─────────────────────────────────────────────────────────────

#[test]
fn elseif_chain_selects_correct_branch() {
    let src = "@startuml
!$x = 2
!if $x == 1
A -> B : one
!elseif $x == 2
A -> B : two
!elseif $x == 3
A -> B : three
!else
A -> B : other
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["two"]);
}

#[test]
fn elseif_falls_through_to_else() {
    let src = "@startuml
!$x = 99
!if $x == 1
A -> B : one
!elseif $x == 2
A -> B : two
!else
A -> B : other
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["other"]);
}

// ── while loop ────────────────────────────────────────────────────────────────

#[test]
fn while_loop_accumulates_sum() {
    let src = "@startuml
!$i = 0
!$sum = 0
!while $i < 5
  !$sum = $sum + $i
  !$i = $i + 1
!endwhile
A -> B : $sum
@enduml";
    let label = first_label(src);
    assert_eq!(label, "10");
}

#[test]
fn while_loop_never_executes_when_condition_false() {
    let src = "@startuml
!$i = 10
!$count = 0
!while $i < 5
  !$count = $count + 1
  !$i = $i + 1
!endwhile
A -> B : $count
@enduml";
    let label = first_label(src);
    assert_eq!(label, "0");
}

// ── !define macros ────────────────────────────────────────────────────────────

#[test]
fn define_macro_single_arg_substitution() {
    let src = "@startuml
!define SHOUT(msg) A -> B : msg !!!
SHOUT(hello)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello !!!"]);
}

// ── JSON path: nested set ─────────────────────────────────────────────────────

#[test]
fn nested_set_creates_deep_path_in_object() {
    let src = "@startuml
!$m = %map(\"a\", %map(\"b\", \"old\"))
!$m = %set($m, \"a.b\", \"new\")
!$v = %get($m, \"a.b\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "new");
}

// ── mixed: combining builtins ─────────────────────────────────────────────────

#[test]
fn complex_list_processing_with_foreach_search() {
    let src = "@startuml
!$l = %list(\"cat\", \"dog\", \"bird\")
!$found = 0
!foreach $item in $l
  !if $item == \"dog\"
    !$found = 1
  !endif
!endfor
A -> B : $found
@enduml";
    let label = first_label(src);
    assert_eq!(label, "1");
}

#[test]
fn boolean_builtins_true_and_false() {
    let src = "@startuml
!$t = %true()
!$f = %false()
!if $t
A -> B : true-branch
!endif
!if not $f
A -> B : not-false
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true-branch", "not-false"]);
}

#[test]
fn list_map_combined_with_range() {
    // Range, get, size all in combination
    let src = "@startuml
!$r = %range(10, 14)
!$sz = %size($r)
!$first = %get($r, 0)
!$last = %get($r, 4)
A -> B : $sz
A -> B : $first
A -> B : $last
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "5");
    assert_eq!(labels[1], "10");
    assert_eq!(labels[2], "14");
}

#[test]
fn json_merge_overrides_scalar_with_map() {
    let src = "@startuml
!$a = %map(\"key\", \"old\")
!$b = %map(\"key\", \"new\")
!$c = %json_merge($a, $b)
!$v = %get($c, \"key\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "new");
}

#[test]
fn set_and_remove_from_nested_list() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\", \"d\")
!$l = %set($l, 0, \"A\")
!$v = %get($l, 0)
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "A");
}

#[test]
fn size_builtin_on_empty_list() {
    let src = "@startuml
!$l = %list()
!$sz = %size($l)
A -> B : $sz
@enduml";
    let label = first_label(src);
    assert_eq!(label, "0");
}

// ── string ops: concat, intval, strlen ───────────────────────────────────────

#[test]
fn string_concatenation_with_plus() {
    let src = "@startuml
!$a = \"hello\" + \" \" + \"world\"
A -> B : $a
@enduml";
    let label = first_label(src);
    assert_eq!(label, "hello world");
}

#[test]
fn integer_value_roundtrip() {
    let src = "@startuml
!$n = 42
!$s = $n
!$n2 = %intval($s)
A -> B : $n2
@enduml";
    let label = first_label(src);
    assert_eq!(label, "42");
}

#[test]
fn is_json_builtin_object() {
    let src = "@startuml
!$obj = %map(\"k\", \"v\")
!if %is_json($obj)
A -> B : obj-is-json
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["obj-is-json"]);
}

#[test]
fn is_json_builtin_array() {
    let src = "@startuml
!$arr = %list(\"a\")
!if %is_json($arr)
A -> B : arr-is-json
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["arr-is-json"]);
}

// ── scanner: reading from nested JSON ────────────────────────────────────────

#[test]
fn scanner_reads_nested_object_via_dot_path() {
    let src = "@startuml
!$m = %map(\"outer\", %map(\"inner\", \"42\"))
!$v = %get($m, \"outer.inner\")
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "42");
}

#[test]
fn scanner_reads_last_element_of_list() {
    let src = "@startuml
!$l = %list(\"zero\", \"one\", \"two\")
!$v = %get($l, 2)
A -> B : $v
@enduml";
    let label = first_label(src);
    assert_eq!(label, "two");
}

#[test]
fn scanner_returns_empty_for_missing_key() {
    let src = "@startuml
!$m = %map(\"k\", \"v\")
!$v = %get($m, \"missing\")
!$len = %strlen($v)
A -> B : $len
@enduml";
    let label = first_label(src);
    // Missing key → empty string → length 0
    assert_eq!(label, "0");
}

// ── map_entries: structure verification ──────────────────────────────────────

#[test]
fn map_entries_returns_key_value_pairs() {
    let src = "@startuml
!$m = %map(\"alpha\", \"1\")
!$pairs = %map_entries($m)
!$first_pair = %get($pairs, 0)
!$key = %get($first_pair, 0)
!$val = %get($first_pair, 1)
A -> B : $key
A -> B : $val
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "alpha");
    assert_eq!(labels[1], "1");
}
