# Chapter 22 — Creole Audit

Source: `/tmp/puml-spec/ch22-creole.txt` (1314 lines)
Spec date: PlantUML Language Reference Guide 1.2025.0

Status legend: ✅ supported · 🟡 partial · ❌ not supported

Core implementation: `src/creole.rs`. Module docstring:
> Supports: **bold**, //italic//, ""mono"", __underline__, --strikethrough--, [[url label]] hyperlinks, <color:X>text</color>, <size:N>text</size>, <b>, <i>, <u> HTML tags, \n line breaks, and <&icon> placeholders.

---

### 22.1 Emphasized text — 🟡
**Feature:** `**bold**`, `//italic//`, `""monospaced""`, `--strikethrough--`, `__underline__`, `~~wave-underline~~`.
**Syntax example:** `This is **bold**`
**Status:** 🟡 (5 of 6 — no wave-underline `~~...~~`)
**Evidence:** `src/creole.rs:238-278` (`**bold**`, `//italic//`, `""mono""`, `__underline__`, `--strike--`). No code for `~~...~~`.
**Notes:** Wave-underline is rendered in HTML via `<w>` tag in PlantUML — both creole `~~` and HTML `<w>` are missing.

### 22.2 Lists — ❌ (likely; not found in creole.rs)
**Feature:** Bullet `*` and numbered `#` lists with nesting (`**`, `##`).
**Syntax example:** `* Bullet list\n** Sub item\n# Numbered\n## Sub`
**Status:** ❌
**Evidence:** No list parsing in `src/creole.rs`. `**` is consumed as bold-toggle (`src/creole.rs:238`). At line-start `*` or `#` would not be recognized as list markers.
**Notes:** Major gap — lists are commonly used in notes/legends.

### 22.3 Escape character (`~`) — ❌
**Feature:** `~` escapes the next creole metacharacter (e.g. `~**not bold**`).
**Status:** ❌
**Evidence:** `~` not handled in `src/creole.rs` tokenizer; would be passed through as literal text.
**Notes:** Means `~~` is also not interpreted as wave; small upside.

### 22.4 Headings — ❌
**Feature:** Line-leading `=`, `==`, `===`, `====` indicate heading sizes.
**Status:** ❌
**Evidence:** No heading parsing in creole or text renderer.
**Notes:** Spec uses inside usecase/note text.

### 22.5 Emoji `<:name:>` and `<#color:name:>` — 🟡
**Feature:** Twemoji emoji via `<:1f600:>`, `<:innocent:>`, `<#green:sunny:>`, `<#0:sunglasses:>`. `emoji <block>` listing command.
**Status:** 🟡 Partial
**Evidence:** `src/creole.rs` decodes a small deterministic emoji subset plus numeric hex emoji tags such as `<:1f600:>`.
**Notes:** Full PlantUML emoji catalog, colorized emoji forms, and the `emoji` listing directive remain missing.

### 22.6 Horizontal lines (`----`, `====`, `____`, `..title..`) — ❌
**Feature:** Inline horizontal rules with optional title; works inside notes and creole text.
**Status:** ❌
**Evidence:** No matches for horizontal-rule parsing in `src/creole.rs` or `src/render/text.rs`. (`text.rs` mentions `tree_branch`/`tree_leaf` only as character constants — unrelated.)

### 22.7 Links `[[url]]` / `[[url label]]` / `[[url{tooltip} label]]` — 🟡
**Feature:** Square-bracket URL link with optional label and optional `{tooltip}`.
**Status:** 🟡 (url + label yes; `{tooltip}` not handled)
**Evidence:** `src/creole.rs:278+` handles `[[url]]` and `[[url label]]`. Test cases at `src/creole.rs:560,568`. SVG anchor emission `src/creole.rs:56-60` (xlink:href).
**Notes:** No `{tooltip}` parsing detected — would land in label or fail.

### 22.8 `<code> ... </code>` — ❌
**Feature:** Verbatim code block tag (no syntax highlighting).
**Status:** ❌
**Evidence:** No `<code>` tag handling in `src/creole.rs`.
**Notes:** Likely renders the literal `<code>` text.

### 22.9 Tables (`|= header |`, `| cell |`, color cells `<#color>`, borders) — ❌
**Feature:** Creole pipe tables with header rows (`|= ... |`), cell colors (`|<#FF8080> red |`), row colors (`<#yellow>| ... |`), text alignment.
**Status:** ❌
**Evidence:** No `|=` / `|` table tokenizer in `src/creole.rs` or `src/render/text.rs`.
**Notes:** Major gap — used heavily for class-field alignment.

### 22.10 Tree (`|_`) — ❌
**Feature:** Tree bullets using `|_`.
**Status:** ❌
**Evidence:** No `|_` parsing in creole.
**Notes:** Note: `src/render/text.rs` has `tree_branch`/`tree_leaf` but those are mindmap rendering helpers, not creole `|_`.

