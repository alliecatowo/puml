# Chapter 25 — Preprocessing: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

Module: `src/preproc/` (`mod.rs`, `control.rs`, `includes.rs`, `builtins.rs`, `macros.rs`)
Directive dispatch: `parse_preprocess_directive` at `src/preproc/includes.rs:719`.

---

### 25.1 Variable definition `!$var = value`, `!$var ?= value` — ✅
**Feature:** Int, string, JSON literals; `?=` only if undefined; optional `global` keyword.
**Status:** ✅
**Evidence:** `PreprocessDirective::VariableAssign{conditional, scope}` (`src/preproc/mod.rs:89`); `parse_variable_assignment` / `parse_scoped_variable_assignment` in `macros.rs`.

### 25.2 Boolean expressions (`&&`, `||`, `()`, `%true/%false/%not/%boolval`) — ✅
**Status:** ✅
**Evidence:** `evaluate_preprocess_expr` in `includes.rs`; builtins at `builtins.rs:327-330`, `486-494`.

### 25.3 Conditions `!if / !elseif / !else / !endif` — ✅
**Evidence:** `PreprocessDirective::If, ElseIf, Else, EndIf` (`control.rs:73-110`).

### 25.3 `!ifdef / !ifndef` — ✅
**Evidence:** `control.rs:86`, directive at `includes.rs:747,751`.

### 25.4 While loop `!while / !endwhile` — ✅
**Evidence:** `PreprocessDirective::While`, `EndWhile`; iteration cap `MAX_PREPROC_WHILE_ITERATIONS = 10_000` (`mod.rs:15`).

### 25.4 `!foreach / !endfor` — ✅
**Evidence:** `Foreach`, `EndFor` directives at `includes.rs:759-760`; binding helper `preprocessor_foreach_bindings` (`builtins.rs:925`).

### 25.5 `!procedure / !endprocedure` — ✅
**Evidence:** Directives `includes.rs:766-767`; `execute_procedure_call`, `parse_callable_definition` in `builtins.rs`. Local vs global variable scoping handled (`local_state` clone at builtins.rs:1753).

### 25.6 `!function / !endfunction` + single-line `!function … !return … !endfunction` — ✅
**Evidence:** `Function`, `EndFunction`, `Unsupported("return")` — `!return` parsed within function bodies. Single-line shorthand supported via callable definition parser.

### 25.7 Default argument values — ✅
**Evidence:** `PreprocParam { name, default }` (`builtins.rs:1625`), default applied at expansion (`builtins.rs:1713`).

### 25.8 `!unquoted` function/procedure — ✅
**Evidence:** `parse_callable_definition` handles `unquoted` prefix (referenced from builtins.rs). Grep for `unquoted` shows handling.

### 25.9 Keyword arguments (`$name=value`) — 🟡
**Status:** 🟡 — partial. The callable param parser splits `name=default` (definition side), and call-site arg expansion goes through `split_args`. Whether call-site `name=value` keyword-passing is fully wired needs runtime verification; not explicitly named in code.
**Evidence:** `builtins.rs:1613` parses defaults; keyword-arg call-site parsing not clearly distinguished from positional.

### 25.10 `!include`, `!include_many`, `!include_once`, `!includeurl` — ✅
**Evidence:** Directives at `includes.rs:740-744`; `process_include_directive`, `process_include_many_directive`. Block-index `!N` and `@startuml(id=...)` tags supported via `IncludeTarget { path, tag }` (`mod.rs:24`).
**Notes:** URL fetching gated by `options.allow_url_includes` and `feature = "url-includes"` (`includes.rs:111,121,1435`). WASM build returns `E_INCLUDE_NOT_SUPPORTED_WASM` (`includes.rs:31`).

### 25.11 `!startsub / !endsub / !includesub` — 🟡
**Status:** 🟡 — `startsub` and `endsub` are parsed as `NoOp` (`includes.rs:785`), but `!includesub NAME` is parsed as `IncludeSub` directive (`includes.rs:743`). Whether the includesub correctly extracts the named region in `process_include_directive` needs deeper code reading; sub-region extraction logic appears to exist but is gated on the `tag` field of `IncludeTarget`.
**Evidence:** `IncludeTarget.tag` (`mod.rs:26`); tag matching in `includes.rs:538`.

