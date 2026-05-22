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

## Headline numbers

Across the 27 chapters the audits scored roughly:
- **~178 ✅ fully supported features**
- **~145 🟡 partial features**
- **~190 ❌ missing features**

≈ 63% of documented PlantUML features have at least partial code support; only ≈ 35% are confidently 1:1.

## Per-chapter support matrix

| # | Chapter | ✅ | 🟡 | ❌ | Audit |
|---|---------|----|----|----|-------|
| 1 | Sequence Diagram | 22 | 16 | 10 | [ch01-sequence.md](audit/ch01-sequence.md) |
| 2 | Use Case Diagram | 10 | 4 | 3 | [ch02-usecase.md](audit/ch02-usecase.md) |
| 3 | Class Diagram | 16 | 13 | 16 | [ch03-class.md](audit/ch03-class.md) |
| 4 | Object Diagram | 3 | 3 | 2 | [ch04-object.md](audit/ch04-object.md) |
| 5 | Activity (legacy) | 0 | 2 | 10 | [ch05-activity-legacy.md](audit/ch05-activity-legacy.md) |
| 6 | Activity (new) | 6 | 12 | 12 | [ch06-activity-new.md](audit/ch06-activity-new.md) |
| 7 | Component | 10 | 6 | 3 | [ch07-component.md](audit/ch07-component.md) |
| 8 | Deployment | 14 kw + 5 sec | 10 sec | 15 kw + 2 sec | [ch08-deployment.md](audit/ch08-deployment.md) |
| 9 | State | 7 | 7 | 11 | [ch09-state.md](audit/ch09-state.md) |
| 10 | Timing | 1 | 9 | 19 | [ch10-timing.md](audit/ch10-timing.md) |
| 11 | JSON | 7 | 4 | 3 | [ch11-json.md](audit/ch11-json.md) |
| 12 | YAML | 3 | 1 | 4 | [ch12-yaml.md](audit/ch12-yaml.md) |
| 13 | nwdiag | 8 | 4 | 5 | [ch13-nwdiag.md](audit/ch13-nwdiag.md) |
| 14 | Salt (Wireframe) | 10 | 11 | 3 | [ch14-salt.md](audit/ch14-salt.md) |
| 15 | ArchiMate | 6 | 2 | 1 | [ch15-archimate.md](audit/ch15-archimate.md) |
| 16 | Gantt | 15 | 11 | 13 | [ch16-gantt.md](audit/ch16-gantt.md) |
| 17 | MindMap | 10 | 3 | 0 | [ch17-mindmap.md](audit/ch17-mindmap.md) |
| 18 | WBS | 6 | 4 | 2 | [ch18-wbs.md](audit/ch18-wbs.md) |
| 19 | Math | 3 | 1 | 1 | [ch19-math.md](audit/ch19-math.md) |
| 20 | Information Engineering | 1 | 2 | 2 | [ch20-ie.md](audit/ch20-ie.md) |
| 21 | Common Commands | 12 | 6 | 4 | [ch21-common.md](audit/ch21-common.md) |
| 22 | Creole | 12 | 3 | 16 | [ch22-creole.md](audit/ch22-creole.md) |
| 23 | Sprites | 0 | 1 | 7 | [ch23-sprites.md](audit/ch23-sprites.md) |
| 24 | Skinparam | 5 | 4 | 5 | [ch24-skinparam.md](audit/ch24-skinparam.md) |
| 25 | Preprocessing | 18 | 8 | 2 | [ch25-preproc.md](audit/ch25-preproc.md) |
| 26 | Unicode | 1 | 1 | 3 | [ch26-unicode.md](audit/ch26-unicode.md) |
| 27 | Standard Library | 7/16 bundled | — | 9/16 missing | [ch27-stdlib.md](audit/ch27-stdlib.md) |

## Strength ranking (best → weakest by ✅% of audited features)

