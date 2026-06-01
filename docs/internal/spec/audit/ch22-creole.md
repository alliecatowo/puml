# Chapter 22 — Creole Audit

Source: PlantUML Language Reference Guide 1.2025.0, Chapter 22
Last updated: 2026-06-01 (refreshed in #1502 — prior entries were stale)

Status legend: ✅ supported · 🟡 partial · ❌ not supported

Core implementation: `src/creole/` (split into `inline.rs`, `parser.rs`, `svg.rs`).

---

### 22.1 Emphasized text — ✅
**Feature:** `**bold**`, `//italic//`, `""monospaced""`, `--strikethrough--`,
`__underline__`, `~~wave-underline~~`.
**Status:** ✅ All six supported.
**Evidence:** `src/creole/inline.rs` — bold (`**`), italic (`//`), mono (`""`),
underline (`__`), strike (`--`), wave underline (`~~`). Tests in `src/creole/tests.rs`.

### 22.2 Lists — ✅
**Feature:** Bullet `*` and numbered `#` lists with nesting (`**`, `##`).
**Status:** ✅ Supported.
**Evidence:** `src/creole/parser.rs` `parse_list_line`. Depth-1 bullets use `- ` prefix;
numbered use `1. `; nesting increases indentation. Tests: `tests/creole_block_parity.rs`
and `tests/creole_block_level_1502.rs`.

### 22.3 Escape character (`~`) — ✅
**Feature:** `~` escapes the next creole metacharacter (e.g. `~**not bold**`).
**Status:** ✅ Supported.
**Evidence:** `src/creole/inline.rs` tilde-escape handling; test `tilde_escapes_creole_markers`
in `src/creole/tests.rs`.

### 22.4 Headings — ✅
**Feature:** Line-leading `=`, `==`, `===`, `====` indicate heading sizes.
**Status:** ✅ Supported (h1–h4 with bold + font-size escalation).
**Evidence:** `src/creole/parser.rs` `parse_heading_line`. Font sizes: h1=24, h2=21,
h3=18, h4=16. Tests: `src/creole/tests.rs` `headings_become_bold_sized_lines` and
`tests/creole_block_level_1502.rs`.

### 22.5 Emoji `<:name:>` and `<#color:name:>` — 🟡
**Feature:** Twemoji emoji via `<:1f600:>`, `<:innocent:>`, `<#green:sunny:>`. `emoji` listing command.
**Status:** 🟡 Partial — small deterministic subset + numeric hex codepoints decoded;
full PlantUML emoji catalog and colorized forms (`<#color:name:>`) partially supported;
`emoji` listing directive not implemented.
**Evidence:** `src/text_markup.rs` `decode_unicode_escapes`.

### 22.6 Horizontal lines (`----`, `====`, `____`, `..title..`) — ✅
**Feature:** Horizontal rules with optional title; works inside notes and creole text.
**Status:** ✅ Supported.
**Evidence:** `src/creole/parser.rs` `is_plain_horizontal_rule` / `parse_titled_rule_line`.
Plain rules (`----`, `====`, `____`) emit `is_hr` sentinel → SVG `<line>` element.
Titled variants (`.. Title ..`, `=== Title ===`) render as styled text.
Tests: `src/creole/tests.rs` `horizontal_rule_lines_render_as_rule_text`,
`tests/creole_block_level_1502.rs`.

### 22.7 Links `[[url]]` / `[[url label]]` / `[[url{tooltip} label]]` — ✅
**Feature:** Square-bracket URL link with optional label and optional `{tooltip}`.
**Status:** ✅ Supported (url, label, tooltip all handled).
**Evidence:** `src/creole/inline.rs` `[[...]]` branch; `src/creole/inline_helpers.rs`
`parse_link_inner`. Tooltip parses `{...}` and emits `<title>` in SVG. Tests:
`src/creole/tests.rs` `link_tooltip_renders_svg_title`.

### 22.8 `<code> ... </code>` — ✅
**Feature:** Verbatim code block tag (no syntax highlighting).
**Status:** ✅ Supported. Content is monospace, all markup disabled verbatim.
**Evidence:** `src/creole/inline.rs` `<code>` branch. Test:
`src/creole/tests.rs` `code_tag_is_verbatim_monospace`.

### 22.9 Tables (`|= header |`, `| cell |`, color cells `<#color>`, borders) — ✅
**Feature:** Creole pipe tables with header rows (`|= ... |`), cell colors
(`|<#FF8080> red |`), row colors (`<#yellow>| ... |`), text alignment.
**Status:** ✅ Supported.
**Evidence:** `src/creole/parser.rs` `parse_table_line`, `parse_row_background`,
`parse_cell_background`. Tests: `src/creole/tests.rs` `table_lines_mark_headers_*`,
`tests/creole_block_level_1502.rs`.

### 22.10 Tree (`|_`) — ✅
**Feature:** Tree bullets using `|_`.
**Status:** ✅ Supported.
**Evidence:** `src/creole/parser.rs` `parse_tree_line`. Emits monospace `` `- `` prefix.
Test: `src/creole/tests.rs` `tree_lines_use_text_tree_prefix`,
`tests/creole_block_level_1502.rs`.

### 22.11 Special characters (`<U+XXXX>`, `&#nnnn;`) — ✅
**Feature:** Unicode codepoint insertion via `<U+221E>` (hex) or `&#nnnnnn;` (decimal).
**Status:** ✅ Supported.
**Evidence:** `src/text_markup.rs` `decode_unicode_escapes`. Tests:
`src/creole/tests.rs` `decodes_decimal_and_hex_*`, `decodes_u_plus_codepoint_tags`.

### 22.12 Legacy HTML tags — ✅
All HTML extension tags documented in PlantUML Ch22 are now supported:

| Tag | Status | Evidence |
|-----|--------|----------|
| `<b>` / `</b>` | ✅ | `src/creole/inline.rs` |
| `<i>` / `</i>` | ✅ | `src/creole/inline.rs` |
| `<u>` / `<u:color>` | ✅ | `src/creole/inline.rs` |
| `<s>` / `<s:color>` | ✅ | `src/creole/inline.rs` |
| `<w>` / `<w:color>` | ✅ | `src/creole/inline.rs` |
| `<plain>` | ✅ | `src/creole/inline.rs` |
| `<color:X>` / `</color>` | ✅ | `src/creole/inline.rs` |
| `<back:X>` / `</back>` | ✅ | `src/creole/inline.rs` |
| `<size:N>` / `</size>` | ✅ | `src/creole/inline.rs` |
| `<font:Name>` / `</font>` | ✅ | `src/creole/inline.rs` |
| `<img:path>` / `<img:url>` | ✅ | `src/creole/inline.rs` (emits `[filename]` placeholder) |
| `<sub>` / `</sub>` | ✅ | `src/creole/inline.rs` |
| `<sup>` / `</sup>` | ✅ | `src/creole/inline.rs` |
| `<strong>` / `<em>` / `<del>` / `<strike>` / `<tt>` | ✅ | Alias mappings in `src/creole/inline.rs` |

### 22.13 OpenIconic `<&iconname>` — 🟡
**Status:** 🟡 (parsed as text placeholder `[iconname]`; icon glyphs not rendered)
**Evidence:** `src/creole/inline.rs` `<&...>` branch.
**Notes:** `listopeniconic` directive not implemented. Placeholder keeps diagrams readable.

### 22.14 Definition lists (`; Term : Definition`) — ✅
**Feature:** Creole-standard definition-list syntax.
**Status:** ✅ Supported (added in #1502). Term is bold; definition follows ` : ` separator
in normal weight. Both sides pass through inline markup. Term-only form (`; Term`)
also supported.
**Evidence:** `src/creole/parser.rs` `parse_definition_list_line`.
Tests: `tests/creole_block_level_1502.rs`.

### Line breaks (`\n`, `\\n`, `<br>`) — ✅
**Evidence:** `src/creole/parser.rs` `normalize_line_breaks`.

---

## Tally — Chapter 22

- ✅ Supported (26): all 6 emphasis forms, bullet+numbered lists, tilde escape, h1–h4
  headings, all horizontal rule forms, links with tooltip, `<code>` verbatim, pipe tables
  with header/cell/row colors, tree (`|_`), all Unicode escapes, all legacy HTML tags
  (`<b>`, `<i>`, `<u:color>`, `<s:color>`, `<w:color>`, `<plain>`, `<color>`, `<back>`,
  `<size>`, `<font>`, `<img>` placeholder, `<sub>`, `<sup>`, HTML aliases), and
  definition lists.
- 🟡 Partial (2): emoji (`<:name:>` — small deterministic subset, not full catalog;
  colorized form partially); OpenIconic `<&icon>` (placeholder, not rendered).
- ❌ Missing (3): full PlantUML emoji catalog/`emoji` listing directive,
  `listopeniconic` directive, `<img:>` actual image embedding (placeholder is fine for
  text-only renderers; blocked on bitmap compositing).

**Headline:** Creole engine is now near-complete for the PlantUML Ch22 spec. All block
constructs (lists, headings, tables, tree, horizontal rules, definition lists) and all
inline/HTML extension tags are implemented. Remaining gaps are emoji catalog completeness
and icon glyph rendering — both intentionally deferred.
