# PlantUML Language Reference — Support Matrix

**Source:** PlantUML Language Reference Guide v1.2025.0 (Feb 2025), 607 pages.
**Goal:** 1:1 feature parity with upstream PlantUML. (Pico-UML and Mermaid are explicitly out of scope — separate languages.)
**Method:** 10 audit agents fanned out across the 27 chapters of the reference, each reading the relevant slice of the extracted text, grepping the puml repo, and producing a per-chapter audit with file:line evidence.

## Status legend

| Tag | Meaning |
|---|---|
| ✅ | Supported — parses and renders the canonical form correctly |
| 🟡 | Partial — parses but renders degraded, OR only some sub-forms work |
| ❌ | Missing — parser rejects, drops silently, or there is no code path |

## Baseline Numbers

The counts below are the historical baseline produced by the initial fan-out
audit. They are useful for orientation, but they are not a live release
dashboard. Recent merged work may make individual chapter rows stale until the
matching per-chapter audit file is refreshed.

Across the 27 chapters the initial audits scored roughly:
- **~178 ✅ fully supported features**
- **~145 🟡 partial features**
- **~190 ❌ missing features**

At that snapshot, roughly 63% of documented PlantUML features had at least
partial code support, and roughly 35% were confidently 1:1. For current planning,
trust the per-chapter audit evidence over these aggregate percentages.

## Per-chapter support matrix

| # | Chapter | ✅ | 🟡 | ❌ | Audit |
|---|---------|----|----|----|-------|
| 1 | Sequence Diagram | 28 | 14 | 6 | [ch01-sequence.md](audit/ch01-sequence.md) |
| 2 | Use Case Diagram | 15 | 2 | 1 | [ch02-usecase.md](audit/ch02-usecase.md) |
| 3 | Class Diagram | 22 | 13 | 11 | [ch03-class.md](audit/ch03-class.md) |
| 4 | Object Diagram | 5 | 2 | 1 | [ch04-object.md](audit/ch04-object.md) |
| 5 | Activity (legacy) | 0 | 2 | 10 | [ch05-activity-legacy.md](audit/ch05-activity-legacy.md) |
| 6 | Activity (new) | 6 | 12 | 12 | [ch06-activity-new.md](audit/ch06-activity-new.md) |
| 7 | Component | 12 | 5 | 2 | [ch07-component.md](audit/ch07-component.md) |
| 8 | Deployment | 14 kw + 5 sec | 10 sec | 15 kw + 2 sec | [ch08-deployment.md](audit/ch08-deployment.md) |
| 9 | State | 14 | 5 | 6 | [ch09-state.md](audit/ch09-state.md) |
| 10 | Timing | 4 | 12 | 13 | [ch10-timing.md](audit/ch10-timing.md) |
| 11 | JSON | 7 | 4 | 3 | [ch11-json.md](audit/ch11-json.md) |
| 12 | YAML | 3 | 1 | 4 | [ch12-yaml.md](audit/ch12-yaml.md) |
| 13 | nwdiag | 12 | 3 | 2 | [ch13-nwdiag.md](audit/ch13-nwdiag.md) |
| 14 | Salt (Wireframe) | 12 | 9 | 3 | [ch14-salt.md](audit/ch14-salt.md) |
| 15 | ArchiMate | 6 | 2 | 1 | [ch15-archimate.md](audit/ch15-archimate.md) |
| 16 | Gantt | 22 | 10 | 7 | [ch16-gantt.md](audit/ch16-gantt.md) |
| 17 | MindMap | 11 | 2 | 0 | [ch17-mindmap.md](audit/ch17-mindmap.md) |
| 18 | WBS | 6 | 4 | 2 | [ch18-wbs.md](audit/ch18-wbs.md) |
| 19 | Math | 3 | 1 | 1 | [ch19-math.md](audit/ch19-math.md) |
| 20 | Information Engineering | 1 | 2 | 2 | [ch20-ie.md](audit/ch20-ie.md) |
| 21 | Common Commands | 14 | 7 | 1 | [ch21-common.md](audit/ch21-common.md) |
| 22 | Creole | 14 | 4 | 16+ | [ch22-creole.md](audit/ch22-creole.md) |
| 23 | Sprites | 7 | 0 | 1 | [ch23-sprites.md](audit/ch23-sprites.md) |
| 24 | Skinparam | 5 | 4 | 5 | [ch24-skinparam.md](audit/ch24-skinparam.md) |
| 25 | Preprocessing | 18 | 8 | 2 | [ch25-preproc.md](audit/ch25-preproc.md) |
| 26 | Unicode | 3 | 2 | 0 | [ch26-unicode.md](audit/ch26-unicode.md) |
| 27 | Standard Library | 7/16 bundled | — | 9/16 missing | [ch27-stdlib.md](audit/ch27-stdlib.md) |

