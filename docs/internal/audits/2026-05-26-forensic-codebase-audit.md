# Forensic Codebase Audit — PUML

**Date:** 2026-05-26
**Branch at audit time:** `fix/c4-family-unknown-relations` (HEAD `aa729faa`)
**Method:** 7 parallel skeptical subagent investigations (architecture, render pipeline,
parser/normalize/preproc, PlantUML feature parity, tests/CI/coverage, issues/docs honesty,
site/WASM/MCP/extension) + direct orchestrator verification of all high-impact claims.
**Stance:** Adversarial. Docs, comments, issues, and the subagents' own outputs were treated
as unverified until checked against code. Every load-bearing claim below was spot-confirmed
by reading the cited file/line (see Appendix B: Verification Log).

> This is a **point-in-time snapshot**, not a living spec. It will drift. When it disagrees
> with current code, the code wins — fix or retire the stale row here, per CLAUDE.md §8.

---

## 0. TL;DR / Overall verdict

For a "100% vibe-coded" project, this is **substantially better engineered than the label
implies, and the debt is structural rather than rotten.** ~116K LOC of Rust (411 files, 1,232
commits in ~11 days), ~1,794 test functions, real renderers for ~28 diagram families, a genuine
Sugiyama-style layout engine, a clean SVG→raster path, and disciplined determinism + no-panic
invariants on the parse path.

The danger you're worried about — sprawl, dead/divergent codepaths — **is present but
concentrated and nameable**, not diffuse rot. The five things that actually matter:

1. **The parser is a fake-modular monolith** — ~13K LOC stitched into one compilation unit via
   34 `include!`s. No privacy boundaries. This is the #1 structural risk and the root of the
   recurring "family misdetection" bug class.
2. **A half-finished typed-scene migration** — only ~6 of ~30 families produce typed render
   artifacts; the rest emit raw SVG strings that get **re-parsed by regex** to validate. Two
   validation regimes coexist indefinitely. The typed scene is advisory, not authoritative.
3. **Logic is reimplemented in JS in the browser** — `!include` resolution and syntax
   tokenization both exist as separate JS implementations that can silently diverge from the
   Rust core, with **no parity test**.
4. **Several headline correctness claims are theater** — `DeterminismMode` is a no-op stub; the
   85% coverage gate **excludes the entire renderer**; the PlantUML oracle is advisory and not a
   required check.
5. **A scatter of duplicated layout/text-metrics logic** — ~5 independent layout engines, text
   width estimated `chars × ~7px` in 9+ places that drift independently.

And a cross-cutting meta-finding: **CLAUDE.md — the agents' own source of truth — has drifted
and contains at least 4 concrete factual errors** that will mis-target future agents. Fixing
the doc is cheap and high-leverage.

Rough honest PlantUML feature parity: **~60–65%** — high breadth (most families render
*something* reasonable), uneven depth, a few real correctness bugs, and stubbed icon libraries
that look supported but aren't.

---

## 1. CLAUDE.md is wrong in specific, fixable ways (do this first — it's cheap)

CLAUDE.md is authoritative for agent behavior, so its errors propagate into every future task.
Confirmed inaccuracies:

