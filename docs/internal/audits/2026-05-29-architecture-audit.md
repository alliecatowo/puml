# Architecture Audit — 2026-05-29

> Companion to `2026-05-26-audit-narrative.md` and
> `2026-05-26-forensic-codebase-audit.md`. Where those documents are the original
> "vibe-coded baseline" and the structured ledger backing it, **this document is
> the 72-hour follow-up**: what landed between 2026-05-26 and 2026-05-29, the
> current diagnostic state of every major subsystem, the layer-boundary picture,
> the next three strategic migrations, and the dispatch list for the orchestrator.
>
> Audit window: 2026-05-26 17:00 → 2026-05-29 10:00 UTC (~64 hours).
> Method: read prior audits + recent merged PRs + branch tip, then walk the source
> tree subsystem-by-subsystem with grep, `gh issue`, and targeted `Read` of the
> most relevant files. No code was modified. Numbers cited are from this audit's
> own measurements unless noted; when they disagree with prose comments in source
> files, the measurement wins.
>
> Point-in-time. The code wins on any conflict.

---

## 1. Where we were on 2026-05-26

The 2026-05-26 forensic audit landed an honest, slightly unflattering portrait
of a ~116K-LOC repo that was structurally healthier than the "100% vibe coded"
label feared — real Sugiyama layout engine, real parser discipline (zero
`unwrap`/`expect`/`panic` on the parse path), real BTreeMap determinism — but
that was carrying five named architectural debts, not five hundred diffuse ones.
The narrative companion (`2026-05-26-audit-narrative.md`) framed the choice as a
fork: parity-first (embrace per-family duplication and chase the oracle number)
versus architecture-first (freeze new families, finish the typed scene, delete
the SVG-regex validator, then resume breadth on a clean base).

The owner picked **option B, aggressively**: kill `DeterminismMode`, accept the
oracle as advisory not gating, target 90%+ coverage with the gate disabled while
climbing, pursue the typed-scene and shared-layout/geometry migrations first, and
do all of it on the single mega-branch `refactor/claude-wave-migrations` with a
single PR finish. Epic #1258 was opened as the umbrella ticket. The structural
migrations explicitly took priority over PlantUML parity catch-up.

The forensic audit's master priority table called the shots: P1 (parser
unmonolithing), P2 (typed-scene migration + SVG-regex validator deletion),
P3 (graceful degradation for one-unknown-line abort), P4 (JS-vs-Rust browser
divergence), P5 (coverage gate excludes the renderer), P6 (~5 independent layout
engines), then the cheap-truth wave (P7 doc fix, P8 DeterminismMode delete,
P12 serde `preserve_order`, P14 VS Code CI, P9 oracle decision). The Wave 0/1
cleanup was explicitly meant to be cheap and ruthless before the structural
bets started.

---

## 2. What landed between 2026-05-26 and 2026-05-29

In ~64 hours the repo shipped **18 merged PRs** spanning #1259 through #1310
plus two large PRs still open at audit time (#1311, #1312, both on
audit-fix branches expected to merge imminently). Below is the wave-by-wave
register, grouped by the function the PR served.

### 2.1 Wave-2 through wave-14 — PlantUML parity catch-up
Despite the forensic audit's "architecture before parity" decision, parity work
ran in parallel with structural work. The waves landed roughly chronologically:

- **#1264 wave-2** — graceful degradation (P3 closed), parser modularization
  begins, shared-layout fixes start filtering into class/component.
- **#1265 wave-3** — **delete the SVG-regex validator (the P2 finale)**,
  JS↔Rust parity work begins, class label/edge fixes.
- **#1266 wave-4** — `!definelong` macros (P15 closed), cloud icon rendering
  (P10 closed for AWS/Azure/GCP), shared style cascade rollout begins.
- **#1267 wave-5** — JSON/YAML document-order (P12 closed via `preserve_order`
  serde feature), style cascade widens, PlantUML syntax parity batch 1.
- **#1268 wave-6** — completes the style cascade (#1184 closed), syntax parity
  batch 2, creole/markup expansion.
- **#1269 wave-7** — preprocessor parity batch 2, syntax parity batch 3,
  sequence depth coverage.
- **#1272 wave-9+10** — creole blocks, timing analog rendering, IE entity
  notation, activity inline color, state inline color, class exotic arrowheads.
- **#1273 wave-11** — sequence verify, component depth, deployment depth,
  salt widgets.
- **#1274 wave-12** — preproc `!function`, use case extension points,
  Chen ER advanced.
- **#1277 wave-13** — nwdiag verify, gantt verify, stereotype-as-shape canonical
  rendering (raw `<<x>>` text replaced with shape mapping).