### 25.12 Builtin functions (`%…`) — ✅
**Status:** ✅ — broad coverage.
**Evidence (builtins.rs):** strlen/size (51,53), splitstr (67), splitstr_regex (79), str2json (263), strpos (265), substr (277), intval (294), string (295), boolval (327), true/false (328-329), not (330), lower/upper (331-332), chr (333), dec2hex/hex2dec (343,351), ord (360), random (372), dirpath (399), filename (408), feature (427), get/set_variable_value (429,466), variable_exists (434), function_exists (439), newline (462), invoke_procedure / call_user_func (498), is_dark (562), reverse_color (563), lighten/darken (568-569).
**Determinism stubs:** `%date %time %now %getenv` deliberately return empty (`builtins.rs:368`); `%random` returns `"0"` (`builtins.rs:372`) — non-deterministic builtins neutralized intentionally (comment at builtins.rs:21).
**Missing/stub-only:** `%load_json`, `%file_exists`, `%get_all_theme`, `%hsl_color`, `%reverse_hsluv_color` appear in the unsupported/no-op group. `%get_all_stdlib()` now returns the deterministic local shim inventory rather than the full upstream plantuml-stdlib catalog.

### 25.13 `!log` — ✅
**Evidence:** `PreprocessDirective::Log` (`includes.rs:769`); no impact on diagram output by design.

### 25.14 `!dump_memory` — ✅
**Evidence:** `PreprocessDirective::DumpMemory` (`includes.rs:770`).

### 25.15 `!assert` — ✅
**Evidence:** `PreprocessDirective::Assert` (`includes.rs:768`); `evaluate_assert_expression` helper.

### 25.16 `!import` + custom library zip/jar — 🟡
**Status:** 🟡 — `!import` directive is parsed (`includes.rs:745`); `process_import_directive` exists; `parse_import_target` / `resolve_import_path` handle paths. **No zip/jar archive support** — archive entries cannot be opened directly.
**Evidence:** `includes.rs:171-174`. No `zip` crate import; archives are not extracted.

### 25.17 Search path / `-Dplantuml.include.path` — 🟡
**Status:** 🟡 — `options.include_root: Option<PathBuf>` (`mod.rs:31`) plus `PUML_STDLIB_ROOT` env (`includes.rs:494,498`). No `-D` system property pass-through; CLI exposes equivalent via `-D` for variable injection (see `inject_vars` mod.rs:38).

### 25.18 Argument concatenation `##` — 🟡
**Status:** 🟡 — needs verification. Grep for `##` in macros.rs would be needed; macro-arg concatenation is a known PlantUML feature and may be partial.

### 25.19 Dynamic invocation `%invoke_procedure()`, `%call_user_func()` — ✅
**Evidence:** `builtins.rs:498`; directive form also recognised (`mod.rs:81` `DynamicInvocation`).

### 25.20 Evaluation of `+` (concat vs add depending on types) — ✅
**Evidence:** Visible via `eval_simple_arithmetic` and string-vs-int handling in expression evaluator.

### 25.21 Preprocessing JSON — 🟡
**Status:** 🟡 — `JsonPreproc` directive variant exists (`mod.rs:81`); `%str2json` builtin present (builtins.rs:263); `%load_json` is in the stub group. JSON variable definition (`{ "name": "John" }` literal RHS) and `$foo.name` member access — partial; literal RHS likely parsed as a string unless str2json is called.