| Claim in CLAUDE.md | Reality (verified) |
|---|---|
| `src/render/family.rs` is "the most contended god file" (§12) | It is **31 lines** — a pure `mod`/`pub use` shim. Content was split into `src/render/family/` (19 files, ~6.7K LOC, largest 599). **There is no god-file in the repo.** |
| Layout module "live at `src/render/graph_layout.rs`" (§1, §8) | It's a **directory** `src/render/graph_layout/` (rank/crossing/coordinates/groups/router/scene + tests). The path in §8 is wrong. |
| Layout engine is "stage 1 complete" (§1) | Module header says **Stage 3** (orthogonal channel routing live). Doc *undersells* by two stages. |
| "orthogonal edge routing wired into sequence and class" (§1) | **Sequence is FALSE** — sequence messages are straight 2-point polylines (`sequence/scene.rs:472`), emits `route_channel_ids: Vec::new()`. Routing is wired into **class + component + deployment**, not sequence. |
| #399 "Language service" is an **open epic** (§1, §10) | Issue #399 is **CLOSED**. |
| Coverage gate is `--fail-under-lines 85` (§9, §14) | No file uses 85. PR gate (the actual merge blocker) = **83**; `check-all.sh` = **87**; `release_contract_audit.rs` pins **87**. The doc value matches none. |
| "PEG-based .puml grammar" (§1) | Not PEG. No pest/nom. It's a **hand-rolled line-oriented string matcher**. `winnow = "1.0"` is declared in Cargo.toml with **zero usages** (dead dependency). |
| Language service "accessible via MCP and the VS Code extension" (§1) | VS Code: true. **MCP: false** — exposes only check/diagnostics/render, no hover/completion/semantic-tokens/formatting. Site: never calls the WASM language service at all (dead code). |

**Action:** A 30-minute PR correcting §1, §8, §9, §10, §12 of CLAUDE.md pays for itself
immediately. This is the single highest leverage-per-effort item in the whole audit.

---

## 2. Master priority list (ranked by impact × tractability)