- **#1308 + #1309 wave-14** — five PlantUML-compatible named themes
  (plain/cerulean/cyborg/hacker/materia, 33 tests), smart-default shape mapping
  for DDD/architectural stereotypes (closes #1285).
- **#1310 wave-14 group A** — five visual-audit P0s drained in one PR
  (#1289 parser quote strip, #1293 IE crow's-foot marker defs, #1294 four salt
  widget bugs, #1301 wire duplicate port label, #1303 gantt milestone dedup).
- **#1311 wave-14 group B/C (open)** — four state and sequence P0s (#1304
  composite actions + transition labels, #1305 stereotyped pseudostate labels,
  #1306 history pseudostates inside composite, #1295 ref-over-multibox margin).
- **#1312 wave-14 activity+layout (open)** — six activity and layout P0s
  (#1299 fork bar clamp, #1300 nested if/else routing, #1302 swimlane columns,
  #1287 deep-nested package bbox propagation, #1288 inter-package edge routing,
  #1290 Kubernetes nesting).

### 2.2 Aggressive CI overhaul
Three back-to-back PRs hammered the PR gate into shape:

- **#1270 + #1271 + #1276** — "make gates lighter," then "aggressive PR-gate
  teardown — sub-4min wall time," then "align PR Gate to Main Gate + radical
  slim + structural drift contract test." Together these:
  - Rip the aggregator and merge `coverage` and `test` into a single
    instrumented `nextest` run (one compile pass, both signals).
  - Introduce `[profile.release-ci]` (opt-level 1, no LTO, codegen-units 16)
    for binary-size and artifact-regen jobs.
  - Switch the cargo index to sparse HTTP via
    `CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse`.
  - Reorganise `lefthook.yml` so pre-commit runs only fast guards
    (fmt + file-size + renderer boundaries) and pre-push adds `cargo check`.
  - Add a structural drift contract test under `tests/*_contract_audit.rs`.
- **#1275** — "split 5 oversized files to unblock main CI" — the file-size
  guardrail (`check_rust_file_sizes.py`) was tightened to 600 LOC with a small
  allowlist for generated icon tables.
- **#1278 + #1279 + #1280 + #1281 + #1282 + #1283 + #1284** — automated release
  pipeline overhaul: full LTO for shipped binaries, tag-based main→release
  trigger, lsp/wasm assets in the release bundle, robust Cargo.toml version
  bump, reduced binary size profile.

The PR gate's current `--fail-under-lines` is **87** (with renderer/parser
ignore regex). The "90% target" in CLAUDE.md is aspirational, not enforced.

### 2.3 Visual-audit pass — 20 P0s filed
PR **#1307** committed a three-document visual audit
(`docs/internal/visual-audit/2026-05-28-{control-flow,structural,specialized}.md`)
that triaged the full PNG corpus by family bucket and filed **20 P0 issues** plus
25+ P1/P2 — that is the inventory the three audit-drain PRs (#1310 #1311 #1312)
have been working through. As of audit time, 5 of 20 P0s closed via #1310; the
other 10 are queued behind #1311 and #1312 (open, expected to merge); the
remaining 5 (#1297 sequence activation, #1296 creole chapter-22, #1298 mindmap
multi-line, #1292 usecase system boundaries, #1291 usecase actor generalization)
are not yet in flight.

### 2.4 Coverage uplift to a 90% target (gate is still 87)
**#1286** rescued 4,789 lines of abandoned coverage tests under
`coverage/uplift-to-90`. The PR title says "to 90%" but the actual gate in
`scripts/check-all.sh` and `.github/workflows/pr-gate.yml` is still
`--fail-under-lines 87`. The 90% number is the directional target; bumping the
gate is tracked separately (open issue #700: "ratchet coverage gate 87→89").

The coverage ignore regex still excludes the renderer:
`src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\.rs|src/(frontend|normalize|parser|render|specialized)/.*\.rs`.
**P5 from the prior audit is therefore not closed** — the coverage gate has
ratcheted up on the CLI / language service / API / theme / output surfaces, but
the renderer, parser, and normalize layers still have no line-coverage floor.

### 2.5 Issue board audit
The board was drained from **61 open → 30 open** over the audit window.
Standing-epic noise (closed epics still shown as open) was reconciled; #1183
was retired in #1257 after the raw-syntax taxonomy split landed.

### 2.6 File-size guardrail tightened
`scripts/check_rust_file_sizes.py` now enforces **600 LOC** for authored Rust
files (4 files allowlisted: three generated icon tables — bootstrap, material,
openiconic — and `src/parser/family_declarations.rs` at 607, the +23 LOC
visibility annotation overshoot from the parser-unmonolith refactor). PR #1275
split 5 files that had drifted over the line during the parity waves.

### 2.7 Lefthook reorganized + drift contract test
`lefthook.yml` is now lean: fmt, file-size, renderer boundaries on pre-commit;
`cargo check --all-targets --all-features --locked` on pre-push, plus the same
two guards again. A new structural drift contract test under
`tests/*_contract_audit.rs` ensures the renderer-artifact API, branch
protection script, release contract, docs harness, and ecosystem rollout all
match their documented contracts.

---

## 3. Current state — subsystem diagnostic

This section walks each major directory under `src/` and reports its current
shape, what changed since the forensic audit, and where the open risks sit.
Sizes are from `wc -l` on 2026-05-29. Path-relative file counts in parentheses.

### 3.1 `src/parser/*` — 28 files, ~9,400 LOC

**Status:** the 34-`include!` monolith is **gone**. Top of the prior audit's
priority list, P1, is now done in structure (not in spirit — see below).

The parser is now a real directory with submodules:

- `src/parser/core/` — `families.rs` (330), `blocks.rs` (93), `mod.rs` (11).
  The actual diagram-family routing.
- `src/parser/state/` — `declaration.rs`, `block.rs`, `transition.rs`, `mod.rs`.
- `src/parser/activity/` — `conditions.rs`, `notes.rs`, `style.rs`,
  `old_style.rs`, `mod.rs`.
- `src/parser/gantt/` — `calendar.rs`, `tasks.rs`, `mod.rs`.
- `src/parser/tests/` — six test fixtures live in-tree.

Plus 22 top-level files covering directives, family declarations, sequence
parsing, component groups, multiline, sprites, timing, etc. The biggest
authored files are `family_declarations.rs` (607, allowlisted), `directives.rs`
(591), `core_preprocessed.rs` (600), `sequence_keywords.rs` (551),
`sequence_messages.rs` (521), `family_scopes.rs` (498), `family_relations.rs`
(492), `component.rs` (482).

**What survives from the prior audit:**

- **Determinism + no-panic invariant — verified clean.** Spot-grep across
  `src/parser/*.rs`, `src/parser/core/*.rs`, `src/parser/state/*.rs`,
  `src/parser/activity/*.rs`, `src/parser/gantt/*.rs` for `unwrap()` / `expect(` /
  `panic!` / `unreachable!` returned **zero hits** outside `tests/`.
- **Quote-strip bug pattern.** #1289 was the visible one this wave
  (`clean_component_group_label` leaked stray `"` from quoted package names
  with stereotypes); two earlier waves fixed similar leaks. The pattern keeps
  recurring because quoted names + stereotypes + colors + aliases flow through
  family-specific cleaners with subtle return-points. There is no single
  "strip quoted identifier with attached metadata" helper. **Open architecture
  smell.**
- **C4 family detection.** #1252, #1254, #1256 all touched leading-bang macro
  and "unknown leading lines" recovery for C4. Family detection has hardened
  but is still a regex/string-matching pipeline.

**New risks since prior audit:**

- The 600-LOC files (`directives.rs` 591, `core_preprocessed.rs` 600,
  `sequence_keywords.rs` 551, `sequence_messages.rs` 521) sit one parity wave
  away from tripping the file-size guardrail. There is real continued split
  pressure here.
- `src/parser/sequence.rs` is **1 line** — a `pub use` shim. The real content
  lives in `sequence_keywords.rs`, `sequence_messages.rs`, `sequence_participants.rs`.
  Same anti-pattern that the prior `family.rs` had: the named "obvious entry
  point" file is empty.

### 3.2 `src/preproc/*` — 32 files, ~6,270 LOC

**Status:** preprocessor was decomposed extensively. Layout is now:

- `src/preproc/mod.rs` (161) — public surface.
- `src/preproc/control.rs` (587) — top-level directive flow.
- `src/preproc/control/` — `flow.rs` (261), `entrypoint.rs` (45),
  `callables.rs` (32), `source_map.rs` (58), `url.rs` (55).
- `src/preproc/includes/` — `resolution.rs` (471), `stdlib.rs` (445),
  `expr.rs` (400), `directives.rs` (190), `url.rs` (196), `target.rs` (93),
  `paths.rs` (163), `wasm.rs` (95), `diagnostics.rs` (39), `mod.rs` (32).
- `src/preproc/builtins/` — `dispatch.rs` (530), `callable.rs` (414),
  `constructors.rs` (318), `json.rs` (216), `color.rs` (174), `regex.rs` (168),
  plus `datetime.rs`, `collections.rs`, `value.rs`, `scanner.rs`,
  `dispatch_strings.rs`, `mod.rs`.
- `src/preproc/macros/` — `mod.rs` (403), `expand.rs` (223),
  `definelong.rs` (43).

**What landed:**

- **`!definelong` is now supported** (P15 closed) — wave-4 (#1266).
  Body is captured between `!definelong NAME(params)` and `!enddefinelong`,
  parsed into a `Macro` with body lines preserved.
- **`!function` + `!procedure`** — wave-12 (#1274). Builtins dispatch through
  `builtins/dispatch.rs` with arithmetic, string ops, JSON helpers,
  collection helpers, datetime helpers, and a regex helper.
- **Arithmetic** in macro expressions handled by `builtins/expr.rs` in
  `includes/expr.rs` (operator precedence, parens, integer + decimal).
- **JS-side `!include`** — `site/static/js/editor.js` lines 19-410 still owns
  a parallel `resolveIncludes` implementation in 92 lines of JavaScript.
  Brief grep at line 359-410 confirms the dual code path is still alive. The
  WASM module exports an authoritative resolver that the site does not call.
  **P4 from prior audit is partly addressed** (wave-3 brought "JS↔Rust
  parity" work) but no parity-test contract pins the two implementations
  together. **Divergence risk persists.**

**Known smells:**

- `dispatch.rs` (530 LOC) is the biggest authored preproc file. Still in
  bounds of the 600 guardrail but it's one wave away.
- `includes/resolution.rs` (471) carries the URL caching + stdlib lookup
  logic in one file; the file-size pressure compounds when a new include
  protocol lands.

### 3.3 `src/normalize/*` — 33 files, ~12,340 LOC

**Status:** the family-specific normalizer pattern is fully laid out. The
typed unknown/unsupported/deferred taxonomy from wave-1 (#1150) and the
raw-syntax split (#1257) are in place — `StatementKind::Unknown(String)`
fallback in normalizers is gone.

Layout:

- `src/normalize/mod.rs` (254), `common.rs` (452).
- `src/normalize/family/` — 15 files split by concern: `extended.rs` (551),
  `stub.rs` (594), `relations.rs` (233), `mindmap.rs` (335), `tree.rs` (472),
  `nodes.rs` (268), `visibility.rs` (482), `timing.rs` (95),
  `directives.rs` (56), plus `extended/` (activity, component, styles, timing).
- `src/normalize/sequence/` — 9 files: `lifecycle.rs` (418),
  `participants.rs` (236), `messages.rs` (237), `state.rs` (252),
  `groups.rs` (182), `autonumber.rs` (179), `style.rs` (176),
  `pagination.rs` (61), `validation.rs` (22), `mod.rs` (92).
- `src/normalize/timeline/` — 5 files: `tasks.rs` (359), `schedule.rs` (258),
  `calendar.rs` (289), `chronology.rs` (174), `dates.rs` (138).
- Per-family normalizers: `state.rs` (369), `state/nodes.rs` (449),
  `archimate.rs` (335), `wire.rs` (494), `nwdiag.rs` (460),
  `board_files.rs` (233), `chen.rs`, `sdl.rs`, `regex.rs`, `ebnf.rs`,
  `stdlib.rs`, `structured.rs`.

**Graceful-degradation** is wired (P3 closed) — eleven `// Graceful
degradation: skip the unsupported line and emit a` comments visible across the
family normalizers. Confirmed by grep in `chen.rs`, `timeline.rs`,
`family/extended.rs`, `family/tree.rs`, `family/stub.rs`, `state.rs`,
`stdlib.rs`, `sequence/state.rs`. A `// Graceful degradation` line was also
seen in `common.rs:147`.

**One cross-layer violation:** `src/normalize/wire.rs` contains
`use crate::parser;` — the only file in `src/normalize/*` that imports the
parser layer. See §4 for details.

**Largest normalize files:** `family/stub.rs` (594), `family/extended.rs` (551),
`wire.rs` (494), `family/visibility.rs` (482), `family/tree.rs` (472),
`nwdiag.rs` (460), `common.rs` (452), `state/nodes.rs` (449),
`sequence/lifecycle.rs` (418). None over 600.

### 3.4 `src/render/*` — 84+ files, ~30K LOC (renderer surface)

**Status:** this is where most of the wave work happened. Layout:

- `src/render/mod.rs` (~200 LOC entry point).
- `src/render/family.rs` is **36 lines** — a pure `mod`/`pub use` shim. The
  real content lives in `src/render/family/` (25 files, ~9K LOC). The
  prior-audit fix to CLAUDE.md ("there is no god file") still holds.
- `src/render/family/` — 25 files. Biggest: `box_grid.rs` (613),
  `class_node_render.rs` (599), `box_grid_edges.rs` (578), `tree.rs` (534),
  `class_render.rs` (532), `node_shapes.rs` (528), `class_smart_shapes_impl.rs`
  (474), `tree_scene.rs` (453), `cloud_icons.rs` (447), `class_relations.rs`
  (405), `class_relation_labels.rs` (392), `family_node_shapes.rs` (367),
  `class_layout.rs` (361), `projections.rs` (372), `class_members.rs` (342),
  `box_grid_labels.rs` (315), `c4_nodes.rs` (313), `class_smart_shapes.rs`
  (269), `class_routing.rs` (277), `group_frames.rs` (210),
  `box_grid_frames.rs` (147), `box_grid_ports.rs` (139),
  `box_grid_canvas.rs` (133), `class_metadata.rs` (106), `class_types.rs` (47).
- `src/render/sequence/` — 9 files. `messages.rs` (483), `metadata.rs` (354),
  `dimensions.rs` (372), `notes.rs` (321), `participants.rs` (208),
  `lifecycle.rs` (152), `structures.rs` (193), plus `scene/` subdir
  (`labels.rs` + scene file).
- `src/render/state/` — 9 files. `node_render.rs` (588), `scene.rs` (388),
  `edges.rs` (352), `layout.rs` (369) + `layout/sizing.rs`, `nodes.rs` (216),
  `labels.rs` (159), `projection.rs` (97), `types.rs` (96).
- `src/render/activity/` — 11 files. `layout/flow/mod.rs` (594),
  `nodes.rs` (516), `mod.rs` (452), `scene.rs` (433), `arrows.rs` (392),
  `swimlanes.rs` (216), `branches.rs` (140), plus `layout/` subdir.
- `src/render/timing/` — `model.rs` (357), `messages.rs` (407),
  `svg_emit.rs` (411), `rows.rs` + `rows/` subdir, `axes.rs` (199).
- `src/render/timeline/` — `gantt.rs` (582), `gantt_scene.rs` (270),
  `chronology.rs` (358), `dates.rs` (281), `rows.rs` (337), `scale.rs` (287),
  `details.rs` (220), `util.rs` (78).
- `src/render/graph_layout/` — `mod.rs` (200), `coordinates.rs` (398),
  `crossing.rs` (272), `groups.rs` (348), `rank.rs` (291), `router.rs` (593),
  `router/channels.rs` (154), `router/contract.rs` (67),
  `router/obstacles.rs` (70), `scene.rs` (398), `tests.rs` (600).
- `src/render/specialized/` — chart, nwdiag, sdl, archimate, ebnf, regex,
  math, ditaa.
- `src/render/mindmap/` — `tree.rs` + `wbs.rs` + scenes + style + labels +
  nodes.
- `src/render/salt/` — `layout.rs` (367), `model.rs` (336), `text.rs` (244),
  `parsing.rs` (177), `style.rs` (218), `transform.rs` (160), plus
  `widgets.rs` (252) and `widgets/` subdir.
- `src/render/validate/` — 7 files. `mod.rs` (222), `invariants.rs` (380),
  `geometry.rs` (334), `report.rs` (100), `metrics.rs` (148),
  `svg_hooks.rs` (194), `types.rs` (74).
- `src/render/data/` — `model.rs`, `parse.rs`, `svg.rs` (JSON/YAML render).
- Per-family entry files: `chen.rs` (578), `wire.rs` (433), `timing.rs`,
  `timeline.rs`, `salt.rs`, `sequence.rs` (575), `state.rs`, `stdlib.rs`,
  `mindmap.rs`, `data.rs`, `board_files.rs`.
- Geometry helpers: `geometry.rs` (217), `layout_constants.rs` (~150),
  `text.rs`, `text_family_misc.rs`, `text_metrics.rs`, `text_output.rs`,
  `text_specialized.rs`, `text_timeline.rs`, `svg.rs`.

**Key state changes since prior audit:**

- **Typed RenderScene migration is broad.** 41 files mention `RenderScene` —
  every family entry file (`board_files`, `chen`, `data`, `salt`, `stdlib`,
  `timing`, plus `family/box_grid`, `family/class_render`, `activity/scene`,
  `state/scene`, `sequence/scene`, `mindmap/scene`, `mindmap/wbs_scene`,
  `timeline/gantt_scene`, `timeline/chronology`, `specialized/archimate_scene`,
  `specialized/sdl_scene`, `specialized/chart`, `specialized/regex`,
  `specialized/math`, `specialized/ebnf`, `specialized/ditaa`,
  `specialized/nwdiag/scene`, plus `graph_layout/scene`, `family/tree_scene`,
  `wire`). The remaining outliers that still emit SVG without a typed scene
  are concentrated in the older specialized families (regex, math, ebnf,
  ditaa). **P2 is substantially complete** for the graph and timeline-shaped
  families. The wholesale `_artifact` + `_svg` pair API in `src/render/*.rs`
  exposes both shapes per family.
- **The SVG-regex validator is deleted** (P2 finale, #1265).
  `src/render/validate/report.rs` confirms the geometry checks now run against
  the typed `RenderScene` via `scene.validate_scene()`. The remaining SVG-string
  helpers in `validate/svg_hooks.rs` are **correction-only**
  (viewBox expansion, label-background `<rect>` insertion) — they mutate the
  SVG string but no longer probe it for geometry. The doc-comment on
  `run_with_scene` makes the new contract explicit.
- **Recursive bbox propagation** — #1312 added
  `src/render/graph_layout/groups.rs` (348 LOC) with recursive parent-bbox
  growth so deeply-scoped package frames (e.g. `acme::hr::payroll`) nest
  correctly inside their ancestor frames. Was the unblocker for #1287
  (class/32 deep packages) and #1290 (deployment Kubernetes 3-level nesting).
- **Channel routing is real and tested.** `graph_layout/router.rs` (593) +
  `router/channels.rs` (154) own the orthogonal channel construction; tests
  in `graph_layout/tests.rs` (600) include determinism assertions. The router
  is consumed by class, component, deployment, c4. Sequence and state still
  run their own routing. **#592 and #593 from the active follow-ups list are
  partially landed, not done.**

**Remaining smells:**

- `chars × 7` text-width estimation still lives in **at least 7 places**:
  `src/render/chen.rs` (×2), `src/render/graph_layout/scene.rs` (×3),
  `src/render/svg.rs`, `src/render/wire.rs`. Prior audit's P6 (~5 layout
  engines, 9+ text-width estimates) is **not closed**. The shared geometry
  contract still hasn't pulled these into a single source of truth.
- The class-family files are heavy. `class_*` is 9 files in `family/`
  totalling ~3K LOC. The split is by concern (layout, members, metadata,
  node-render, relation-labels, relations, render, routing, smart-shapes-impl,
  smart-shapes, types) but several files brush the 600 ceiling
  (`class_node_render.rs` at 599, one bug fix from tripping the guardrail).

### 3.5 `src/render_core/` — 4 files, ~1.2K LOC

The typed-scene contract module. `src/render_core.rs` is the 474-line root
exposing `Point`, `Size`, `Rect`, `Polyline`, `Segment`, `RenderScene`,
`SceneNode`, `SceneEdge`, `SceneGroup`, `SceneLabel`, etc., plus
`mod backend; mod issues; pub mod validate;`. The renderer-boundary guard
(`scripts/check_renderer_boundaries.py`) enforces that `render_core` does **not**
import `frontend`/`parser`/`model`/`api`/`render`/`output` — i.e. it stays
neutral. Confirmed by grep: zero forbidden imports in `src/render_core/`.

**Status of typed-scene migration:** `RenderScene` populated by 41 files (see
§3.4). The migration is now wide enough that the `render_core` API is the
*assumed* shape for any new family. New entries are added through
`SceneNode`/`SceneEdge`/`SceneGroup`/`SceneLabel`. The wire-up between scene
construction and validation runs through `render_core::validate.rs` (548 LOC),
which is the typed analogue of the old SVG-regex validator.

### 3.6 `src/theme/*` — 21 files, ~4,800 LOC

**Status:** the shared style cascade is **largely complete** (#1184 closed
in wave-6). Files:

- `cascade.rs` (415) — the top-level `GraphStyleCascade` aggregating skinparam +
  theme + stereotype + `<style>` layers.
- `shared_cascade.rs` (530) — the cascade implementation. Header still says
  `# Migration status — TODO(#1184)`; #1184 is **closed** but this top-of-file
  TODO marker survives. Cheap cleanup.
- `shared_cascade_families.rs` (260),
  `shared_cascade_tests.rs` (389),
  `shared_cascade_family_tests.rs` (293).
- `effective.rs` (289), `styles.rs` (439), `apply.rs` (233), `color.rs` (251),
  `presets.rs` (597), `values.rs` (67), `catalog.rs` (43).
- `src/theme/skinparam/` — per-family skinparam mappers: `component.rs` (376),
  `sequence.rs` (316), `generic.rs` (308), `class.rs` (298), `chart.rs` (78),
  `activity.rs` (83), `timing.rs` (82), `state.rs` (72), `helpers.rs` (34).
- `src/theme/skinparam.rs` (26) — re-export shim.
- `src/theme/presets.rs` (597) — five PlantUML-compatible named themes from
  #1308: plain, cerulean, cyborg, hacker, materia.

**Open architecture work:** #450 (extend theme presets to all diagram families,
not just sequence) — five themes ship as of #1308 but the per-family preset
coverage is uneven. Class/component/deployment carry the most preset
infrastructure; specialized families (timing, chart, gantt) still inherit
generic settings.

### 3.7 `src/api/*` — 8 files, ~1,000 LOC

The public API surface. Files:

- `mod.rs` (29) — public re-exports.
- `pipeline.rs` (168) — `parse_with_pipeline_options`,
  `preprocess_with_pipeline_options`.
- `render.rs` (254) — the public SVG compatibility shim that calls the typed
  artifact API behind the scenes (renderer-boundary allowlist entry).
- `render_scene.rs` (183) — typed scene access.
- `render_summary.rs` (99) — scene summary JSON shape.
- `markdown.rs` (106) — markdown fence extraction.
- `lsp.rs` (28) — `lsp_capabilities` re-export.
- `types.rs` (136) — `DiagramFamily`, `DiagramInput`, `CompatMode`,
  `FrontendSelection`, etc.

Architecture-wise the API layer is clean and small. The "artifact-vs-SVG"
shape is the right cut — internal callers consume artifacts, the
backwards-compat SVG shape stays as a thin adapter.

### 3.8 `src/source.rs` (259) + `src/frontend/*` (8 files)

`src/source.rs` defines `Span`, `MappedSpan`, `SourceMap`, `SourceFile`,
`SourceId`. Used by 75 files. The frontend layer translates non-default input
surfaces (Mermaid, PicoUML) into PlantUML-shaped source before the shared
parser runs.

- `src/frontend/mod.rs` (137) — `FrontendResult`, source-map plumbing.
- `src/frontend/picouml.rs` (321) — PicoUML adapter.
- `src/frontend/mermaid/{mod,flowchart,sequence,class,state,er,common}.rs` —
  per-mermaid-family adapters.

The source-map plumbing is real. `MappedSpan` flows through preprocess, the
include resolver, Mermaid lowering, PicoUML lowering, and the typed-unknown
taxonomy. PR #1250 ("Thread include origins through source-mapped
diagnostics") shipped the last big chunk of that wiring.

### 3.9 `crates/puml-wasm/*`

Single-crate WASM build target. `src/lib.rs` — exposes `compile`,
`render_svg`, language service surfaces. The forensic audit noted that the
site does not call the WASM module's exported include resolver — that claim
was not re-verified in this audit beyond confirming `site/static/js/editor.js`
still owns its own `resolveIncludes` at lines 19-410, the WASM API exposes
the authoritative resolver, and no parity test pins them together.

### 3.10 `site/` — Zola static site

The in-browser editor. JS files under `site/static/js/`:

- `editor.js` (593) — editor wiring, owns the JS `!include` resolver
  (lines 19-410).
- `wasm-renderer.js` (98) — WASM loader and render dispatch.
- `puml-tokens.js` (169) — JS tokenizer, parallel to the Rust tokenizer.
- `gallery.js` (125), `home.js` (106), `inline-fence-preview.js` (196),
  `manifest.js` (53), `puml-lang.js` (23).

**Divergence risk persists.** Two parallel tokenizers (`puml-tokens.js` and
`src/parser/`) and two parallel include resolvers (`editor.js` lines 19-410
and `src/preproc/includes/`) without a parity contract.

### 3.11 `agent-pack/` — MCP server + skills + bins

- `agent-pack/bin/puml-mcp` (389 LOC of Python) — MCP server.
  Tool list: `puml_check`, `puml_diagnostics`, `puml_render_svg`,
  `puml_render_file`, `puml_render_png`.
- `agent-pack/bin/puml-lsp` — bash wrapper around the LSP binary.
- `agent-pack/skills/` — `puml-class-author`, `puml-sequence-author`,
  `puml-sequence-reviewer`, `puml-writing-guide`.

The MCP surface is **broader than diagnostics-only** (the prior audit's claim
that MCP was "diagnostics-only" was already a slight understatement; it now
clearly exposes render). Still missing: hover, completion, semantic tokens,
formatting — the rich language-service surface is LSP-only.

---

## 4. Layer-boundary violations

The renderer-boundary guard (`scripts/check_renderer_boundaries.py`) is wired
into pre-commit, pre-push, and `scripts/check-all.sh`. It enforces a focused
slice of the target pipeline:

```text
frontend -> preprocess -> parse/lower -> normalize -> build_scene -> validate_scene -> backend
```

What it actually enforces, as of audit:

- **`render-core-neutral`**: `src/render_core/*` must not import
  `frontend`/`parser`/`model`/`api`/`render`/`output`. Status: clean.
- **`artifact-boundary`**: `render_svg_pages_from_model` may only be called
  from `src/api/render.rs`, `src/api/mod.rs`, `src/lib.rs` (the compatibility
  shim). Everywhere else must consume `RenderArtifact`. Status: enforced.
- **`svg-adapter-boundary`**: only `src/api/render.rs` may call
  `render_*_svg` family-specific functions; CLI / WASM / api / bin call the
  typed artifact API. Status: enforced.
- **`artifact-constructor-boundary`**: only `src/output/contract.rs` may
  construct `RenderArtifact { ... }` literals or `= RenderArtifact { ... }`;
  everyone else uses the constructors. Status: enforced.
- **`artifact-state-boundary`**: only `src/output/contract.rs` and
  `src/render/mod.rs` may mutate `.scene_availability` /
  `.invariant_report` fields. Status: enforced.
- **`output-conversion-boundary`**: only `src/output.rs` and
  `tests/visual_regression.rs` may use `resvg::`, `svg2pdf::`,
  `image::codecs`, `image::ImageEncoder`. Status: enforced.

**Cross-layer imports actually found:**

Grep `use crate::parser` across `src/render` → **zero hits**. Render does
not reach back into parser.
Grep `use crate::render` across `src/parser` → **zero hits**.
Grep `use crate::render` across `src/normalize` → **zero hits**.
Grep `use crate::normalize` across `src/parser` → **zero hits**.
Grep `use crate::preproc` across `src/render` or `src/normalize` → **zero hits**.

The one **violation** found:

- `src/normalize/wire.rs` contains `use crate::parser;` inside a function body.
  This is the only file in `src/normalize/*` that imports the parser layer.
  Wire-family normalization is calling back into a parser helper. **Worst
  offender, and the only offender.**

Grep `use crate::ast` across `src/render` → 6 hits, all benign:
- `src/render/family/class_node_render.rs:1` — imports `MemberModifier`.
- `src/render/family/class_members.rs:1` — imports `MemberModifier`.
- `src/render/family/tree.rs:1` — imports `MemberModifier`.
- `src/render/sequence/notes.rs:1` — imports `NoteKind`.
- `src/render/family/tree_scene.rs:307` — imports `DiagramKind` (test
  context).
- `src/render/mod.rs:1` — re-exports `DiagramKind`.

These are model-type imports, not behavioral leaks. Acceptable.

**Verdict on layer boundaries:** the renderer-boundary guard catches what
matters and the codebase respects it. The one outlier (`normalize/wire.rs`
→ parser) should become a focused ticket: extract the parser helper into a
shared utility module or move the normalization step earlier in the pipeline.

---

## 5. Deadweight modules

`cargo machete` is not installed in this environment, so deadweight detection
relied on grep + manual review. Findings:

### 5.1 Confirmed dead dependencies

- **`winnow = "1.0"`** in `Cargo.toml:83` — **zero usages** in `src/` or
  `crates/`. The forensic audit flagged this; it has not been removed.
  One-line cleanup.

### 5.2 `#[allow(dead_code)]` survey

14 hits across `src/`:

- `src/render/family/box_grid.rs:19, 23` — two allowlisted struct fields
  on internal staging types. Plausible reserved-for-future-use.
- `src/render/graph_layout/mod.rs:56, 60, 63, 72, 81, 95, 198` — six
  allowlisted items plus one file-level allow on a helper. These likely
  date back to the channel-router migration and need re-validation.
  **Specific candidates for retirement:** the helper at line 198 is the
  most suspicious — if it has been unused since the channel router landed,
  it should be deleted, not allowed.
- `src/render/graph_layout/crossing.rs:212` — one allow on a helper.
- `src/render/geometry.rs:94` — one allow.
- `src/render/wire.rs:433` — one allow.
- `src/preproc/includes/url.rs:190` — one allow.
- `src/parser/activity/style.rs:159` — one allow.

These should be triaged: each one is either a "kept around for the next
refactor" claim that should become a ticket, or genuinely dead code that
should be deleted.

### 5.3 `#[cfg(test)]` modules at file level

73 hits — most are test submodules colocated with code, which is fine.
No "whole file is test scaffold" outliers observed in this pass.

### 5.4 TODO / FIXME / XXX density

Only **2** TODO/FIXME markers in `src/`:

- `src/specialized/shared.rs:37` — `// consume @startXXX line, possibly with a title`.
  Not a TODO, a comment with the word "XXX" — false positive.
- `src/theme/shared_cascade.rs:24` — `//! # Migration status — TODO(#1184)`.
  **#1184 is closed**; this comment header should be retired or updated.

Three "XXX" hits in tests are placeholder text in salt fixtures, not TODOs.

This is **a remarkably clean codebase by TODO-density**, matching the
prior audit's finding.

### 5.5 Stale comments referencing closed work

- `src/theme/shared_cascade.rs:24` references closed #1184.
- `src/render/graph_layout/mod.rs` likely has stale "Stage 3" claims;
  prior audit flagged this. Not re-verified in detail.
- `src/parser/family_declarations.rs` is allowlisted in the file-size
  guardrail with a comment pointing to #1258 — that ticket is still open
  but the rationale ("pre-existing large module; +23 lines from refactor
  visibility annotations") is no longer the right framing now that the
  parser unmonolith is done. Should be split.

### 5.6 Uncovered files

The coverage ignore regex still excludes the entire renderer, parser,
normalize, and specialized layers. **Files in those trees that have zero
test coverage are invisible to the gate.** No on-disk coverage report
was regenerated for this audit; the most recent
`docs/benchmarks/render_check_latest.json` is from 2026-05-28 and reports
338/341 passed (3 excluded, 0 failed) — that is a docs-drift report, not a
coverage report.

---

## 6. The next three strategic migrations

Each recommendation is sketched as: **why now**, **what it touches**,
**ROI / acceptance criteria**, **estimated effort**.

### 6.1 Migration #1 — Unify text-width estimation under a single trait

**Why now.** The prior audit's P6 ("~5 independent layout engines; text-width
estimated `chars × ~7px` in 9+ places that drift") is the one structural debt
that did **not** get touched in the typed-scene + shared-cascade waves. As of
2026-05-29 it still lives in `chen.rs` (×2), `graph_layout/scene.rs` (×3),
`svg.rs`, `wire.rs`, plus the validator's private copy in `validate/metrics.rs`
and several others not enumerated here. Every label-clipping bug in the
visual-audit P0 backlog (#1298 mindmap, #1295 ref-over margin, #1304 composite
action text, #1300 nested if/else label drift) traces back to the same root
cause: layout and validation are each guessing the same number with slightly
different formulas and the answers drift apart.

**What it touches.**

- New module: `src/render_core/text_metrics.rs` exposing a single
  `TextMetrics` trait or pure function: `pub fn estimate_text_width(text: &str,
  font_size: f64) -> f64`. Move the `chars × ~7` formula to one place,
  parametrize by font size.
- Replace the call sites in `chen.rs`, `graph_layout/scene.rs`, `svg.rs`,
  `wire.rs`, `validate/metrics.rs`, plus any others discovered during
  audit-by-grep.
- Add a regression test: `tests/text_width_consistency.rs` that asserts the
  validator and the layout engine see the same width for representative
  strings (ASCII, multibyte, mixed) at the canonical font size.
- Optionally fold the "chars-to-pixels with multibyte awareness" into a
  shared `Span`/`MappedSpan`-aware metric so source-map line lengths stay
  truthful.

**ROI / acceptance criteria.**

- Single source of truth for text width. New estimates are forbidden in
  family renderers (enforce via a `check_text_width_metrics.py` guard or a
  clippy `disallowed_method` lint).
- Visual-audit P0/P1 backlog items rooted in label-clipping
  (#1298, #1295-class, parts of #1304 and #1300) close as the cascade of
  drift dies.
- 7+ duplicated formulas collapse to one.
- Estimated effort: **2-3 days of focused work**. Mostly mechanical
  replacement; the regression test is the real intellectual content.

### 6.2 Migration #2 — Promote `route_channels` to the canonical
edge-routing layer for sequence + state + activity

**Why now.** The channel router (`src/render/graph_layout/router.rs`,
593 LOC, plus `router/channels.rs` 154) is real, deterministic, tested, and
consumed by **class, component, deployment, c4**. The active follow-ups
list (#592 finish hierarchical graph-layout adoption across node-and-edge
families, #593 converge orthogonal routing on shared route channels) is
exactly this migration. It's been on the list since the original layout-
engine plan; the typed-scene wave bought a clean target shape for it. Right
now sequence still emits 2-point straight polylines (forensic audit flagged
this) and the visual-audit P0 backlog has multiple "edges through nodes" or
"label drift" defects (#1300, #1288, #1290) that the per-family routers
can't fix cleanly.

**What it touches.**

- `src/render/sequence/scene.rs` — currently emits straight 2-point polylines.
  Migrate message-line emission through the channel router. The sequence
  domain is 1-D in y-axis but the activation-bar overlap problem (#1297
  open) is a routing problem the channel router can model.
- `src/render/state/edges.rs` (352 LOC) — state transitions currently
  use a per-family routing path. The recursive composite + history
  pseudostate fixes from #1306 / #1304 / #1305 surfaced enough geometry
  to make state-transition routing through the channel layer realistic.
- `src/render/activity/arrows.rs` (392 LOC) — activity arrow routing has
  its own L/Z-shape per-arrow logic. The nested-if-else routing bug
  (#1300) is the canonical case the channel router solves.
- Migrate per-family routing helpers to call the typed channel router via
  `RouteRequest` / `ChannelRouter` from `graph_layout/router.rs`.
- Add new typed scene fields where missing: each migrated family must
  populate `route_channels` so the validator can check
  "edge does not cross non-endpoint node" via the typed scene rather than
  emit polylines and hope.

**ROI / acceptance criteria.**

- Sequence messages render as channel-routed polylines (still visually
  straight in the common case, but the typed scene knows it).
- Activity nested-if-else (#1300) closes without per-family hacks.
- State composite-transition (#1304) and inter-package class edge
  (#1288) close as the typed channel router gates on
  "no edge through non-endpoint node."
- One layout engine instead of three. The fork tax — "which family is this
  in? what router does it use?" — vanishes for new families.
- Estimated effort: **2 weeks**. Sequence is the most surgical (1-D
  domain, well-understood). State is medium. Activity is the deepest
  because of swimlanes and fork-bars.

### 6.3 Migration #3 — Browser/CLI parity contract for tokenize + `!include`

**Why now.** This is the prior audit's P4. The JS-side `resolveIncludes`
(`site/static/js/editor.js` lines 19-410, 92 LOC of fetch-based include
expansion) and the JS-side tokenizer (`site/static/js/puml-tokens.js`,
169 LOC of token-class regex) both still ship. The WASM module exposes the
authoritative implementations. Nothing pins the two together, and one of the
forensic audit's most-quoted observations was: "the user-facing 'does my
diagram work' surface runs different logic than the CLI, by construction."
72 hours later, that surface still runs different logic. The wave-3 PR
(#1265) brought "JS↔Rust parity" work but did not install a contract test.

**What it touches.**

- Decide the policy: (a) delete the JS implementations and route everything
  through WASM (clean but adds WASM-init latency to the first paint), or
  (b) keep both and install a parity test corpus (`tests/site_parity/`)
  with N representative `.puml` fixtures whose tokenizations must match
  byte-for-byte between the JS tokenizer and the Rust tokenizer, plus
  N fixtures whose `!include` resolution must match.
- New module: `tests/site_parity.rs` or `site/tests/parity.spec.js`
  driving both layers against a shared corpus.
- Wire into the PR gate so a divergence becomes a CI failure.
- If (a) is chosen, this is a deletion PR — much cheaper. If (b), expect
  to discover real divergences and fix them as part of the work.

**ROI / acceptance criteria.**

- "What I see in the editor is what the CLI sees" becomes a tested
  invariant rather than a hope.
- New language features (a new directive, a new comment syntax) carry a
  parity-test requirement, slowing JS-side drift to zero.
- The "site ships parallel logic" footnote retires.
- Estimated effort: **(a) 1-2 days**, **(b) 4-5 days**. Recommendation:
  pick (a) and accept the latency cost; the editor already loads WASM for
  rendering, the marginal cost is small.

---

## 7. Operational debt

### 7.1 Stash backlog

`git stash list` shows **10 stashes**, including:

- `stash@{0}` / `stash@{1}` — WIP on `fix/audit-group-bc-state-sequence`
  branch (the PR #1311 source).
- `stash@{2}` — WIP on `fix/audit-group-a` (PR #1310 source).
- `stash@{3}` — "On coverage/uplift-to-90: preserve before pivot."
- `stash@{4}` — WIP on `coverage/uplift-to-90`.
- `stash@{5}` — "On coverage/uplift-to-90: leak before coverage fix."
- `stash@{6}` — WIP on main (post wave-13).
- `stash@{7}` — "On chore/release-extra-artifacts: tmp worktree state
  for branch switch."
- `stash@{8}` — "On coverage/uplift-to-90: coverage WIP files for next
  branch."
- `stash@{9}` — "On refactor/claude-wave-13: leaks before coverage PR."

**Action:** review and drop. Each stash is a parallel-agent receipt that
should either be promoted to a branch or dropped. As-is, they accumulate
across waves.

### 7.2 Branch backlog

`git branch -a` reports **474 refs**. The clutter includes:

- `backup/fix-issue-957-pre-rebase`,
  `backup/local-main-before-pr-967` — pre-rebase safety branches that
  outlived their purpose.
- `chore/release-*` × 5 — multiple parallel attempts at the same release
  workflow; one of them merged via #1280.
- `chore/visual-spacing-audit`, `chore/wave-5b-layout-split` — pre-wave-12
  parking branches.
- `codex/*` — 12+ branches from the older Codex agent flow that should be
  archived or deleted (these date back to 2026-05-22/23/24).
- `ci/issue-453-pinned-wasm-pack`, `ci/issue-454-oracle-path-triggers`,
  `ci/overhaul-aggressive-teardown`, etc. — CI experiment branches that
  fed wave-CI overhaul PRs.

**Action:** schedule a "branch hygiene" pass:
```bash
gh api repos/:owner/:repo/branches --jq '.[] | select(.protected == false) | .name' \
  | grep -vE '^(main|refactor/claude-wave-migrations)$' \
  | while read b; do gh api repos/:owner/:repo/branches/$b > /dev/null 2>&1 && \
      echo "candidate: $b"; done
```
plus a pass deleting all `codex/*` branches older than 2026-05-25, and a
pass on `backup/*` older than 2 weeks.

### 7.3 Tmp directories and abandoned experiment files

`ls /tmp/*puml*` shows ~22 `.puml` scratch files left over from agent
debugging sessions (`/tmp/c4_*`, `/tmp/chen.puml`, `/tmp/aws_icons_test.puml`,
etc.). One actual directory: `/tmp/jsonyaml-rescue/` — looks like a rescue
workspace that should be archived or deleted.

**Action:** `rm /tmp/*.puml /tmp/jsonyaml-rescue` after confirming nothing
in-flight depends on them. These do not affect the repo but they grow
unbounded.

### 7.4 Dropped TODOs / stale references that should become issues

- `src/theme/shared_cascade.rs:24` — "TODO(#1184)" pointing at a closed
  issue. **Update or delete the marker.**
- `docs/internal/architecture/renderer-refactor-roadmap.md` — references
  #1181, #1182, #1184 as "Active Follow-Ups." #1184 is closed. #1181 still
  open (file-size guardrail enforcement). #1182 — verify state.
- `docs/internal/architecture/layout-engine-vision.md` line 50 — "Stage 1
  — Port assignment (~1 day, +40% crossing reduction)." Channel router is
  live; port assignment is at minimum partly subsumed. Update the doc to
  reflect current stage.
- `CLAUDE.md §1` still says "Hierarchical layout module live at
  `src/render/graph_layout.rs` (stage 1 complete)" — wrong path (it's a
  directory now), wrong stage (router lives), unchanged since the prior
  audit flagged it. **Update.**
- `docs/release-checklist.md` — verify it documents the wave-CI-shipped
  release flow, not the old workflow.
- The "stage 3" comments inside `src/render/graph_layout/mod.rs` (cited by
  prior audit) — re-verify against current code; either delete or replace
  with a real status indicator.

### 7.5 Untracked files in worktree

`git status` in the audit worktree shows:

- `.claude/scheduled_tasks.lock` — agent harness state, gitignored.
- `tests/fixtures/families/valid_salt_bootstrap.svg` — likely a test
  fixture that should be committed or deleted, depending on whether the
  associated test references it.

---

## 8. What to dispatch NEXT

Concrete agent prompts the orchestrator can hand to Sonnet workers in the
next 24 hours. Ranked by impact × tractability.

### Dispatch #1 — Doc fixes (cheap, high leverage)

**Why first.** Sub-half-hour Sonnet job. Fixes mis-targeting for every
agent that will run after.

**Prompt:**
> Update CLAUDE.md §1 to reflect: (a) layout module is `src/render/graph_layout/`
> (a directory, not a file); (b) channel routing is wired into class /
> component / deployment / c4 (not "orthogonal edge routing wired into
> sequence and class diagram families" — sequence still uses straight lines
> and state still has its own routing); (c) coverage gate is 87 (with
> renderer ignore regex), not 85, with a directional target of 90; (d) MCP
> exposes diagnostics + render (not "diagnostics only"); (e) the parser is
> now a real directory with submodules, not a 34-`include!` monolith.
> Also update `docs/internal/architecture/renderer-refactor-roadmap.md`
> "Active Follow-Ups" to remove #1184 (closed) and refresh #1181 / #1182
> status. Delete the TODO(#1184) marker in `src/theme/shared_cascade.rs:24`.
> Open a focused ticket for each remaining stale reference identified.
> 30 minutes max; doc-only PR.

### Dispatch #2 — Delete `winnow` dead dependency

**Why.** One-line `Cargo.toml` change. Closes a forensic-audit P-class
finding that has been open since 2026-05-26.

**Prompt:**
> Delete `winnow = "1.0"` from `Cargo.toml` and run `cargo update -p
> winnow --precise 0 || true` plus `cargo build --release` to verify it was
> truly unused. Commit `chore(deps): remove unused winnow dependency`.

### Dispatch #3 — Branch / stash hygiene pass

**Why.** Operational debt section 7.1 and 7.2. Reduces "what's that branch?"
ambiguity for future agents.

**Prompt:**
> Walk `git stash list` and either restore each WIP to a branch (if it
> represents real in-flight work) or drop it. Walk `git branch -a` and
> delete branches under `codex/*` older than 2026-05-25; archive or delete
> `backup/*` older than 2 weeks; consolidate `chore/release-*` parallel
> attempts. Document the action plan in a comment on Epic #1258 before
> executing.

### Dispatch #4 — Strategic migration #1 (text-width unification)

**Why.** Highest-ROI structural fix on the table. Closes prior-audit P6 and
unblocks 3-4 label-clipping P0/P1 visual-audit items.

**Prompt:**
> Implement migration #1 from
> `docs/internal/audits/2026-05-29-architecture-audit.md` §6.1. Create
> `src/render_core/text_metrics.rs` exposing
> `pub fn estimate_text_width(text: &str, font_size: f64) -> f64`. Migrate
> every `chars().count() * 7` site identified in §3.4 to call it
> (currently in `chen.rs`, `graph_layout/scene.rs`, `svg.rs`, `wire.rs`,
> `validate/metrics.rs`, and any additional sites found by grep). Add
> `tests/text_width_consistency.rs` asserting validator and layout see
> the same width for ASCII, multibyte, and mixed strings at 12px and
> 14px font sizes. PR-per-agent (Flow B); branch
> `refactor/issue-NNN-text-width-unification` after filing the parent
> ticket.

### Dispatch #5 — Drain remaining P0 visual-audit backlog

**Why.** 5 of 20 P0s from the 2026-05-28 audit are not yet in flight:
#1297 sequence activation bars across alt frames, #1296 creole chapter-22
constructs, #1298 mindmap multi-line spacing, #1292 usecase system boundary
frames, #1291 usecase actor-generalization triangle.

**Prompt:**
> Pick #1297 OR #1298 (split across two Sonnets). Each: read the
> referenced fixture in `docs/examples/*`, render to `/tmp/before.png`,
> Read with the vision tool, implement minimum correct fix, render to
> `/tmp/after.png`, Read, then PR-per-agent. Confirm visually before
> blessing baselines. #1297 traces into
> `src/render/sequence/lifecycle.rs` (activation tracking); #1298 traces
> into `src/render/mindmap/labels.rs` (line spacing).

### Dispatch #6 — Strategic migration #3 (browser parity contract)

**Why.** Lower urgency than the text-width unification but lower effort.
Closes the longest-standing structural divergence in the repo.

**Prompt:**
> File a tracking ticket; pick option (a) from
> `docs/internal/audits/2026-05-29-architecture-audit.md` §6.3 (delete the
> JS `!include` + tokenize layers, route through WASM). Verify that the
> WASM module's `resolveIncludes` and tokenizer are exported and
> functional. Update `site/static/js/editor.js` to call the WASM
> resolver in `hasIncludes` branches. Delete `site/static/js/puml-tokens.js`
> if the WASM-side tokenization is performant enough on the editor's
> incremental-syntax-highlight path; otherwise install a parity test corpus
> `tests/site_parity_tokenization.rs` driving both layers against a shared
> fixture set.

### Dispatch #7 — Bump coverage gate from 87 → 89 (open #700)

**Why.** P5 ratchet — directional improvement; tests already exist.

**Prompt:**
> Run `cargo llvm-cov --all-features --package puml --fail-under-lines 89
> --ignore-filename-regex '<current regex>'`. If green, bump
> `scripts/check-all.sh` and `.github/workflows/pr-gate.yml` to 89.
> If red, identify the lowest-coverage modules not in the ignore regex,
> add meaningful tests, then bump. Close #700.

### Dispatch #8 — Strategic migration #2 (channel routing for
sequence / state / activity)

**Why.** Highest structural value but largest effort. Start with **sequence**
as the surgical first step.

**Prompt:**
> Implement migration #2 §6.2 stage 1: migrate
> `src/render/sequence/scene.rs` message-line emission to populate
> `route_channels` via the typed channel router from
> `src/render/graph_layout/router.rs`. Visual output must be byte-identical
> for the existing baselines (verify via `cargo test --release --test
> visual_regression`). The new typed scene must expose enough channel
> data for the validator to assert "no message line crosses a participant
> head" or "no message ends inside an activation bar." File child issue
> under #592.

---

## 9. Closing observations

The 72 hours between 2026-05-26 and 2026-05-29 represent the most
architecturally productive window in the repo's history. The forensic audit's
biggest structural bets — typed-scene migration (P2), graceful degradation
(P3), JSON/YAML preserve_order (P12), `DeterminismMode` deletion (P8),
`!definelong` support (P15), parser unmonolithing (P1), shared style
cascade (#1184) — **all moved**. P2 and P3 are substantially closed; P1
is structurally done; P8, P12, P15 are closed. P4 (browser parity), P5
(renderer coverage gate), P6 (text-width drift) are the remaining structural
debts from the original list.

Wave-by-wave parity work continued in parallel and added five named themes,
five DDD smart-default stereotype mappings, gantt and nwdiag verification,
creole blocks, timing analog rendering, IE entity notation, salt widgets,
preproc `!function`/`!procedure`. The PNG corpus visual audit on 2026-05-28
generated 20 P0s; 5 are closed, 10 are in two PRs awaiting merge, 5 remain
unstarted.

The CI overhaul shaved PR-gate wall time from ~8-9 minutes toward a sub-4
minute target. File-size guardrail tightened to 600 LOC. Renderer-boundary
guard now enforces five distinct boundary rules. Lefthook reorganized
toward fast pre-commit + thorough pre-push.

The codebase remains determinism-clean, panic-clean on user input, and
TODO-clean. The visible TODOs in `src/` are two (one a typo, one a stale
reference to a closed ticket). 1.7K-plus tests, the structural drift
contract test family, the file-size guardrail, the renderer boundary
guard, and the source-map plumbing all give future agents real handles to
navigate by.

The three migrations recommended in §6 are the **highest-ROI next moves**.
Migration #1 (text-width unification) closes the prior-audit P6 and unblocks
a cascade of label-clipping defects. Migration #2 (channel routing across
sequence/state/activity) closes #592 and #593 and removes the fork-tax for
future families. Migration #3 (browser parity contract) closes the
longest-standing structural divergence and is the cheapest of the three.

After these land, the architecture-first thesis from 2026-05-26 will be
substantially complete and the repo can resume parity-first work on a
clean base — which is the original sequencing the owner picked.

---

*This file is a point-in-time architectural snapshot. If a future code reading
disagrees with a claim here, fix or retire the row instead of trusting the
doc.*