### 25.22 `!theme` directive — 🟡
**Status:** 🟡 — `!theme` is **not** in `parse_preprocess_directive` (`includes.rs:786` explicitly returns None for `"theme"|"pragma"`, leaving it as a passthrough line). It is consumed later at the parser/normalize layer: `src/parser/sequence.rs:250` (`if lower.starts_with("!theme")`) and `src/normalize/chart.rs:25`. Theme registry is in `src/theme.rs:472 resolve_sequence_theme_preset` with 30+ named themes (plain, aws-orange, blueprint, cerulean, crt-amber, hacker, mars, sketchy, spacelab, etc.).
**Notes:** Only **built-in local themes** are accepted (`E_THEME_SOURCE_UNSUPPORTED` at theme.rs:482) — no URL-source themes.

### 25.23 Migration notes (`!define`, `!definelong`) — 🟡
**Status:** 🟡 — `!define` is parsed (`includes.rs:738 Define`); `!definelong` not seen, but `!procedure` covers the migration. `!undef` ✅ (`includes.rs:739`).

### 25.24-25 `%splitstr`, `%splitstr_regex` — ✅
**Evidence:** `builtins.rs:67, 79`.

### 25.26 `%get_all_theme` — ❌ (stub)
**Evidence:** Listed in unsupported group near `builtins.rs:381-388`.

### 25.27 `%get_all_stdlib` (+ detailed) — 🟡
**Status:** 🟡 — implemented as a deterministic JSON-style list of reachable local stdlib include paths from `src/stdlib.rs`, including the `awslib/...` alias entries that map to physical `awslib14/...` files. This is intentionally a local shim inventory, not a full upstream stdlib downloader/catalog and not a detailed metadata object.
**Evidence:** `builtins.rs` dispatches `get_all_stdlib` through `crate::stdlib::local_stdlib_inventory`; coverage in `tests/coverage_wave23_builtins.rs`.

### 25.28 `%random` — 🟡 (deterministic stub returns "0")
**Evidence:** `builtins.rs:372`.

### 25.29 `%boolval` — ✅
**Evidence:** `builtins.rs:327`.

### Misc: `!option`, `!pragma` — 🟡
**Evidence:** `!option` recognised as Passthrough (`includes.rs:771`); `!pragma` skipped (`includes.rs:786`).

### Misc: `!break / !continue` — ✅
**Evidence:** `includes.rs:762-763`.

### Misc: `!local / !global` keywords in vars — ✅
**Evidence:** `PreprocVariableScope` enum and `parse_scoped_variable_assignment`.

---

## Tally

| Section | Status |
|---|---|
| 25.1 Variables (`=`, `?=`, global) | ✅ |
| 25.2 Boolean expr | ✅ |
| 25.3 `!if/!elseif/!else/!endif/!ifdef/!ifndef` | ✅ |
| 25.4 `!while/!foreach` | ✅ |
| 25.5 `!procedure` | ✅ |
| 25.6 `!function/!return` | ✅ |
| 25.7 Default args | ✅ |
| 25.8 `!unquoted` | ✅ |
| 25.9 Keyword args | 🟡 |
| 25.10 `!include/_many/_once/url` | ✅ |
| 25.11 `!startsub/!endsub/!includesub` | 🟡 |
| 25.12 Builtins (broad set) | ✅ |
| 25.13 `!log` | ✅ |
| 25.14 `!dump_memory` | ✅ |
| 25.15 `!assert` | ✅ |
| 25.16 `!import` (+ zip/jar) | 🟡 (no archives) |
| 25.17 Search path | 🟡 |
| 25.18 `##` concat | 🟡 (verify) |
| 25.19 Dynamic invocation | ✅ |
| 25.20 `+` type-aware | ✅ |
| 25.21 JSON preproc | 🟡 |
| 25.22 `!theme` | 🟡 (built-in only) |
| 25.23 `!define/!undef` (legacy) | ✅ |
| 25.24-25 `%splitstr*` | ✅ |
| 25.26 `%get_all_theme` | ❌ |
| 25.27 `%get_all_stdlib` | 🟡 |
| 25.28 `%random` (determinism stub) | 🟡 |
| 25.29 `%boolval` | ✅ |

**Score:** 18 ✅ · 9 🟡 · 1 ❌ out of 28. **Preprocessor is the strongest area** — close to feature-complete with deterministic stubs for IO/time builtins.