| # | Finding | Severity | Effort | Area |
|---|---|---|---|---|
| P1 | Parser is a 34-`include!` monolith with no module boundaries | **HIGH** | High | parser |
| P2 | Typed-scene migration stalled at ~6/30 families; SVG-string re-parse is the real validator | **HIGH** | High | render |
| P3 | Strict parsing aborts the **entire** diagram on one unknown line (PlantUML degrades) | **HIGH** | Med | parser/normalize |
| P4 | JS reimplements `!include` + tokenizer in browser; no parity test vs Rust | **HIGH** | Med | site/wasm |
| P5 | Coverage gate excludes the renderer (63% of `src/`); the core has no line floor | **HIGH** | Med | ci |
| P6 | ~5 independent layout engines; text-width estimated 9+ ways, drifting | **MED-HIGH** | High | render |
| P7 | CLAUDE.md factual errors mislead agents | **MED** | **Low** | docs |
| P8 | `DeterminismMode` is a no-op stub presented as an enforced invariant | **MED** | Low | api |
| P9 | Oracle conformance is advisory, not gating; 2/21 fixtures blocking (both trivial) | **MED** | Med | ci |
| P10 | AWS/Azure/GCP/tupadr3 icon libs ship as stereotype-box **stubs** (look supported, aren't) | **MED** | High | parity |
| P11 | Class-family relation labels float disconnected; parallel edges collapse; labels dropped | **MED** | Med | render |
| P12 | JSON/YAML object keys alphabetized, not document order (serde_json w/o `preserve_order`) | **MED** | **Low** | render |
| P13 | scene/SVG divergence in box_grid (no `rebuild_scene`; scene validates stale geometry) | **MED** | Low | render |
| P14 | VS Code extension has zero CI coverage; shaky dep pins (`typescript@^6`, `@types/node@^25`) | **MED** | Low | ci/ext |
| P15 | `!definelong` unsupported → hard error (common in real macro libs) | **MED** | Med | preproc |
| P16 | Stale issue bodies (#1183, #725, #448) describe code states that no longer exist | LOW | Low | docs |
| P17 | `too_many_arguments` ×37, dead `graph_layout` items w/ lying "Stage 3" comments | LOW | Low | render |
| P18 | No DOT/Graphviz passthrough, no Sudoku (entirely missing families) | LOW | High | parity |

**The quick wins (P7, P8, P12, P14, P16):** all low-effort, all just cleanup or one-line fixes.
Doing them clears the "is anything actually broken vs just claimed" fog.

**The structural bets (P1, P2, P6):** these are the ones that decide whether this codebase ages
well. They're the "architectural sprawl" you're worried about, made concrete.

---

## 3. Detailed findings by area

### 3.1 Architecture & sprawl

- **Module map (LOC by dir):** `render` ~33K (29% of code, 136 files), `parser` ~13K (48 files),
  `normalize` ~11.7K (47 files), `material_icons.rs`+`bootstrap_icons.rs` = **17K combined**
  (generated lookup tables — 15% of raw LOC, mentally exclude them), `preproc` ~5.9K, `theme`
  ~3.9K, `specialized` ~3.8K, `language_service` ~1.9K, `frontend` ~1.9K.
- **No god-file.** Largest hand-written non-test file is ~599 LOC. The codebase has been
  *aggressively split* — the opposite of a god-file problem. CLAUDE.md §12 is stale (see §1).
- **`include!`-monolith (P1):** `src/parser.rs` (61 LOC) does `include!(...)` **34 times**,
  inlining ~9.8K LOC into one compilation unit; `parser/activity.rs` and `parser/state.rs`
  recursively `include!` more. Net: a ~13K-LOC parser with **one flat namespace, no privacy, no
  module boundaries.** The file split is cosmetic. Helpers can silently collide or leak globally;
  refactoring is hazardous. `src/parser/core.rs` is a 1-line tombstone still `include!`d.
- **Dead code (genuine):** `layout_to_i32_positions` (graph_layout/mod.rs:199, `#[allow(dead_code)]`
  + lying "Used by Stage 3" comment, zero callers), `barycenter` borrowed-str variant
  (crossing.rs:213, only `_owned` is used), `_size` stub (wire.rs:397), `GraphLayout.node_ranks`
  field (written never read), `Direction::LeftRight`/`LayoutOptions.direction` (never wired;
  `layout_hierarchical` ignores direction). Several `#[allow(dead_code)]` annotations on
  `edge_paths`/`route_channels`/`scene` are **stale lies** — those fields are actually live.
- **TODO/FIXME/HACK density:** effectively **zero** (1 in non-test src). Unusually clean.
- **unwrap/panic discipline:** Non-test `unwrap()` = 29 (most inside `#[cfg(test)]` modules);
  `expect()` = 124 (mostly internal invariants, not user input); real `panic!` = essentially only
  in test modules. **`src/parser/` has zero `expect()`/`panic!`/`unwrap()`** — the
  no-panic-on-user-input invariant is genuinely upheld on the parse frontline.
- **`#[allow(clippy::too_many_arguments)]` ×37** across 22 files — real "params threaded instead
  of context struct" smell, densest in preproc/includes, render/activity, render/family.
- **Abandoned features genuinely deleted (CLAUDE.md accurate here):** "dual chart renderer"
  removed in `f3e8ce81`; `--lsp-capabilities` manifest removed in `b17da41e` (replaced by
  `--dump-capabilities` → `lsp_capabilities()`, a real intended replacement, not a zombie).

### 3.2 Render pipeline & layout

- **Data flow (as built):** source → (optional mermaid/picouml frontend adapt) → `parser::parse`
  → `ast::Document` → `normalize_family` → `NormalizedDocument` (20-variant enum) → two-level
  dispatch (`match` on the enum, then `FamilyRenderKind` for the `Family` variant) → per-family
  layout + **ad-hoc SVG string building** → `validate_svg` (re-parses the string) → `RenderArtifact`
  → optional resvg/tiny-skia raster.
- **SVG string is the de-facto IR (P2).** Every renderer builds a `String` via `push_str`/
  `format!` of raw SVG tags. There is **no central scene→SVG serializer**. The authoritative
  validation + auto-correction (viewBox expansion, label background-rect insertion) operate on the
  **re-parsed SVG string** via regex/string scanning (`validate/invariants.rs` `extract_text_elements`,
  `parse_viewbox`, `extract_node_bboxes`). Geometry is computed → serialized → re-parsed to
  validate. Fragile (depends on exact attribute formatting).
- **Typed scene exists but is advisory.** `RenderScene` (`render_core.rs:367`) with
  nodes/edges/groups/lanes/labels/route_channels + geometry validation. Only ~6 renderers populate
  it (sequence, class/FamilyStub, box_grid component/deployment, nwdiag, wire). `SceneAvailability`
  enum {`TypedScene`,`NotMigrated`,`Unsupported`} explicitly tracks the incomplete migration.
  `run_with_scene` runs SVG-string invariants first, then merely *appends* typed issues — the scene
  does not drive output.
- **Layout engine is REAL (not aspirational).** `src/render/graph_layout/` (~2.8K LOC): longest-path
  ranking + DFS back-edge cycle breaking (`rank.rs`), barycenter crossing minimization + transposition
  refinement (`crossing.rs`), coordinate assignment, a 593-LOC channel router (`router.rs`), scene
  builder, 20 tests. Legit Sugiyama implementation. **But adoption is ~3 families:** class/object/usecase,
  component/deployment, chen. Everything else hand-rolls layout.
- **~5 parallel layout engines (P6):** `graph_layout/` (shared by 3), sequence `layout/` (~2.2K LOC),
  state `state/layout/`, activity `activity/layout/flow.rs`, salt `salt/layout.rs`, plus per-specialized
  (nwdiag, chart). No shared contract beyond geometry primitives. Bug fixes must be applied N times.
- **Text-width estimation forked 9+ ways (P6):** a "central" `text_metrics.rs` (`DEFAULT_MONOSPACE_CHAR_WIDTH = 7`)
  used by only 5 callers, while independent `CHAR_WIDTH = 7`/`CHAR_WIDTH_PX = 7` copies live in
  `family/tree.rs`, `chart/layout.rs`, `nwdiag/layout.rs`+`scene.rs`, `state/labels.rs`, `salt/text.rs`,
  `mindmap/labels.rs`, `wire.rs`, `svg.rs`, **and the validator's own copy** (`validate/svg_hooks.rs:190`).
  They drift independently → label-overflow invariants can disagree with the layout that made the labels.
- **scene/SVG divergence in box_grid (P13):** scene is built from the router's raw `edge_paths`, but the
  SVG edges are drawn from a *post-processed* version (`box_grid_edges.rs` snapping/collision). box_grid
  does **not** call `rebuild_scene`, so scene validation silently validates stale geometry. (class_layout
  *does* rebuild — asymmetry confirmed.)
- **PNG path is a genuine strength:** strictly downstream of SVG (`output.rs` resvg/usvg → tiny_skia).
  One geometry source; no divergent raster path. f64→i32 downcast at the engine/renderer boundary is a
  minor precision smell.
- **SVG root-tag `<svg …>` emitted by hand in ~28 files**, only 6 with full xmlns header. No document builder.

### 3.3 Parser / normalize / preproc

- **Parser tech:** hand-rolled line matcher. Core is `parse_preprocessed()` (`core_preprocessed.rs`,
  ~597 LOC) — a ~50-branch `while i < lines.len()` waterfall of `starts_with`/`split`/`trim` +
  family-detection heuristics (`detect.rs`, 178 LOC of `starts_with` chains). **Ordering-dependent:**
  reordering branches changes detection — this is the structural root of the recurring family-misdetection
  bug class (e.g. the C4-relation-before-detection fix on this very branch).
- **Fail-loud, low-tolerance (P3).** Unknown lines → `StatementKind::UnsupportedSyntax` →
  `normalize/common.rs` converts to **hard errors** (`E_*_UNSUPPORTED_SYNTAX`). Empirically verified:
  one gibberish line aborts the *entire* render. PlantUML is far more lenient (renders what it can).
  This is the highest user-facing risk for real-world `.puml` using any unimplemented syntax.
- **No-panic invariant upheld:** zero bare unwrap/expect/panic/unreachable/todo in non-test
  parser/preproc/normalize. All return `Result<_, Diagnostic>`.
- **Normalize is a per-family dispatcher, not a pass pipeline.** All branches live; no dead/no-op
  normalizers. (Note: `normalize/family/stub.rs` is a 549-LOC *full* normalizer — "stub" = UML stub
  declarations, not incompleteness. Misleading name.)
- **Rust preprocessor is unusually complete:** define/macros, all conditionals, while/foreach/break/
  continue, function/procedure/return, ~200 `%builtin` functions, variables + local/global scopes,
  includes (include/includesub/include_many/includeurl/import), include-once dedup, cycle detection,
  path-escape guards, assert/log/dump (expand-then-discard for determinism). **Gaps:** `!definelong`
  is **unsupported → hard error** (P15, common in real macro libraries, confirmed zero matches in src);
  `%json`-style JSON preprocessing explicitly unsupported.
- **Rust-vs-JS preproc divergence (P4):** This is NOT two full implementations — it's a two-layer
  arrangement. WASM has no filesystem, so `!include` returns `E_INCLUDE_NOT_SUPPORTED_WASM`; the browser's
  `resolveIncludes()` (~75 LOC in `site/static/js/editor.js`) is an `!include` *pre-fetcher* only, then
  hands merged text to WASM which does all real preprocessing. **3 concrete defects in the JS layer:**
  (a) depth cap 8 vs Rust's 32 — deep include trees silently truncate in-browser; (b) no `include_once`/
  cycle dedup; (c) `!includesub File!section` tag regex bug (never strips `!tag`). Plus the docs
  inline-fence preview doesn't call `resolveIncludes` at all, so any `!include` there hits WASM and
  errors.

### 3.4 PlantUML feature parity (~60–65% overall)

**Strong (genuinely good):** sequence (~85%, PlantUML-grade: alt/opt/loop/par/critical/break,
activation stacks, create/destroy, autonumber, ref, notes incl hnote/rnote, box, hide footbox),
component/deployment (~75%), state (~75%), timing (~70%), gantt/chronology (~70%), mindmap/wbs (~70%),
salt (~65%), regex/ebnf railroad (~65%).

**Cross-cutting strengths:** skinparam is **deep** (~300+ keys across `theme/skinparam/*`, 1616 LOC);
themes ~40 named presets (`presets.rs`) applying real colors; creole/markup **strong** (bold/italic/
underline/strike/color/size/links/tables/lists/HTML); OpenIconic + Bootstrap + Material icons bundled
with real SVG glyph data (~18K LOC) and render inline.

**Weak / buggy / missing:**
- **Class (~65%) has real bugs (P11):** relation labels float disconnected from edges, parallel edges
  between the same pair collapse/overlap, inheritance label text dropped, weak grid auto-layout with
  wasted whitespace. Class is PlantUML's #1 use case after sequence — this matters.
- **JSON/YAML (~70%) key-ordering bug (P12):** keys rendered **alphabetically**, not document order
  (`serde_json` without `preserve_order` feature — confirmed in Cargo.toml). Silent correctness bug
  for every multi-key object.
- **AWS/Azure/GCP/tupadr3/office icon libs are STUBS (P10):** `!include` resolves (so no error) but each
  service maps to a plain `object <<stereotype>>` box, NOT the vendor icon artwork. Looks supported,
  isn't. High-impact for cloud-architecture diagrams (among PlantUML's most-used stdlib features). The
  stub files are honestly commented as "compatibility stub," but the user-visible result misleads.
- **Math (~45%):** real ~1100-LOC AsciiMath layout engine (sums/fractions/sqrt/sub-sup/Greek), but NOT
  full JLaTeXMath — complex LaTeX falls back to a monospace box.
- **C4 (~60%):** Person/System/Container/Component + Rel macros parse, but System/Enterprise boundary
  frames not consistently drawn as dashed grouping boxes, no SHOW_LEGEND, generic palette not C4's exact
  per-type colors.
- **Missing entirely (P18):** DOT/Graphviz passthrough (`@startdot`/`digraph`), Sudoku (`@startsudoku`).
- **Auto-layout quality** for graph families is grid-based, not force/layered — suboptimal placement.

**Honesty check — claimed-but-not-real:** the icon stubs (P10) are the main "looks supported but isn't."
ditaa and math *look* like stubs (have `_fallback` paths) but are **real** engines. `FamilyStub`
(class/object/usecase) is a misnomer — routes to a full (if buggy) renderer.

### 3.5 Tests / CI / coverage / correctness

- **Test volume is real:** ~335 unit `#[test]` in `src/`, ~1,449 integration across 83 files, ~10 in
  crates = **~1,794** total. Only 2 genuine `#[ignore]` (a bless tool + a slow redundant oracle check).
  No `assert!(true)` anywhere; ~2 `is_ok()`-only + ~23 `let _ =` no-panic patterns, all honestly named.
  Sampled tests assert real computed values — **meaningful, not synthetic fill.**
- **Coverage gate excludes the renderer (P5).** The `--ignore-filename-regex` drops all of
  `render/`, `parser/`, `normalize/`, `frontend/`, `specialized/` — **259 of 411 files (63%)**. The
  ~85%-class number is measured against language-service/CLI/layout/preproc only. **The core
  rendering/layout logic has no line-coverage floor.** Three inconsistent thresholds: PR gate 83
  (the actual merge blocker — the *lowest*), check-all 87, doc says 85.
- **`DeterminismMode` is theater (P8).** `interpret_determinism_contract(_mode)` discards the arg and
  has an empty body (`api/pipeline.rs:133`). `Strict` and `Full` are byte-identical. The underlying
  determinism is plausibly real (BTreeMap discipline — only 10 HashMaps in `src/`, all in the LSP doc
  store where order can't affect output; relations.rs has an explicit "BTreeMap not HashMap" comment),
  but the *named mechanism* enforces nothing.
- **Oracle conformance is advisory (P9).** Real machinery (SHA-pinned PlantUML JAR, `-tsvg -pipe`,
  diff) but **coarse text-grep metrics** (element counts, viewBox ±10%), not pixel/structural. Soft
  thresholds: ≥80% = pass, 50–79% = advisory WARN (still "success"), <50% = fail. Committed report is a
  `{"skipped": true}` sentinel. Only **2 of 21** promoted fixtures are `gate: blocking` (both trivial
  sequence diagrams). **Intent-vs-reality gap:** `branch-protection.sh` *lists* `differential-svg-oracle`
  as a required context, but the testing audit found live `main` protection requires only
  `fmt-clippy-test-coverage-quick`. Verify which is true on the actual repo settings.
- **Visual baselines:** genuine per-channel RGBA pixel diff (threshold 3/255), red-highlight diff PNGs
  on failure. But thin corpus — **30 committed baselines**, mostly one-per-family, 22 manifest fixtures
  deferred to text-only. PNG diff only runs on Linux (no-ops on macOS dev). Main-gate explicitly refuses
  to block on PNG byte drift.
- **What truly gates merge:** one synthesized required check (`fmt-clippy-test-coverage-quick`) =
  fmt + clippy `-D warnings` + nextest + doctest + coverage 83 (renderer-excluded) + wasm check +
  docs-drift + conditional site-smoke. **Theater layer:** oracle (not required, soft, skip-sentinel),
  `oracle_smoke` (`continue-on-error: true`), benchmarks (not in PR path), PNG drift (not blocking).
  The greenwashing is concentrated in exactly the *conformance/parity* layer that backs the boldest
  "PlantUML parity" claims.

### 3.6 Issues & docs honesty

- **Issue tracker is unusually disciplined.** 44 open / 500+ closed in an ~11-day-old repo
  (>11:1 closed:open). **Every open issue updated within 3 days** — no rotting backlog. Labels applied
  consistently (31 agent-ready, 19 P1, 6 P0, 17 architecture). High signal: most issues cite concrete
  file paths + measurable acceptance criteria. No true duplicates.
- **Epic health:** #88 (Oracle) alive, #89 (CI hardening) alive, **#399 (Language service) CLOSED but
  listed open in CLAUDE.md**, #590 (Renderer architecture) alive and the most active (drives the bulk of
  recent PRs toward the typed-scene/domain-contract migration).
- **Stale issue bodies (P16):** #1183 lists 6 normalizer files allegedly still using
  `StatementKind::Unknown` — all 6 have **zero matches** (migrated to `RawSyntaxCategory`). #725 (wire)
  and #448 (board/files) describe features as missing that **already landed** — but both kept open
  *deliberately with dated "partial" rationale*, so tracked-as-partial, not blindly stale.
- **Docs:** all referenced docs exist; generally current and well-disclaimed (the legacy parity ledgers
  self-disclaim "if this disagrees with code, retire the row"). Anti-stale-doc directive is being acted
  on (issues #970/#968 cleaned up wave logs). **The site docs are the exception** — see 3.7.
- **PR flow is coherent**, not thrashing: recent PRs cluster on #590's typed-scene migration + frontend/
  source-map/C4 fixes. Work flowing = work described in epics.
- **Closure evidence is mixed:** some excellent (LOC + guardrail output + SHA), some rubber-stamped via
  PR auto-link with thin issue-side evidence (#1058, #1025, #769).
- **Verdict:** issues/docs reflect reality ~85–90% faithfully. The "lies" are **lag** (bodies trailing
  fast-moving code), not **fiction** (claimed-but-nonexistent features).

### 3.7 Site / WASM / MCP / VS Code extension

- **The premise in the docs is wrong (and that's a finding).** `site/` is **Zola (static-site gen) +
  vanilla JS + CodeMirror 6**, NOT "TypeScript + Vite." Zero `.ts` files, no Vite, no framework, no
  package.json. The repo's own `site/content/developer/specs/studio-spa.md` describes a TS SPA
  (`semanticTokens.ts`, `compile()`) **that was never built.** Stale specs will mislead future agents
  into "fixing" things to match fiction.
- **WASM bridge is clean and in-sync.** `crates/puml-wasm/src/lib.rs` (547 LOC) is a thin binding over
  the real pipeline + language service. Builds clean. The one deliberate gap: no `!include` (browser
  reimplements it — see P4).
- **MCP server is a thin, honest CLI wrapper** (`agent-pack/bin/puml-mcp`, 390 LOC Python; every tool
  shells out to the binary; path-containment + URL-includes-off-by-default). Good smoke test. **But**
  exposes only check/diagnostics/render — **no language-service tools** (contradicts CLAUDE.md).
- **VS Code extension is the healthiest non-core surface.** A real LSP client (`vscode-languageclient`)
  spawning the Rust `puml-lsp`; **reimplements zero language features in TS**. CLI fallback also hits the
  Rust binary. **But:** zero CI coverage (no workflow builds/tests it), shaky dep pins
  (`typescript@^6.0.3`, `@types/node@^25.9.1` may not resolve), stale displayName ("Sequence Diagrams").
- **Language service is FRAGMENTED, not unified (contra CLAUDE.md).** Only VS Code truly consumes it.
  MCP = diagnostics only. Site exposes `compile()`/`languageService()` from WASM but **never calls them**
  (dead code) — the editor highlights with a **hand-written JS tokenizer** (`puml-tokens.js`, 169 LOC)
  instead. So there are effectively **three independent syntax definitions:** Rust `language_service/
  syntax.rs`, JS `puml-tokens.js`, and the VS Code TextMate grammar. They will drift.
- **Divergence risks (P4):** (1) `!include` resolver reimplemented in ~92 LOC of JS vs ~5.9K LOC Rust
  preproc — browser preview can resolve includes differently than CLI/LSP, **no parity test**; (2) the
  JS tokenizer drifts from the Rust grammar; (3) WASM `semantic_tokens` is lossy (collapses to
  keyword/operator) *and* unused in-browser.

---

## 4. Recommended sequencing (if you want a plan)

**Wave 0 — cheap truth/cleanup (a day, mostly low-risk):**
- P7: Fix CLAUDE.md §1/§8/§9/§10/§12 factual errors.
- P12: Add `serde_json = { version = "1", features = ["preserve_order"] }` → fixes JSON/YAML ordering.
- P8: Either make `DeterminismMode` do something or delete the enum + flag and stop claiming it enforces.
- P16: Update issue bodies #1183/#725/#448 (or close with evidence).
- P17: Delete the 4 genuinely-dead `graph_layout` items + strip the 3 stale `#[allow(dead_code)]` lies.
- P14: Add a VS Code extension build/smoke step to CI; pin deps to resolvable versions.
- Resolve the P9 intent-vs-reality gap: decide whether `differential-svg-oracle` is required on `main`
  and make `branch-protection.sh` match reality.

**Wave 1 — close the credibility gaps (these back the "parity" story):**
- P5: Either stop excluding the renderer from coverage, or add a *separate* renderer-coverage gate so
  the core has *some* floor. At minimum, document honestly what the 83/87 number covers.
- P9: Decide if the oracle is a gate or a dashboard. If a gate, promote more fixtures to blocking and
  tighten thresholds. If a dashboard, stop implying it's conformance enforcement.
- P3: Add a graceful-degradation mode (render what parses, surface unknown lines as warnings/visible
  error blocks) instead of aborting the whole diagram. Biggest real-world UX win.

**Wave 2 — structural bets (the anti-sprawl work):**
- P1: Convert the parser's 34 `include!`s into real `mod` declarations with `pub(super)`/`pub(crate)`
  boundaries. This is the foundation for safely fixing the family-misdetection bug class.
- P2: Finish the typed-scene migration (drive remaining families through `_artifact`/typed scene),
  then make the scene authoritative for validation and **delete the SVG-string re-parse path** +
  `NotMigrated` variant. This is the single biggest correctness-architecture improvement available.
- P6: Unify text metrics to one module (delete the 9+ copies, especially the validator's own), and
  define a shared layout contract so the ~5 engines can converge over time.
- P4: Add a Rust↔JS include-resolution parity test; ideally move include pre-fetch behind a single
  spec the JS shim must satisfy (or surface WASM's include limits explicitly in the editor).

**Wave 3 — depth/parity (only if parity is a real goal):**
- P11 (class relation labels/routing), P10 (real cloud icon libs), P10/P18 (missing families), C4 depth.

---

## Appendix A: Subagent reports

Seven parallel investigations fed this synthesis. Their full reports (with additional file:line
detail beyond what's distilled here) were produced in the audit session on 2026-05-26:
architecture/sprawl, render pipeline/layout, parser/normalize/preproc, PlantUML parity,
tests/CI/coverage, issues/docs honesty, site/WASM/MCP/extension.

## Appendix B: Verification Log (orchestrator-confirmed, not just relayed)

Claims personally re-checked against code before inclusion:

| Claim | Verification |
|---|---|
| `family.rs` is 31 lines (no god-file) | `wc -l src/render/family.rs` → 31; real code in `src/render/family/` (19 files) |
| `graph_layout` is a directory | `ls src/render/graph_layout/` → rank/crossing/coordinates/groups/router/scene + mod; `router.rs` = 27KB |
| `winnow` declared, zero usages | `grep -rln winnow src crates` → none; `Cargo.toml:58: winnow = "1.0"` |
| `serde_json` lacks `preserve_order` | `Cargo.toml:56: serde_json = "1"` (no features) |
| `DeterminismMode` is a no-op | `api/pipeline.rs:133 fn interpret_determinism_contract(_mode: DeterminismMode)` — empty body, arg discarded |
| Coverage excludes renderer | `pr-gate.yml:305` ignore-regex includes `src/(frontend\|normalize\|parser\|render\|specialized)/.*\.rs`; PR=83, check-all=87 |
| `!definelong` unsupported | `grep -rn definelong src/` → zero matches |
| Parser is `include!`-monolith | `grep -c "include!" src/parser.rs` → 34 |
| Oracle intent-vs-reality | `branch-protection.sh:12` lists both contexts incl `differential-svg-oracle`; testing audit found live `main` requires only `fmt-clippy-test-coverage-quick` — **flagged for live verification** |
