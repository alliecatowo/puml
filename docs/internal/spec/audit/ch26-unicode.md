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

### 26.3 Special characters `&#XXXX;` — ❌
**Feature:** HTML-style numeric entity `&#1234;` decoded to the corresponding Unicode codepoint.
**Status:** ❌
**Evidence:** Grep for `&#[0-9]|&#x` in render/normalize layers turns up only one match in `src/creole.rs:478` which is the **output-side** escaping of `'` to `&#39;` — not an **input-side** entity decoder. No general numeric character reference parser found.
**Notes:** Effect: `&#9728;` (☀) will likely appear as the literal seven characters in output.

### 26.3 `<U+XXXX>` form — ❌
**Feature:** Inline a codepoint via `<U+2603>` (snowman).
**Status:** ❌
**Evidence:** Grep for `U\+|U\\+` in src/ returns no matches. The spec example explicitly uses `<U+0025>` as a percent-sign escape; PUML does not interpret this — it will appear literally.

### 26.3 Emoji `<:NameOfEmoji:>` / `<:XXXXX:>` — ❌
**Feature:** Inline a named emoji (e.g. `<:smile:>`) — PlantUML ships with an emoji catalogue.
**Status:** ❌
**Evidence:** Grep for `<:` in src/ returns nothing. No emoji catalogue, no `<:name:>` lexer.

---

## Tally

| Feature | Status |
|---|---|
| UTF-8 identifiers / labels | ✅ |
| `-charset` CLI flag | 🟡 (no-op, UTF-8 only) |
| `&#XXXX;` numeric entity | ❌ |
| `<U+XXXX>` codepoint escape | ❌ |
| `<:emoji:>` named emoji | ❌ |

**Score:** 1 ✅ · 1 🟡 · 3 ❌ out of 5. Native UTF-8 works because Rust strings are UTF-8; **all three special-character escape forms are missing**, including the spec's own preferred `<U+0025>` form for percent-sign.
