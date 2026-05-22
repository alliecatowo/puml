# Chapter 23 — Sprites: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

### 23 Sprite definition (hex grid) — ❌
**Feature:** Define a monochrome sprite via 4/8/16-gray hex grid: `sprite $name { ... }`
**Syntax example:**
```
sprite $foo1 {
  FFFFFFFFFFFFFFF
  F0123456789ABCF
  ...
}
```
**Status:** ❌
**Evidence:** No general `sprite $name { ... }` parser/AST. The only `Sprite*` symbols are Salt-mockup placeholders at `src/render/salt.rs:425,426,721,740` (just records the name and draws a dashed-rect stub). No global sprite registry, no hex grid decoder, no rendering of the bitmap.
**Notes:** Lexically, the keyword `sprite` is recognised only inside Salt widget grids. Outside Salt, the line is treated as a generic statement and likely dropped.

### 23 Sprite reference `<$name>` — ❌
**Feature:** Embed sprite in text/label as `<$name>` and inline parameters: `<$name{scale=3}>`, `<$name*3>`, `<$name,scale=3,color=orange>`
**Syntax example:** `Alice -> Bob : Testing <$foo1{scale=3,color=orange}>`
**Status:** ❌
**Evidence:** No `<$...>` lexer/expander outside of Salt's `parse_salt_sprite_ref` (`src/render/salt.rs:740`) which only matches inside Salt cells and renders a stub. Grep for `<\$` returns no creole/text-layer matches.
**Notes:** Creole renderer (`src/creole.rs`) has no sprite token handler — `<$name>` will leak through to output as literal text or be HTML-escaped.

### 23.1 Inline SVG sprite — ❌
**Feature:** `sprite name <svg ...>...</svg>` as inline SVG block (uses sub-format `<$name*3>`).
**Syntax example:** `sprite foo1 <svg width="8" height="8" viewBox="0 0 8 8"><path d="..."/></svg>`
**Status:** ❌
**Evidence:** Not found. No multi-line SVG sprite parser, no SVG pass-through into output.
**Notes:** Would require an XML/SVG mini-parser and an SVG-fragment store; nothing of that exists.

### 23.2 Sprite color override — ❌
**Feature:** `<$name,color=orange>` recolors the monochrome bitmap at render time.
**Status:** ❌
**Evidence:** No bitmap colourization logic. See above.

### 23.3 `-encodesprite` CLI — ❌
**Feature:** `java -jar plantuml.jar -encodesprite 16z foo.png` encodes a PNG to sprite text (4/8/16/4z/8z/16z).
**Status:** ❌
**Evidence:** No `--encodesprite`/`encode-sprite` CLI flag in `src/cli.rs`. Grep for `encodesprite` empty.
**Notes:** Out-of-scope for a renderer; nontrivial (requires PNG decode + grayscale quantization + custom compression).

### 23.4 Importing Sprite (GUI Open Sprite Window) — ❌ (N/A)
**Feature:** GUI helper to import an image and generate sprite text.
**Status:** ❌ (intentionally out of scope; no GUI)

### 23.5 Sprite examples (`[15x15/8z] <encoded>` form) — ❌
**Feature:** Single-line sprite with size + compressed payload: `sprite $printer [15x15/8z] NOtH3W0W208HxF...`
**Status:** ❌
**Evidence:** Not parsed.

### 23.6 StdLib sprites — 🟡
**Feature:** Use stdlib-bundled sprite libraries (`!include <archimate/...>` etc.) which transitively define sprites.
**Status:** 🟡 partial — `!include <…>` stdlib resolution exists (`src/preproc/includes.rs:464,468,494`) and 7 stdlib directories ship (`stdlib/{awslib14,azure,C4,gcp,material,office,tupadr3}`), so the include resolves; however the sprite definitions inside those files do nothing because the `sprite` directive is a no-op.
**Notes:** Side-effect: `Foo(alias, "label")` macros that *only* invoke a sprite render the rectangle/component but with no icon.

### 23.7 `listsprites` command — ❌
**Feature:** Diagnostic diagram that lists every defined sprite.
**Syntax example:** `@startuml\nlistsprites\n@enduml`
**Status:** ❌
**Evidence:** No `listsprites` / `listsprite` keyword in parser. Grep empty.

---

## Tally

| Feature | Status |
|---|---|
| Hex sprite definition | ❌ |
| `<$name>` reference + scale/color | ❌ |
| Inline SVG sprite | ❌ |
| `-encodesprite` CLI | ❌ |
| Sprite import GUI | ❌ (N/A) |
| `[WxH/Nz]` encoded form | ❌ |
| StdLib sprite libraries | 🟡 (include resolves; sprites no-op) |
| `listsprites` | ❌ |

**Score:** 0 ✅ · 1 🟡 · 7 ❌ out of 8 features. **Sprite support is essentially absent.**
