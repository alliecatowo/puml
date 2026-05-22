# Chapter 19 — Maths Audit

Tally: 3 ✅ / 1 🟡 / 1 ❌

### 19.0 Inline `<math>...</math>` in other diagrams — 🟡
**Feature:** AsciiMath inline expressions embedded in activity/sequence/note text
**Syntax example:** `Bob -> Alice : Can you solve: <math>ax^2+bx+c=0</math>`
**Status:** 🟡
**Evidence:** No `<math>`/`</math>` inline marker handler found in render text/creole (grep returned no hits). Likely passed through as literal text.
**Notes:** Standalone math renderer exists but inline-in-other-diagrams is not threaded through render paths.

### 19.0 Inline `<latex>...</latex>` in other diagrams — ❌
**Feature:** JLaTeXMath inline expressions in other diagrams
**Status:** ❌
**Evidence:** No inline latex token detection in src/render or src/creole.
**Notes:** Same gap as inline math.

### 19.1 Standalone `@startmath` / `@endmath` AsciiMath diagram — ✅
**Feature:** Whole-diagram AsciiMath block
**Syntax example:** `@startmath\nf(t)=...\n@endmath`
**Status:** ✅
**Evidence:** src/parser/blocks.rs:55-94 BlockKind::Math; src/normalize/raw.rs:3-10 normalize_math; src/render/specialized/math.rs (render_math_svg with fallback); src/specialized/math.rs:1151 render_math_from_parts

### 19.1 Standalone `@startlatex` / `@endlatex` LaTeX diagram — ✅
**Feature:** Whole-diagram LaTeX block (uses same Math kind)
**Status:** ✅
**Evidence:** parser/blocks.rs:55, 93 maps @startlatex → BlockKind::Math; same render path as math
**Notes:** Spec calls out separate math/latex but puml unifies them under MathDocument. Output uses ASCIIMathTeXImg-equivalent for both — LaTeX-specific rendering may differ from JLaTeXMath fidelity.

### 19.2 How it works (informational) — ✅
**Status:** ✅ (no implementation needed; just docs)