### 22.11 Special characters (`<U+XXXX>`, `&#nnnn;`) — ✅
**Feature:** Unicode codepoint insertion via `<U+221E>` (hex) or `&#nnnnnn;` (decimal).
**Status:** ✅ Supported
**Evidence:** `decode_unicode_escapes` in `src/creole.rs` decodes decimal / hex numeric entities and PlantUML `<U+...>` tags before tokenization. Covered by `decodes_numeric_character_references`, `decodes_plantuml_u_plus_tags`, and integration coverage around `valid_unicode_escapes.puml`.
**Notes:** Invalid or out-of-range codepoints remain literal, which keeps malformed input deterministic.

### 22.12 Legacy HTML tags — 🟡

#### 22.12.x `<b>` — ✅
**Evidence:** `src/creole.rs:372-373`.

#### 22.12.x `<i>` — ✅
**Evidence:** `src/creole.rs:388-389`.

#### 22.12.x `<u>` / `<u:color>` — 🟡
**Status:** 🟡 (`<u>` yes; `<u:color>` not detected)
**Evidence:** `src/creole.rs:404-405`. No `<u:#...>` colored variant.

#### 22.12.x `<s>` / `<s:color>` strike — ❌
**Evidence:** No `<s>` handling.
**Notes:** Creole `--strike--` works but the HTML tag does not.

#### 22.12.x `<w>` / `<w:color>` wave — ❌
**Evidence:** No `<w>` handling.

#### 22.12.x `<plain>` — ❌
**Evidence:** No `<plain>` handling.

#### 22.12.x `<color:X>...</color>` — ✅
**Evidence:** `src/creole.rs:337+`.

#### 22.12.x `<back:X>...</back>` background — ❌
**Evidence:** No `<back:` handling.

#### 22.12.x `<size:N>...</size>` — ✅
**Evidence:** `src/creole.rs:354+`.

#### 22.12.x `<font:Name>...</font>` font-family — ❌
**Evidence:** No `<font:` handling.

#### 22.12.x `<img:path>` / `<img:url>` / `{scale=0.3}` — ❌
**Evidence:** No `<img:` handling in creole.

#### 22.12.2 `<sub>` / `<sup>` — ❌
**Evidence:** No subscript/superscript handling.

### 22.13 OpenIconic `<&iconname>` — 🟡
**Status:** 🟡 (parsed as placeholder, but icons not rendered)
**Evidence:** `src/creole.rs:319-322` (`<&icon>` recognized as span). Module docstring says "icon placeholders". Test `src/creole.rs:631`.
**Notes:** Likely emits the icon name as text or empty — actual icon glyph rendering not confirmed. `listopeniconic` directive not implemented.

### 22.14+ List rendering on all diagrams — ❌
**Status:** ❌ (follows from 22.2)
**Evidence:** No list machinery; per-diagram list rendering thus absent.

### Line breaks (`\n`, `\\n`, `<br>`) — ✅
**Evidence:** `src/creole.rs:31` `normalize_line_breaks`, handles `\\n` and `<br>`/`<br/>`. Tspan multi-line render at `src/creole.rs:131-145`.

---

## Tally — Chapter 22

- ✅ Supported: `**bold**`, `//italic//`, `""mono""`, `__underline__`, `--strike--`, `[[url label]]`, `<b>`, `<i>`, `<u>` (basic), `<color:X>`, `<size:N>`, line breaks (`\n`, `<br>`), numeric entities, and `<U+...>` codepoint escapes
- 🟡 Partial: emphasized text (missing `~~wave~~`), `[[url{tooltip} label]]` tooltip, OpenIconic `<&icon>` (parsed but rendering unconfirmed), and emoji tags (small deterministic subset, not the full PlantUML catalog/directive)
- ❌ Missing (16+): lists (`*`/`#`), `~` escape, headings (`=`, `==`, ...), full `emoji` directive/catalog parity, horizontal lines (`----`/`====`/`____`/`..title..`), `<code>`, tables (`|= |`, `|`, `<#color>` cells), tree `|_`, `<s>`, `<w>`, `<plain>`, `<back:>`, `<font:>`, `<img:>`, `<sub>`/`<sup>`, `<u:color>`, `listopeniconic`

**Headline:** Creole engine covers the core inline formatting set (bold/italic/mono/underline/strike, color, size, simple links, line breaks, basic `<b>/<i>/<u>` HTML) plus numeric / `<U+...>` Unicode escapes and a small deterministic emoji subset. It is still missing all block-level Creole (lists, headings, tables, tree, horizontal rules), most HTML extension tags (`<s>`, `<w>`, `<plain>`, `<back>`, `<font>`, `<img>`, `<sub>`, `<sup>`), and full PlantUML emoji catalog/directive parity.
