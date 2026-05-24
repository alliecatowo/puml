# Chapter 23 — Sprites: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

### 23 Sprite definition (hex grid) — ✅
**Feature:** Define a monochrome sprite via 4/8/16-gray hex grid: `sprite $name { ... }`
**Syntax example:**
```
sprite $foo1 {
  FFFFFFFFFFFFFFF
  F0123456789ABCF
  ...
}
```
**Status:** ✅
**Evidence:** `src/parser/core.rs:1019` parses top-level `sprite` statements into `StatementKind::SpriteDef`; `src/sprites.rs:93` decodes hex-grid rows with optional `[WxH/N]` hints; `src/normalize/family.rs:41` stores definitions in a `SpriteRegistry`; `tests/integration.rs:6977` covers a `[4x4/16]` hex sprite rendering through a sequence label.
**Notes:** This covers the ordinary monochrome grid path. Validation reports `[E_SPRITE_INVALID]` rather than silently dropping malformed rows (`tests/integration.rs:7033`).

### 23 Sprite reference `<$name>` — ✅
**Feature:** Embed sprite in text/label as `<$name>` and inline parameters: `<$name{scale=3}>`, `<$name*3>`, `<$name,scale=3,color=orange>`
**Syntax example:** `Alice -> Bob : Testing <$foo1{scale=3,color=orange}>`
**Status:** ✅
**Evidence:** `src/sprites.rs:34` parses `<$...>` references and scale/color parameters; `src/render/svg.rs:53` routes labels containing `<$` through inline sprite rendering when a registry is active; `src/render/svg.rs:218` emits mixed text/sprite groups; `tests/integration.rs:6977` verifies scale, color, and escaping behavior.
**Notes:** The active path is in shared SVG label rendering, so sequence labels and other renderers that call `creole_text` can display sprites. This is still not a full text-layout engine for all possible Creole/sprite combinations.

### 23.1 Inline SVG sprite — ✅
**Feature:** `sprite name <svg ...>...</svg>` as inline SVG block (uses sub-format `<$name*3>`).
**Syntax example:** `sprite foo1 <svg width="8" height="8" viewBox="0 0 8 8"><path d="..."/></svg>`
**Status:** ✅
**Evidence:** `src/parser/core.rs:1043` collects one-line or multi-line inline SVG sprite definitions; `src/sprites.rs:161` stores SVG source and dimensions; `src/sprites.rs:264` renders SVG sprite references; `tests/integration.rs:7009` verifies a scaled inline SVG reference.
**Notes:** SVG content is passed through as a fragment. This is useful for trusted diagrams, but it is intentionally not a full XML sanitizer.

### 23.2 Sprite color override — ✅
**Feature:** `<$name,color=orange>` recolors the monochrome bitmap at render time.
**Status:** ✅
**Evidence:** `src/sprites.rs:63` accepts `color`/`colour` parameters; `src/sprites.rs:270` applies the requested fill to monochrome sprite pixels; `tests/integration.rs:6977` verifies `color=orange` appears in rendered SVG.

### 23.3 `-encodesprite` CLI — ✅
**Feature:** `java -jar plantuml.jar -encodesprite 16z foo.png` encodes a PNG to sprite text (4/8/16/4z/8z/16z).
**Status:** ✅
**Evidence:** `src/cli.rs` includes `-encodesprite` handling; `src/sprites.rs:294` encodes pixel buffers; `tests/integration.rs:7045` verifies the CLI emits a compressed sprite definition for a generated PNG.
**Notes:** The CLI form is implemented for renderer-local use and does not attempt to mirror PlantUML's GUI import workflow.

### 23.4 Importing Sprite (GUI Open Sprite Window) — ❌ (N/A)
**Feature:** GUI helper to import an image and generate sprite text.
**Status:** ❌ (intentionally out of scope; no GUI)

### 23.5 Sprite examples (`[15x15/8z] <encoded>` form) — ✅
**Feature:** Single-line sprite with size + compressed payload: `sprite $printer [15x15/8z] NOtH3W0W208HxF...`
**Status:** ✅
**Evidence:** `src/parser/core.rs:1153` parses encoded sprite payloads after a `[WxH/Nz]` header; `src/sprites.rs:117` decodes packed or compressed payloads; `tests/integration.rs:6996` verifies the compressed `[15x15/8z]` sample renders and can be listed.

### 23.6 StdLib sprites — ✅
**Feature:** Use stdlib-bundled sprite libraries (`!include <archimate/...>` etc.) which transitively define sprites.
**Status:** ✅
**Evidence:** `!include <...>` stdlib resolution remains in `src/preproc/includes.rs`; parsed `sprite` directives now populate the registry via `src/normalize/family.rs:41`; `tests/integration.rs:7022` verifies `!include <material/folder>` renders `<$ma_folder{scale=2}>` as a visible icon.
**Notes:** This confirms the direct sprite-reference path for included stdlib sprite files. Macro libraries may still have separate macro-expansion limitations outside chapter 23.

### 23.7 `listsprites` command — ✅
**Feature:** Diagnostic diagram that lists every defined sprite.
**Syntax example:** `@startuml\nlistsprites\n@enduml`
**Status:** ✅
**Evidence:** `src/parser/core.rs:64` recognizes `listsprite`/`listsprites`; `src/normalize/family.rs:44` records the request; `src/render/svg.rs:21` renders the sprite sheet with bundled OpenIconic icons plus user-defined sprites; `tests/integration.rs` verifies list output and sprite count metadata.

### 23.8 OpenIconic inline SVG icons — ✅
**Feature:** PlantUML/OpenIconic-style label icons via `<&name>` and bare `&name`.
**Syntax example:** `Alice -> Bob : Open <&folder,scale=2,color=#2563eb> then &cloud-upload`
**Status:** ✅
**Evidence:** `src/openiconic.rs` vendors all 223 MIT Open Iconic SVG files as generated data; `src/sprites.rs` parses `<&...>` and safe `&name` references and resolves them to SVG sprites; `src/render/svg.rs` renders those icons as inline SVG paths in labels and `listsprites`; `tests/integration.rs` verifies rendered `<path>` output for both syntaxes.
**Notes:** Open Iconic is MIT licensed by Waybury. The import script is `scripts/import_openiconic.py`, and attribution is recorded in `THIRD_PARTY_NOTICES.md`.

---

## Tally

| Feature | Status |
|---|---|
| Hex sprite definition | ✅ |
| `<$name>` reference + scale/color | ✅ |
| Inline SVG sprite | ✅ |
| `-encodesprite` CLI | ✅ |
| Sprite import GUI | ❌ (N/A) |
| `[WxH/Nz]` encoded form | ✅ |
| StdLib sprite libraries | ✅ |
| `listsprites` | ✅ |
| OpenIconic `<&name>` / `&name` SVG icons | ✅ |

**Score:** 8 ✅ · 0 🟡 · 1 ❌ out of 9 features. The remaining ❌ is the intentionally out-of-scope GUI import helper; renderer and CLI sprite support are now broadly present.
