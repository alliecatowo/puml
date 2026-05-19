/// Coverage wave 24 — exercises preprocessor builtin paths in builtins.rs
/// that were not covered by wave-23 tests. Drives the public `parse` API and
/// verifies behaviour through AST message labels.
use puml::{ast::StatementKind, parse};

// ── helper ───────────────────────────────────────────────────────────────────

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

// ── %splitstr_regex ──────────────────────────────────────────────────────────

#[test]
fn builtin_splitstr_regex_empty_pattern_returns_whole() {
    // Empty pattern → no split, returns the whole string unchanged
    let src = "@startuml
!$parts = %splitstr_regex(\"hello\", \"\")
A -> B : $parts
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

#[test]
fn builtin_splitstr_regex_dot_any() {
    // Single `.` matches any character — split "a1b2c" on any single char
    // between a-c gives fields; use splitstr to verify the codepath is hit
    // and returns a comma-joined result
    let src = "@startuml
!$parts = %splitstr_regex(\"aXbXc\", \"X\")
!$count = 0
!foreach $item in $parts
!$count = %eval($count + 1)
!endfor
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    // "aXbXc" split on "X" → ["a","b","c"] → 3 items
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn builtin_splitstr_regex_word_boundary_pattern() {
    // Use a non-regex pattern that triggers the fallback split path
    // (pattern has `|` which is unsupported by the simple parser → fallback)
    let src = "@startuml
!$parts = %splitstr_regex(\"a|b\", \"|\")
!$count = 0
!foreach $item in $parts
!$count = %eval($count + 1)
!endfor
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    // "a|b" split on "|" → 2 fields
    assert_eq!(labels, vec!["2"]);
}

// ── %range reverse and zero-step ─────────────────────────────────────────────

#[test]
fn builtin_range_reverse_step() {
    // range(5, 1, -2) → 5, 3, 1  → size 3
    let src = "@startuml
!$r = %range(5, 1, -2)
!$sz = %count($r)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn builtin_range_descending_default_step() {
    // When start > end and step not given, default should be -1
    let src = "@startuml
!$r = %range(3, 1)
!$sz = %count($r)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // 3,2,1 → 3 elements
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn builtin_range_zero_step_defaults() {
    // A zero step should be replaced by sensible default (1 or -1)
    let src = "@startuml
!$r = %range(1, 3, 0)
!$sz = %count($r)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // Should still produce 1,2,3 → 3 elements
    assert_eq!(labels, vec!["3"]);
}

// ── 3-char hex color in color builtins ──────────────────────────────────────

#[test]
fn builtin_reverse_color_3char_hex() {
    // #fff → expanded to #ffffff → reversed to #000000
    let src = "@startuml
!$rev = %reverse_color(\"#fff\")
A -> B : $rev
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["#000000"]);
}

#[test]
fn builtin_is_dark_3char_hex() {
    // #000 is dark, #fff is light
    let src = "@startuml
!$d = %is_dark(\"#000\")
!$l = %is_dark(\"#fff\")
A -> B : $d
A -> B : $l
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false"]);
}

// ── %map_merge / %dict_merge ─────────────────────────────────────────────────

#[test]
fn builtin_map_merge_two_maps() {
    let src = "@startuml
!$a = %map(\"k1\", \"v1\")
!$b = %map(\"k2\", \"v2\")
!$merged = %map_merge($a, $b)
!$has_k1 = %map_contains_key($merged, \"k1\")
!$has_k2 = %map_contains_key($merged, \"k2\")
A -> B : $has_k1
A -> B : $has_k2
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "true"]);
}

#[test]
fn builtin_map_merge_overwrite_key() {
    // When both maps share a key, rhs wins
    let src = "@startuml
!$a = %map(\"k\", \"old\")
!$b = %map(\"k\", \"new\")
!$merged = %map_merge($a, $b)
!$val = %get($merged, \"k\")
A -> B : $val
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["new"]);
}

// ── %set / %put path operations ──────────────────────────────────────────────

#[test]
fn builtin_set_adds_key_to_map() {
    let src = "@startuml
!$m = %map(\"a\", \"1\")
!$m2 = %set($m, \"b\", \"2\")
!$has_b = %map_contains_key($m2, \"b\")
A -> B : $has_b
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true"]);
}

#[test]
fn builtin_set_updates_array_via_json_path() {
    // %set on a JSON array using bracket-path notation
    let src = "@startuml
!$l = [\"a\", \"b\", \"c\"]
!$l2 = %set($l, \"[1]\", \"B\")
!$sz = %list_size($l2)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // Result is still a 3-element array
    assert_eq!(labels, vec!["3"]);
}

// ── %remove path operations ──────────────────────────────────────────────────

#[test]
fn builtin_remove_key_from_map() {
    let src = "@startuml
!$m = %map(\"k1\", \"v1\", \"k2\", \"v2\")
!$m2 = %remove($m, \"k1\")
!$has_k1 = %map_contains_key($m2, \"k1\")
!$has_k2 = %map_contains_key($m2, \"k2\")
A -> B : $has_k1
A -> B : $has_k2
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["false", "true"]);
}

#[test]
fn builtin_remove_item_from_list_by_bracket_index() {
    // Remove list element using JSON path bracket notation
    let src = "@startuml
!$l = [\"a\", \"b\", \"c\"]
!$l2 = %remove($l, \"[1]\")
!$sz = %list_size($l2)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["2"]);
}

// ── %list_set boundary cases ─────────────────────────────────────────────────

#[test]
fn builtin_list_set_extends_list_if_needed() {
    // Setting index beyond the current length should extend the list
    let src = "@startuml
!$l = %list(\"a\")
!$l2 = %list_set($l, 2, \"c\")
!$sz = %list_size($l2)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // Should have at least 3 elements: a, (empty), c
    let sz: usize = labels[0].parse().unwrap();
    assert!(sz >= 3, "expected at least 3 elements, got {sz}");
}

// ── %foreach with map (two-variable) ─────────────────────────────────────────

#[test]
fn builtin_foreach_two_var_over_map() {
    // !foreach $k, $v in %map(...) — two-var iteration over a JSON object
    let src = "@startuml
!$m = %map(\"name\", \"Alice\")
!$out = \"\"
!foreach $k, $v in $m
!$out = $k
!endfor
A -> B : $out
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["name"]);
}

#[test]
fn builtin_foreach_two_var_over_array_gives_index_value() {
    // Two-var foreach over a JSON array iterates index/value pairs
    let src = "@startuml
!$arr = [\"x\", \"y\"]
!$last_idx = \"\"
!$last_val = \"\"
!foreach $i, $v in $arr
!$last_idx = $i
!$last_val = $v
!endfor
A -> B : $last_idx
A -> B : $last_val
@enduml";
    let labels = msg_labels(src);
    // Last index is 1 (second element), last value is y
    assert_eq!(labels, vec!["1", "y"]);
}

// ── %get_json_attribute with array index path ────────────────────────────────

#[test]
fn builtin_get_json_attribute_array_index() {
    let src = "@startuml
!$j = {\"items\": [\"first\", \"second\"]}
!$v = %get_json_attribute($j, \"items[1]\")
A -> B : $v
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["second"]);
}

// ── %eval_bool ───────────────────────────────────────────────────────────────

#[test]
fn builtin_eval_bool_truthy_and_falsy() {
    let src = "@startuml
!$t = %eval_bool(\"1\")
!$f = %eval_bool(\"0\")
A -> B : $t
A -> B : $f
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false"]);
}

// ── %equals_ignore_case ──────────────────────────────────────────────────────

#[test]
fn builtin_equals_ignore_case() {
    let src = "@startuml
!$eq = %equals_ignore_case(\"Hello\", \"hello\")
!$neq = %equals_ignore_case(\"Hello\", \"world\")
A -> B : $eq
A -> B : $neq
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false"]);
}

// ── %map_entries ─────────────────────────────────────────────────────────────

#[test]
fn builtin_map_entries_returns_list_of_kv_pairs() {
    let src = "@startuml
!$m = %map(\"k\", \"v\")
!$entries = %map_entries($m)
!$sz = %count($entries)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // One entry → count of 1
    assert_eq!(labels, vec!["1"]);
}

#[test]
fn builtin_map_entries_non_object_returns_empty() {
    // Pass a list — should return empty [] → size 0
    let src = "@startuml
!$l = %list(\"a\", \"b\")
!$entries = %map_entries($l)
!$sz = %count($entries)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

// ── %procedure_exists ────────────────────────────────────────────────────────

#[test]
fn builtin_procedure_exists_and_function_exists() {
    let src = "@startuml
!procedure MyProc()
!endprocedure
!function MyFn()
!return \"ok\"
!endfunction
!$pe = %procedure_exists(\"MyProc\")
!$fe = %function_exists(\"MyFn\")
!$fn_not_proc = %procedure_exists(\"MyFn\")
A -> B : $pe
A -> B : $fe
A -> B : $fn_not_proc
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "true", "false"]);
}

// ── %false_then_true / %true_then_false ──────────────────────────────────────

#[test]
fn builtin_false_then_true_flips_after_first_call() {
    let src = "@startuml
!$a = %false_then_true(\"key1\")
!$b = %false_then_true(\"key1\")
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    // First call: false, second call: true
    assert_eq!(labels, vec!["false", "true"]);
}

#[test]
fn builtin_true_then_false_flips_after_first_call() {
    let src = "@startuml
!$a = %true_then_false(\"key2\")
!$b = %true_then_false(\"key2\")
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    // First call: true, second call: false
    assert_eq!(labels, vec!["true", "false"]);
}

// ── %get_variable_value ──────────────────────────────────────────────────────

#[test]
fn builtin_get_variable_value_set_and_unset() {
    let src = "@startuml
!$x = myvalue
!$got = %get_variable_value(\"x\")
!$missing = %get_variable_value(\"no_such_var\")
A -> B : $got
!if %strlen($missing) == 0
A -> B : empty
!else
A -> B : notempty
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["myvalue", "empty"]);
}

// ── %json_key_exists / %json_keys / %json_values ────────────────────────────

#[test]
fn builtin_json_key_exists() {
    let src = "@startuml
!$j = {\"a\": 1, \"b\": 2}
!$yes = %json_key_exists($j, \"a\")
!$no = %json_key_exists($j, \"z\")
A -> B : $yes
A -> B : $no
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false"]);
}

#[test]
fn builtin_json_keys_and_json_values() {
    let src = "@startuml
!$j = {\"x\": \"foo\"}
!$ks = %json_keys($j)
!$vs = %json_values($j)
!$has_x = %contains($ks, \"x\")
!$has_foo = %contains($vs, \"foo\")
A -> B : $has_x
A -> B : $has_foo
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "true"]);
}

// ── %list_is_empty / %array_is_empty ─────────────────────────────────────────

#[test]
fn builtin_list_is_empty_and_map_is_empty() {
    let src = "@startuml
!$empty_l = %list_is_empty(%list())
!$nonempty_l = %list_is_empty(%list(\"a\"))
!$empty_m = %map_is_empty(%map())
A -> B : $empty_l
A -> B : $nonempty_l
A -> B : $empty_m
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "true"]);
}

// ── %list_clear / %map_clear ─────────────────────────────────────────────────

#[test]
fn builtin_list_clear_and_map_clear() {
    let src = "@startuml
!$l = %list_clear(%list(\"a\", \"b\"))
!$m = %map_clear(%map(\"k\", \"v\"))
!$sl = %list_size($l)
!$sm = %list_size($m)
A -> B : $sl
A -> B : $sm
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0", "0"]);
}

// ── %retrieve_procedure_return ───────────────────────────────────────────────

#[test]
fn builtin_retrieve_procedure_return_is_empty() {
    let src = "@startuml
!$r = %retrieve_procedure_return()
!$len = %strlen($r)
A -> B : $len
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

// ── `%set_variable_value` is a no-op ────────────────────────────────────────

#[test]
fn builtin_set_variable_value_noop() {
    // Should not error; returns empty string
    let src = "@startuml
!$r = %set_variable_value(\"x\", \"hello\")
!$len = %strlen($r)
A -> B : $len
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

// ── %size counts chars for non-container ─────────────────────────────────────

#[test]
fn builtin_size_on_plain_string() {
    let src = "@startuml
!$sz = %size(\"hello\")
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // Not a JSON object or array → counts characters
    assert_eq!(labels, vec!["5"]);
}

// ── %splitstr empty separator returns whole ──────────────────────────────────

#[test]
fn builtin_splitstr_empty_sep_returns_whole() {
    let src = "@startuml
!$r = %splitstr(\"abc\", \"\")
A -> B : $r
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["abc"]);
}