## Historical Strength Notes

This section preserves the original audit's relative strengths for planning
context. It is not a live ranking, and some denominators or chapter rows may lag
recent merged work. Re-check the linked per-chapter audit before opening or
closing implementation work from these notes.

1. **ArchiMate** — 6✅/9 (67%).
2. **Preprocessing** — 18✅/28 (64%). The repo's strongest large area: full `!if/!elseif/!ifdef/!while/!foreach/!function/!procedure` with `!local/!global/!unquoted`, broad builtin set, `!include/_many/_once/url/import`.
3. **MindMap** — 9✅/14 (64%).
4. **Math** — 3✅/5 (60%).
5. **Use Case** — 15✅/18 (83%).
6. **JSON** — 7✅/14 (50%). serde_json + block parsing both solid.
7. **WBS** — 6✅/12 (50%).
8. **nwdiag** — 11✅/17 (65%). Networks/groups/addresses/peer links solid; global styling and full shape parity still lag.
9. **Sequence** — 28✅/48 (58%). Strong on participants/arrows/notes/groups/activations/mainframe/aligned-notes/short-arrows/lifeline-strategy; partial on theming and exotic arrows.
10. **Common Commands** — 9✅/21 (43%).
11. **Salt (Wireframe)** — 10✅/24 (42%). Wide widget coverage, weakest at Creole-in-cells.
12. **Deployment** — ~25✅/47 (53%). Node-shape keyword parity is broad; style blocks, roundCorner, sprites, and exotic arrows remain the largest gaps.
13. **Gantt** — 22✅/39 (56%). Solid core; many verbal-form date constructs missing.
14. **Object** — 3✅/8 (38%).
15. **YAML** — 3✅/8 (38%).
16. **Skinparam** — 5✅/14 (36%).
17. **Component** — 12✅/19 (63%).
18. **Creole** — 14✅/34+ (41%). Core inline formatting and Unicode escapes work; block-level formatting remains missing.
19. **State** — 7✅/25 (28%) in the baseline. Recent state-note and state-data-projection work means the headline percentage is stale; trust `ch09-state.md` for current evidence.
20. **Class** — 9✅/45 (20%). Generics, member-qualified refs, hide/remove, and a large stereotype-skin matrix all missing.
21. **Information Engineering** — 1✅/5 (20%). `entity` routed to sequence parser.
22. **Unicode** — 1✅/5 (20%) in the baseline. Numeric entities, `<U+...>`, and a small deterministic emoji subset now have code/test coverage; trust `ch26-unicode.md` for current evidence.
23. **Activity (new)** — 6✅/30 (20%). SDL terminators (6.21.2), kill/detach shapes (6.5, 6.20) added 2026-05-21.
24. **Timing** — 1✅/29 (3%). MVP-only.
25. **Activity (legacy)** — 0✅/12 (0%). Effectively unsupported — migrate to new syntax.
26. **Sprites** — 7✅/8 (88%). Renderer and CLI sprite support have landed; only the GUI import helper is intentionally out of scope.

## Cross-cutting findings — the patterns that explain everything

These themes recur across many chapters. Fixing any one of them lifts the support number in several diagram families at once.

### 1. Sprites have recently landed

The initial audit found no general sprite system, but current code supports sprite definitions, `<$name>` references, `listsprites`, stdlib sprite includes, and `-encodesprite`. Treat chapter 23 as mostly implemented and check `audit/ch23-sprites.md` before opening new sprite work; remaining follow-up should focus on family-specific layout polish and macro-library behavior rather than rebuilding the core sprite parser.

### 2. `<style>` blocks partially wired

PlantUML's modern theming language (CSS-like selectors inside `<style>...</style>`) now has narrow sequence and componentDiagram component-color slices that lower to existing skinparam plumbing. It remains mostly unwired across title/header/footer selectors, class, activity, state, timing, and deployment.

### 3. Stereotype-scoped skinparam `<<X>>` overrides

`skinparam class<<Trait>> { … }` and equivalents on usecase/object/component/state are dropped — only the unscoped form is applied.

### 4. Hide / remove model-pruning system outside component `$tag` controls

Component diagrams now support `hide`/`remove`/`restore $tag` and `@unlinked` pruning. Class and broader deployment-specific filtered forms still need follow-up, including full `hide <Foo>` parity where applicable.

### 5. Creole block-level