1. **ArchiMate** — 6✅/9 (67%).
2. **Preprocessing** — 18✅/28 (64%). The repo's strongest large area: full `!if/!elseif/!ifdef/!while/!foreach/!function/!procedure` with `!local/!global/!unquoted`, broad builtin set, `!include/_many/_once/url/import`.
3. **MindMap** — 9✅/14 (64%).
4. **Math** — 3✅/5 (60%).
5. **Use Case** — 10✅/17 (59%).
6. **JSON** — 7✅/14 (50%). serde_json + block parsing both solid.
7. **WBS** — 6✅/12 (50%).
8. **nwdiag** — 8✅/17 (47%). Networks/groups/addresses solid; peer links absent.
9. **Sequence** — 22✅/48 (46%). Strong on participants/arrows/notes/groups/activations; partial on theming and exotic arrows.
10. **Common Commands** — 9✅/21 (43%).
11. **Salt (Wireframe)** — 10✅/24 (42%). Wide widget coverage, weakest at Creole-in-cells.
12. **Deployment** — ~19✅/47 (40%). Most node-shape keywords missing.
13. **Gantt** — 15✅/39 (38%). Solid core; many verbal-form date constructs missing.
14. **Object** — 3✅/8 (38%).
15. **YAML** — 3✅/8 (38%).
16. **Skinparam** — 5✅/14 (36%).
17. **Component** — 6✅/18 (33%).
18. **Creole** — 12✅/31 (39%). Inline only; all block-level formatting missing.
19. **State** — 7✅/25 (28%). `note … of` is a hard error.
20. **Class** — 9✅/45 (20%). Generics, member-qualified refs, hide/remove, and a large stereotype-skin matrix all missing.
21. **Information Engineering** — 1✅/5 (20%). `entity` routed to sequence parser.
22. **Unicode** — 1✅/5 (20%). All three escape forms missing.
23. **Activity (new)** — 6✅/30 (20%). SDL terminators (6.21.2), kill/detach shapes (6.5, 6.20) added 2026-05-21.
24. **Timing** — 1✅/29 (3%). MVP-only.
25. **Activity (legacy)** — 0✅/12 (0%). Effectively unsupported — migrate to new syntax.
26. **Sprites** — 0✅/8 (0%). Cascades into stdlib icons (see cross-cutting #1 below).

## Cross-cutting findings — the patterns that explain everything

These themes recur across many chapters. Fixing any one of them lifts the support number in several diagram families at once.

### 1. Sprites are completely absent

No `sprite $name [w*h] {...}` definition, no `<$name>` reference, no `listsprites`, no `-encodesprite`. Only Salt-internal placeholder stubs. **Knock-on effect:** every stdlib icon library (AWS, Azure, GCP, Material, Office, tupadr3) parses cleanly but renders as plain stereotyped boxes — so chapter 27 looks much worse than it ought to.

### 2. `<style>` blocks unwired

PlantUML's modern theming language (CSS-like selectors inside `<style>...</style>`) is ignored across every chapter that mentions it (component, activity, state, timing, common, deployment).

### 3. Stereotype-scoped skinparam `<<X>>` overrides

`skinparam class<<Trait>> { … }` and equivalents on usecase/object/component/state are dropped — only the unscoped form is applied.

### 4. Hide / remove / `$tag` / `@unlinked` model-pruning system

Class, component, deployment all support `hide @unlinked`, `remove $tag`, `hide <Foo>`. None of this is wired. Major gap for filtered diagrams.

### 5. Creole block-level

Only inline Creole (`**bold**`, `//italic//`, `__under__`, `--strike--`, `[[url]]`, `<color:>`, `<size:>`, `<font:>`) is honored. **All block-level forms are missing:** lists `*`/`#`, headings `=`/`==`, horizontal rules `----`/`====`, tables `|= |`, tree `|_`, emoji `<:name:>`, `<U+XXXX>`/`&#nnn;` escapes, `<sub>`/`<sup>`, `<plain>`, `<back:>`, `<font:>`, `<img:>`. This affects every diagram that renders user text.

### 6. Direction modifiers

`left to right direction` works in some normalizers (usecase, class), not in others. `top to bottom direction` is largely unmapped. The family pipeline doesn't propagate it consistently.

### 7. `allowmixing` + embedded JSON inside other diagrams

Mentioned in usecase (2.18), object (4.8), class. No code path — diagrams can't host a `json` block today.

### 8. Crow's-foot / IE arrowheads (`||--o{`, `|o--||`, etc.)

Chapter 20 and class chapter both need these for IDEF1X / IE diagrams. Not natively parsed; the `entity` keyword is mis-routed to the sequence parser at `src/parser/family.rs:231` — fixing that alone is the highest-leverage change for IE conformance.

### 9. `mainframe`

Sequence + common. Missing entirely.

### 10. Exotic arrowheads

Sequence (`-//`, `->o`, `->x`, doubled-slash forms), class (`#--`, `x--`, `}--`, `+--`, `^--`), deployment (`--@`, `-->>`, `0)--(0`, `-(0)-`) all fall through to plain arrows.

### 11. Note-on-state and inline state color cause hard errors

`note left/right/top/bottom of Foo` in a state diagram triggers a hard `E_STATE_MIXED` error (normalizer wildcard arm at `src/normalize/state.rs:141`). Inline `state Foo #pink` is dropped. These should degrade gracefully, not 500.

### 12. Timing renderer is MVP-only

`TimingDeclKind` has only 4 variants (`src/ast.rs:292-298`) — no `analog` lifecycle, no cross-lane `X -> Y` message arrows, anchor points `@:name` and clock-multiplier `@clk*N` silently drop events because `time_to_x` requires `i64` and date/time tick values fail to parse.

## Top remediation priorities

Ranked by leverage (features unlocked per unit of work):

1. **Sprite system** (ch23) — biggest single unlock. Enables 5+ stdlib libraries.
2. **`<style>` block parser + selector engine** (ch21, ch24) — unlocks modern theming across all families.
3. **Class diagram generics + `extends`/`implements` keywords + member-qualified refs `Foo::field`** (ch03) — class is currently the weakest big-family chapter.
4. **Activity (new) `ActivityStep` enrichment** (ch06) — adding color/arrow-style/connector slots to the AST is shallow and unlocks ~14 features.
5. **IE `entity` routing fix + crow's-foot arrowheads** (ch20) — one-line classification change + arrow-table additions makes IE diagrams functional.
6. **Deployment node-shape variants** (ch08) — 15 missing `FamilyNodeKind` variants (actor/agent/queue/stack/etc.). Mostly mechanical.
7. **State diagram `note … of` degraded handling** (ch09) — turn the hard error into a graceful skip; unblocks many existing diagrams.
8. **Timing `analog` lifecycle + cross-lane arrows + date/anchor tick parsing** (ch10) — timing is currently the weakest family overall.
9. **Stereotype-scoped skinparam dispatch** (cross-cutting #3) — extends theme table.
10. **JSON/YAML `#highlight` directive + style integration** (ch11, ch12) — small, high-visibility.

## Where to dig deeper

- Per-chapter audits live in [`audit/`](audit/). Each entry has feature description, syntax example, status tag, and file:line evidence.
- The full TOC skeleton (every subsection, no annotations) was the seed for the audits.

## Maintenance

When a feature ships, update the relevant per-chapter audit and bump the counts in the matrix above. When a new PlantUML version is published, re-run the extract + fan-out workflow (see `scripts/regen-spec-audit.sh` — to be authored if this becomes a recurring exercise).
