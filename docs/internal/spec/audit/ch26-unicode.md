# Chapter 26 — Unicode: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

### 26.1 UTF-8 letters in identifiers (CJK, etc.) — ✅
**Feature:** Actor/participant/use-case names can be any Unicode letters (e.g. `actor 使用者`, `participant "頭等艙" as A`).
**Status:** ✅
**Evidence:** Rust source is UTF-8 by default; parser identifier rules in `src/parser/` use char-based matching (no ASCII-only restriction). Source loaded as `String` (Rust UTF-8) throughout `src/source.rs`. No ASCII gates.
**Notes:** Rendering depends on the embedded font having those glyphs — this is a runtime/font concern not a parser concern.

### 26.2 Charset (`-charset` CLI) — 🟡
**Feature:** PlantUML accepts `-charset UTF-8`, `ISO-8859-1`, `UTF-16BE`, `UTF-16LE`, `UTF-16`.
**Syntax example:** `java -jar plantuml.jar -charset UTF-8 files.txt`
**Status:** 🟡 — `--charset` CLI flag is present but treated as a **no-op** compatibility shim accepting only UTF-8.
**Evidence:** `src/cli.rs:139-141`:
```
/// No-op compatibility flag (only UTF-8 input is supported).
#[arg(long, value_name = "CHARSET", default_value = "UTF-8")]
pub charset: String,
```
Assertion at `src/cli.rs:281` confirms default.
**Notes:** Non-UTF-8 inputs are not transcoded; this is a deliberate simplification.

### 26.3 Special characters `&#XXXX;` — ✅
**Feature:** HTML-style numeric entity `&#1234;` decoded to the corresponding Unicode codepoint.
**Status:** ✅ Supported
**Evidence:** `src/creole.rs` implements `decode_unicode_escapes`, including decimal `&#8734;` and hex `&#x221E;` numeric references. Unit tests cover valid and malformed forms.
**Notes:** Invalid numeric entities remain literal rather than erroring.

### 26.3 `<U+XXXX>` form — ✅
**Feature:** Inline a codepoint via `<U+2603>` (snowman).
**Status:** ✅ Supported
**Evidence:** `decode_unicode_escapes` recognizes `<U+...>` and `<u+...>` codepoint tags before Creole tokenization. Covered by `decodes_plantuml_u_plus_tags` and `tests/fixtures/conformance/valid_unicode_escapes.puml`.

### 26.3 Emoji `<:NameOfEmoji:>` / `<:XXXXX:>` — 🟡
**Feature:** Inline a named emoji (e.g. `<:smile:>`) — PlantUML ships with an emoji catalogue.
**Status:** 🟡 Partial
**Evidence:** `src/creole.rs` decodes a deterministic small emoji subset and numeric hex emoji tags such as `<:1f600:>`, with tests in `decodes_small_emoji_map_and_deterministic_fallback`. `docs/examples/creole/05_ch22_parity.puml` demonstrates the supported form.
**Notes:** PUML does not ship PlantUML's full emoji catalog or `emoji` listing directive yet.

---

## Tally

| Feature | Status |
|---|---|
| UTF-8 identifiers / labels | ✅ |
| `-charset` CLI flag | 🟡 (no-op, UTF-8 only) |
| `&#XXXX;` numeric entity | ✅ |
| `<U+XXXX>` codepoint escape | ✅ |
| `<:emoji:>` named emoji | 🟡 |

**Score:** 3 ✅ · 2 🟡 · 0 ❌ out of 5. Native UTF-8 works because Rust strings are UTF-8; explicit numeric and `<U+...>` escape forms now decode, while charset transcoding and full PlantUML emoji catalog parity remain partial.