Only inline Creole (`**bold**`, `//italic//`, `__under__`, `--strike--`, `[[url]]`, `<color:>`, `<size:>`, `<font:>`) is broadly honored. **Block-level forms remain missing:** lists `*`/`#`, headings `=`/`==`, horizontal rules `----`/`====`, tables `|= |`, tree `|_`, `<sub>`/`<sup>`, `<plain>`, `<back:>`, `<font:>`, `<img:>`. Unicode escape decoding (`<U+XXXX>` / numeric entities) and a small deterministic emoji subset have since landed in `src/creole.rs`; full PlantUML emoji catalog/directive parity is still missing.

### 6. Direction modifiers

`left to right direction` works in some normalizers (usecase, class), not in others. `top to bottom direction` is largely unmapped. The family pipeline doesn't propagate it consistently.

### 7. `allowmixing` + embedded JSON inside other diagrams

Mentioned in usecase (2.18), object (4.8), class, and state. State diagrams now host `json` / `yaml` projection blocks; broader `allowmixing` across class/object/usecase remains missing.

### 8. Crow's-foot / IE arrowheads (`||--o{`, `|o--||`, etc.)

Chapter 20 and class chapter both need these for IDEF1X / IE diagrams. Not natively parsed; the `entity` keyword is mis-routed to the sequence parser at `src/parser/family.rs:231` — fixing that alone is the highest-leverage change for IE conformance.

### 9. `mainframe`

Sequence and normalized family diagrams support the common command; specialized/raw diagram renderers still need follow-up.

### 10. Exotic arrowheads

Sequence (`-//`, `->o`, `->x`, doubled-slash forms), class (`#--`, `x--`, `}--`, `+--`, `^--`), deployment (`--@`, `-->>`, `0)--(0`, `-(0)-`) all fall through to plain arrows.

### 11. State styling gaps remain, but note/data hard errors were fixed

`note left/right/top/bottom of Foo` and `note on link` in state diagrams now parse, normalize, and render with focused tests. Inline `state Foo #pink`, state-body style selectors, and some pseudo-state snapping/layout issues remain open.

### 12. Timing renderer is MVP-only

`TimingDeclKind` has only 4 variants (`src/ast.rs:292-298`) — no `analog` lifecycle, no cross-lane `X -> Y` message arrows, anchor points `@:name` and clock-multiplier `@clk*N` silently drop events because `time_to_x` requires `i64` and date/time tick values fail to parse.

## Top remediation priorities

Ranked by cross-cutting leverage from the audit baseline. These remain useful
themes, but recent merged work may have narrowed some rows; verify the relevant
chapter audit before treating an item as still open.

1. **Sprite follow-through** (ch23) — core support has landed; verify family-specific label layout and stdlib macro behavior before adding more broad sprite infrastructure.
2. **`<style>` block parser + selector engine** (ch21, ch24) — expand beyond the current narrow sequence and component color slices to unlock modern theming across all families.
3. **Class diagram generics + `extends`/`implements` keywords + member-qualified refs `Foo::field`** (ch03) — class is currently the weakest big-family chapter.
4. **Activity (new) `ActivityStep` enrichment** (ch06) — adding color/arrow-style/connector slots to the AST is shallow and unlocks ~14 features.
5. **IE `entity` routing fix + crow's-foot arrowheads** (ch20) — one-line classification change + arrow-table additions makes IE diagrams functional.
6. **Deployment follow-through** (ch08) — bare `usecase` routing, bracket-body divider rendering, exotic arrowheads, and per-shape style support remain after the node-shape keyword slice.
7. **State pseudo-state/composite layout and inline styling** (ch09) — notes/data projections landed, but history/parallel-region/pin snapping and inline color/style remain high-value state work.
8. **Timing `analog` lifecycle + cross-lane arrows + date/anchor tick parsing** (ch10) — timing is currently the weakest family overall.
9. **Stereotype-scoped skinparam dispatch** (cross-cutting #3) — extends theme table.
10. **JSON/YAML `#highlight` directive + style integration** (ch11, ch12) — small, high-visibility.

## Where to dig deeper

- Per-chapter audits live in [`audit/`](audit/). Each entry has feature description, syntax example, status tag, and file:line evidence.
- The full TOC skeleton (every subsection, no annotations) was the seed for the audits.

## Maintenance

When a feature ships, update the relevant per-chapter audit and bump the counts in the matrix above. When a new PlantUML version is published, re-run the extract + fan-out workflow (see `scripts/regen-spec-audit.sh` — to be authored if this becomes a recurring exercise).

Renderer changes should also run the committed-artifact freshness path. As of
PR #997, CI can classify artifact-impacting changes, run artifact regeneration,
and then re-run the normal PR gate on the regenerated head. Local agents should
still visually inspect PNG output before accepting generated artifact changes.
