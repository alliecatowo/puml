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
**Status:** ✅
**Evidence:** `decode_unicode_escapes` handles decimal and hex numeric references (`src/creole.rs:23-32`, `src/creole.rs:503-523`), and `escape_text` runs that decoder before XML escaping (`src/render/svg.rs:125-138`). Integration coverage checks `&#8734;` and `&#x2603;` decode and disappear as literal escape text (`tests/integration.rs:6547-6574`).
**Notes:** Invalid numeric references stay literal and XML-escaped (`tests/integration.rs:6576-6579`).

### 26.3 `<U+XXXX>` form — ✅
**Feature:** Inline a codepoint via `<U+2603>` (snowman).
**Status:** ✅
**Evidence:** `decode_codepoint_tag` handles case-insensitive `<U+...>` tags with 1-6 hex digits and rejects invalid/out-of-range values (`src/creole.rs:525-542`, `src/creole.rs:588-590`). Tests cover `<U+221E>` decoding in sequence labels and family plain labels (`tests/integration.rs:6547-6574`, `tests/integration.rs:6591-6601`).

### 26.3 Emoji `<:NameOfEmoji:>` / `<:XXXXX:>` — 🟡
**Feature:** Inline a named emoji (e.g. `<:smile:>`) — PlantUML ships with an emoji catalogue.
**Status:** 🟡
**Evidence:** `decode_emoji_tag` recognizes `<:...:>` tags, supports hex codepoint emoji (`<:1f600:>`), and maps a small deterministic named subset (`calendar`, `check`, `smile`, `heart`, `sun`, etc.) in `src/creole.rs:544-586`. Tests assert `<:calendar:>` and `<:1f600:>` render decoded while unknown safe names degrade to `:name:` (`tests/integration.rs:6547-6556`).
**Notes:** This is not full PlantUML emoji-catalogue parity; unknown names are not looked up from a bundled emoji list.

---

## Tally

| Feature | Status |
|---|---|
| UTF-8 identifiers / labels | ✅ |
| `-charset` CLI flag | 🟡 (no-op, UTF-8 only) |
| `&#XXXX;` numeric entity | ✅ |
| `<U+XXXX>` codepoint escape | ✅ |
| `<:emoji:>` named emoji | 🟡 (small built-in subset + hex codepoints) |

**Score:** 3 ✅ · 2 🟡 · 0 ❌ out of 5. Native UTF-8 works because Rust strings are UTF-8; numeric references and `<U+...>` are decoded across regular SVG text, while `-charset` remains UTF-8-only and emoji support is a small deterministic subset rather than PlantUML's full catalogue.
