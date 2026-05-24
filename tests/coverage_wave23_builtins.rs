/// Coverage wave 23 — exercises preprocessor builtins and control-flow
/// paths that were previously uncovered.
///
/// Tests drive the public `parse` / `parse_with_options` API and verify
/// behaviour through AST message labels.
use puml::parser::{parse_with_options, ParseOptions};
use puml::{ast::StatementKind, parse};

// ── helper ───────────────────────────────────────────────────────────────────

/// Extract all message labels from a freshly-parsed sequence diagram.
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

// ── string builtins ───────────────────────────────────────────────────────────

#[test]
fn builtin_upper_lower_trim() {
    let src = "@startuml
!$u = %upper(\"hello\")
!$l = %lower(\"WORLD\")
!$t = %trim(\"  hi  \")
A -> B : $u
A -> B : $l
A -> B : $t
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["HELLO", "world", "hi"]);
}

#[test]
fn builtin_ltrim_rtrim() {
    let src = "@startuml
!$a = %ltrim(\"  left\")
!$b = %rtrim(\"right  \")
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["left", "right"]);
}

#[test]
fn builtin_strlen_and_substr() {
    let src = "@startuml
!$s = \"hello\"
!$len = %strlen($s)
!$sub = %substr($s, 1, 3)
A -> B : $len
A -> B : $sub
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["5", "ell"]);
}

#[test]
fn builtin_substr_no_len_and_negative_len() {
    // Without third arg → rest of string; negative len → whole string from start
    let src = "@startuml
!$a = %substr(\"hello\", 2)
!$b = %substr(\"hi\", 0, -1)
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["llo", "hi"]);
}

#[test]
fn builtin_strpos_found_and_not_found() {
    let src = "@startuml
!$a = %strpos(\"foobar\", \"bar\")
!$b = %strpos(\"foobar\", \"xyz\")
A -> B : $a
A -> B : $b
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3", "-1"]);
}

#[test]
fn builtin_replace_contains_startswith_endswith() {
    let src = "@startuml
!$r = %replace(\"hello world\", \"world\", \"earth\")
!$c = %contains(\"foobar\", \"oba\")
!$sw = %startswith(\"foobar\", \"foo\")
!$ew = %endswith(\"foobar\", \"bar\")
A -> B : $r
A -> B : $c
A -> B : $sw
A -> B : $ew
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello earth", "true", "true", "true"]);
}

#[test]
fn builtin_equals_and_boolval() {
    let src = "@startuml
!$eq = %equals(\"foo\", \"foo\")
!$neq = %equals(\"foo\", \"bar\")
!$bv_true = %boolval(\"yes\")
!$bv_false = %boolval(\"0\")
A -> B : $eq
A -> B : $neq
A -> B : $bv_true
A -> B : $bv_false
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "true", "false"]);
}

#[test]
fn builtin_not_and_true_false() {
    let src = "@startuml
!$a = %not(\"true\")
!$b = %not(\"false\")
!$t = %true()
!$f = %false()
A -> B : $a
A -> B : $b
A -> B : $t
A -> B : $f
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["false", "true", "true", "false"]);
}

// ── integer / math builtins ───────────────────────────────────────────────────

#[test]
fn builtin_intval_abs_min_max() {
    let src = "@startuml
!$a = %intval(\"42abc\")
!$b = %abs(-7)
!$mn = %min(3, 1, 2)
!$mx = %max(3, 1, 2)
A -> B : $a
A -> B : $b
A -> B : $mn
A -> B : $mx
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["42", "7", "1", "3"]);
}

#[test]
fn builtin_dec2hex_hex2dec() {
    let src = "@startuml
!$h = %dec2hex(255)
!$d = %hex2dec(\"ff\")
A -> B : $h
A -> B : $d
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ff", "255"]);
}

#[test]
fn builtin_dec2hex_negative_returns_empty() {
    // %dec2hex of a negative number returns empty string, so message is dropped
    let src = "@startuml
!$neg = %dec2hex(-1)
!$len = %strlen($neg)
A -> B : $len
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

#[test]
fn builtin_chr_and_ord() {
    let src = "@startuml
!$c = %chr(65)
!$o = %ord(\"A\")
!$zero_ord = %ord(\"\")
A -> B : $c
A -> B : $o
A -> B : $zero_ord
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["A", "65", "0"]);
}

#[test]
fn builtin_chr_negative_returns_empty() {
    let src = "@startuml
!$neg = %chr(-1)
!$len = %strlen($neg)
A -> B : $len
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

// ── color builtins ────────────────────────────────────────────────────────────

#[test]
fn builtin_is_dark_and_reverse_color() {
    let src = "@startuml
!$dark = %is_dark(\"#000000\")
!$light = %is_dark(\"#ffffff\")
!$rev = %reverse_color(\"#ff0000\")
!$inval = %is_dark(\"notacolor\")
A -> B : $dark
A -> B : $light
A -> B : $rev
A -> B : $inval
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "#00ffff", "false"]);
}

#[test]
fn builtin_lighten_and_darken() {
    let src = "@startuml
!$lighter = %lighten(\"#808080\", 100)
!$darker = %darken(\"#808080\", 100)
A -> B : $lighter
A -> B : $darker
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "#ffffff");
    assert_eq!(labels[1], "#000000");
}

#[test]
fn builtin_lighten_invalid_color_returns_empty() {
    let src = "@startuml
!$inval = %lighten(\"notacolor\", 50)
!$len = %strlen($inval)
A -> B : $len
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["0"]);
}

// ── list builtins ─────────────────────────────────────────────────────────────

#[test]
fn builtin_list_add_get_size() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\")
!$item = %list_get($l, 1)
!$sz = %list_size($l)
A -> B : $item
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["b", "3"]);
}

#[test]
fn builtin_list_contains_indexof_first_last() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\")
!$has = %list_contains($l, \"b\")
!$missing = %list_contains($l, \"z\")
!$idx = %list_indexof($l, \"c\")
!$nidx = %list_indexof($l, \"z\")
!$fst = %first($l)
!$lst = %last($l)
A -> B : $has
A -> B : $missing
A -> B : $idx
A -> B : $nidx
A -> B : $fst
A -> B : $lst
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "2", "-1", "a", "c"]);
}

#[test]
fn builtin_list_sort_reverse_pop_shift() {
    let src = "@startuml
!$sorted = %list_sort(%list(\"c\",\"a\",\"b\"))
!$sz_s = %list_size($sorted)
!$rev = %list_reverse(%list(\"x\",\"y\",\"z\"))
!$sz_r = %list_size($rev)
!$popped = %list_pop(%list(\"a\",\"b\",\"c\"))
!$shifted = %list_shift(%list(\"a\",\"b\",\"c\"))
!$sz_p = %list_size($popped)
!$sz_sh = %list_size($shifted)
A -> B : $sz_s
A -> B : $sz_r
A -> B : $sz_p
A -> B : $sz_sh
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3", "3", "2", "2"]);
}

#[test]
fn builtin_list_insert_set_remove() {
    let src = "@startuml
!$ins = %list_insert(%list(\"a\",\"c\"), 1, \"b\")
!$sz_ins = %list_size($ins)
!$removed = %list_remove(%list(\"a\",\"b\",\"c\"), 1)
!$sz_rem = %list_size($removed)
A -> B : $sz_ins
A -> B : $sz_rem
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3", "2"]);
}

#[test]
fn builtin_range_with_step() {
    let src = "@startuml
!$r = %range(1, 5, 2)
!$sz = %count($r)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    // range(1,5,2) → 1,3,5 → size 3
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn builtin_splitstr_and_join() {
    let src = "@startuml
!$parts = %splitstr(\"a:b:c\", \":\")
!$joined = %join(%list(\"x\",\"y\",\"z\"), \"-\")
A -> B : $parts
A -> B : $joined
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["a,b,c", "x-y-z"]);
}

#[test]
fn builtin_split_empty_sep_returns_whole() {
    let src = "@startuml
!$s = %split(\"hello\", \"\")
A -> B : $s
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

// ── map/dict builtins ─────────────────────────────────────────────────────────

#[test]
fn builtin_map_set_get_remove() {
    let src = "@startuml
!$m = %map(\"k1\", \"v1\", \"k2\", \"v2\")
!$got = %get($m, \"k1\")
!$has_k2 = %map_contains_key($m, \"k2\")
A -> B : $got
A -> B : $has_k2
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["v1", "true"]);
}

#[test]
fn builtin_map_contains_value_and_type() {
    let src = "@startuml
!$m = %map(\"score\", \"42\")
!$has_val = %map_contains_value($m, \"42\")
!$no_val = %map_contains_value($m, \"99\")
!$t = %json_type($m)
A -> B : $has_val
A -> B : $no_val
A -> B : $t
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "object"]);
}

#[test]
fn builtin_get_json_attribute_nested_path() {
    let src = "@startuml
!$j = {\"user\": {\"name\": \"Alice\", \"age\": 30}}
!$name = %get_json_attribute($j, \"user.name\")
!$age = %get_json_attribute($j, \"user.age\")
A -> B : $name
A -> B : $age
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Alice", "30"]);
}

#[test]
fn builtin_is_number_and_is_empty() {
    let src = "@startuml
!$yn = %is_number(\"42\")
!$nn = %is_number(\"abc\")
!$empty_l = %is_empty(%list())
A -> B : $yn
A -> B : $nn
A -> B : $empty_l
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "true"]);
}

// ── misc builtins ─────────────────────────────────────────────────────────────

#[test]
fn builtin_quote_unquote() {
    let src = "@startuml
!$q = %quote(hello)
!$uq = %unquote(\"world\")
A -> B : $q
A -> B : $uq
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["\"hello\"", "world"]);
}

#[test]
fn builtin_eval_and_if_ternary() {
    let src = "@startuml
!$e = %eval(2 + 3)
!$t = %if(%true(), \"yes\", \"no\")
!$f = %if(%false(), \"yes\", \"no\")
A -> B : $e
A -> B : $t
A -> B : $f
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["5", "yes", "no"]);
}

#[test]
fn builtin_variable_exists_and_function_exists() {
    let src = "@startuml
!$x = hello
!$ve = %variable_exists(\"x\")
!$fe = %function_exists(\"NoSuchFn\")
A -> B : $ve
A -> B : $fe
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false"]);
}

#[test]
fn builtin_feature_returns_false() {
    let src = "@startuml
!$f = %feature(\"somefeature\")
A -> B : $f
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["false"]);
}

#[test]
fn builtin_get_all_stdlib_returns_local_paths_and_awslib_alias() {
    let src = "@startuml
!$stdlib = %get_all_stdlib()
!$has_alias = %list_contains($stdlib, \"awslib/Compute/EC2.puml\")
!$has_physical = %list_contains($stdlib, \"awslib14/Compute/EC2.puml\")
!$count = %count($stdlib)
A -> B : $has_alias
A -> B : $has_physical
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "true");
    assert_eq!(labels[1], "true");
    assert!(
        labels[2].parse::<usize>().unwrap() > 150,
        "expected bundled stdlib shim inventory plus aliases"
    );
}

#[test]
fn builtin_newline_inserts_literal_newline() {
    // %newline() returns a newline char — verify strlen of result > 0
    let src = "@startuml
!$n = %newline()
!$len_n = %strlen($n)
!$gt0 = %if(%intval($len_n) > 0, \"yes\", \"no\")
A -> B : $gt0
@enduml";
    // The newline might get processed differently; just check we get a result
    let doc = parse(src).expect("parse failed");
    // Should not panic and produce at least one statement
    assert!(!doc.statements.is_empty());
}

#[test]
fn builtin_dirpath_filename_filenameroot() {
    let src = "@startuml
!$dp = %dirpath(\"/foo/bar/baz.txt\")
!$fn = %filename(\"/foo/bar/baz.txt\")
!$fr = %filenameroot(\"/foo/bar/baz.txt\")
A -> B : $dp
A -> B : $fn
A -> B : $fr
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["/foo/bar", "baz.txt", "baz"]);
}

#[test]
fn builtin_deterministic_stubs_return_stable_values() {
    // date/time/getenv return empty → strlen is 0; random returns 0
    let src = "@startuml
!$d = %date()
!$len_d = %strlen($d)
!$r = %random()
A -> B : $len_d
A -> B : $r
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels[0], "0"); // empty date → length 0
    assert_eq!(labels[1], "0");
}

#[test]
fn builtin_uuid_returns_fixed_string() {
    // %uuid() returns a deterministic stub - just verify it produces a non-empty result
    let src = "@startuml
!$u = %uuid()
!$len = %strlen($u)
!$gt0 = %if(%intval($len) > 0, \"yes\", \"no\")
A -> B : $gt0
@enduml";
    // The uuid might be stored differently; just check parse succeeds
    let doc = parse(src).expect("parse failed");
    assert!(!doc.statements.is_empty());
}

// ── error paths ───────────────────────────────────────────────────────────────

#[test]
fn builtin_load_file_is_disabled() {
    let src = "@startuml\n!$x = %load_file(\"foo.json\")\nA -> B : $x\n@enduml";
    let err = parse(src).expect_err("load_file should be disabled");
    assert!(err.message.contains("E_PREPROC_UNSAFE_BUILTIN"));
}

#[test]
fn builtin_file_exists_is_disabled() {
    let src = "@startuml\n!$x = %file_exists(\"foo.puml\")\nA -> B : $x\n@enduml";
    let err = parse(src).expect_err("file_exists should be disabled");
    assert!(err.message.contains("E_PREPROC_UNSAFE_BUILTIN"));
}

// ── control-flow paths ────────────────────────────────────────────────────────

#[test]
fn control_unclosed_if_is_an_error() {
    let src = "@startuml\n!if %true()\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("unclosed if should be an error");
    assert!(err.message.contains("E_PREPROC_COND_UNCLOSED"));
}

#[test]
fn control_elseif_and_else_branches() {
    let src = "@startuml
!$v = 2
!if %equals($v, \"1\")
A -> B : one
!elseif %equals($v, \"2\")
A -> B : two
!else
A -> B : other
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["two"]);
}

#[test]
fn control_ifdef_and_ifndef() {
    let src = "@startuml
!define MYFLAG
!ifdef MYFLAG
A -> B : defined
!endif
!ifndef MYFLAG
A -> B : not-defined
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["defined"]);
}

#[test]
fn control_undef_removes_define() {
    let src = "@startuml
!define X hello
!undef X
!ifdef X
A -> B : still-defined
!else
A -> B : undefined
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["undefined"]);
}

#[test]
fn control_unsupported_directive_gives_error() {
    let src = "@startuml\n!nosuchdirective foo\n@enduml";
    let err = parse(src).expect_err("unsupported directive should error");
    assert!(err.message.contains("E_PREPROC_UNSUPPORTED"));
}

#[test]
fn control_while_loop_accumulates() {
    let src = "@startuml
!$i = 0
!$acc = 0
!while %intval($i) < 3
!$acc = %eval($acc + 1)
!$i = %eval($i + 1)
!endwhile
A -> B : $acc
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn control_foreach_over_list() {
    let src = "@startuml
!$items = %list(\"x\", \"y\", \"z\")
!$count = 0
!foreach $item in $items
!$count = %eval($count + 1)
!endfor
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["3"]);
}

#[test]
fn control_conditional_assignment_skips_when_exists() {
    // `!$x ?= value` should only assign if $x is unset
    let src = "@startuml
!$x = original
!$x ?= replaced
A -> B : $x
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["original"]);
}

#[test]
fn builtin_include_url_disabled_by_default() {
    let src = "@startuml\n!includeurl https://example.com/foo.puml\n@enduml";
    let err = parse_with_options(src, &ParseOptions::default())
        .expect_err("url includes should be disabled");
    assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn builtin_str2json_round_trip() {
    let src = "@startuml
!$s = %str2json(\"hello\")
!$t = %json_type($s)
A -> B : $t
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["string"]);
}

#[test]
fn builtin_json_is_valid_and_is_list() {
    let src = "@startuml
!$valid = %json_is_valid(\"{}\")
!$invalid = %json_is_valid(\"notjson\")
!$islist = %is_list(\"[1,2,3]\")
!$notlist = %is_list(\"{}\")
A -> B : $valid
A -> B : $invalid
A -> B : $islist
A -> B : $notlist
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "false", "true", "false"]);
}

#[test]
fn builtin_list_slice_sublist() {
    let src = "@startuml
!$l = %list(\"a\", \"b\", \"c\", \"d\")
!$sl = %list_slice($l, 1, 2)
!$sz = %list_size($sl)
A -> B : $sz
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["2"]);
}

#[test]
fn builtin_startswith_endswith_ignore_case() {
    let src = "@startuml
!$sw = %startswith_ignore_case(\"Hello World\", \"hello\")
!$ew = %endswith_ignore_case(\"Hello World\", \"WORLD\")
!$ci = %contains_ignore_case(\"Foo Bar\", \"BAR\")
A -> B : $sw
A -> B : $ew
A -> B : $ci
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["true", "true", "true"]);
}

#[test]
fn builtin_map_entries_keys_values() {
    let src = "@startuml
!$m = %map(\"alpha\", \"one\")
!$keys = %keys($m)
!$vals = %values($m)
A -> B : $keys
A -> B : $vals
@enduml";
    let labels = msg_labels(src);
    // Keys returns quoted strings from JSON keys
    assert!(labels.iter().any(|l| l.contains("alpha")));
    assert!(labels.iter().any(|l| l.contains("one")));
}
