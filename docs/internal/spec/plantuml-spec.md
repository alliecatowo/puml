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
- **~201 ✅ fully supported features**
- **~147 🟡 partial features**
- **~166 ❌ missing features**

≈ 68% of documented PlantUML features have at least partial code support; ≈ 39% are confidently 1:1.

## Per-chapter support matrix

| # | Chapter | ✅ | 🟡 | ❌ | Audit |
|---|---------|----|----|----|-------|
| 1 | Sequence Diagram | 22 | 16 | 10 | [ch01-sequence.md](audit/ch01-sequence.md) |
| 2 | Use Case Diagram | 10 | 4 | 3 | [ch02-usecase.md](audit/ch02-usecase.md) |
| 3 | Class Diagram | 9 | 12 | 24 | [ch03-class.md](audit/ch03-class.md) |
| 4 | Object Diagram | 3 | 3 | 2 | [ch04-object.md](audit/ch04-object.md) |
| 5 | Activity (legacy) | 0 | 2 | 10 | [ch05-activity-legacy.md](audit/ch05-activity-legacy.md) |
| 6 | Activity (new) | 3 | 13 | 14 | [ch06-activity-new.md](audit/ch06-activity-new.md) |
| 7 | Component | 6 | 6 | 6 | [ch07-component.md](audit/ch07-component.md) |
| 8 | Deployment | 14 kw + 5 sec | 10 sec | 15 kw + 2 sec | [ch08-deployment.md](audit/ch08-deployment.md) |
| 9 | State | 7 | 7 | 11 | [ch09-state.md](audit/ch09-state.md) |
| 10 | Timing | 13 | 9 | 7 | [ch10-timing.md](audit/ch10-timing.md) |
| 11 | JSON | 10 | 5 | 0 | [ch11-json.md](audit/ch11-json.md) |
| 12 | YAML | 6 | 2 | 0 | [ch12-yaml.md](audit/ch12-yaml.md) |
| 13 | nwdiag | 8 | 4 | 5 | [ch13-nwdiag.md](audit/ch13-nwdiag.md) |
| 14 | Salt (Wireframe) | 10 | 11 | 3 | [ch14-salt.md](audit/ch14-salt.md) |
| 15 | ArchiMate | 6 | 2 | 1 | [ch15-archimate.md](audit/ch15-archimate.md) |
| 16 | Gantt | 15 | 11 | 13 | [ch16-gantt.md](audit/ch16-gantt.md) |
| 17 | MindMap | 9 | 4 | 1 | [ch17-mindmap.md](audit/ch17-mindmap.md) |
| 18 | WBS | 6 | 4 | 2 | [ch18-wbs.md](audit/ch18-wbs.md) |
| 19 | Math | 3 | 1 | 1 | [ch19-math.md](audit/ch19-math.md) |
| 20 | Information Engineering | 4 | 1 | 0 | [ch20-ie.md](audit/ch20-ie.md) |
| 21 | Common Commands | 9 | 5 | 7 | [ch21-common.md](audit/ch21-common.md) |
| 22 | Creole | 12 | 3 | 16 | [ch22-creole.md](audit/ch22-creole.md) |
| 23 | Sprites | 0 | 1 | 7 | [ch23-sprites.md](audit/ch23-sprites.md) |
| 24 | Skinparam | 5 | 4 | 5 | [ch24-skinparam.md](audit/ch24-skinparam.md) |
| 25 | Preprocessing | 18 | 8 | 2 | [ch25-preproc.md](audit/ch25-preproc.md) |
| 26 | Unicode | 3 | 2 | 0 | [ch26-unicode.md](audit/ch26-unicode.md) |
| 27 | Standard Library | 7/16 bundled | — | 9/16 missing | [ch27-stdlib.md](audit/ch27-stdlib.md) |

## Strength ranking (best → weakest by ✅% of audited features)

1. **Information Engineering** — 4✅/5 (80%). Entity blocks, mandatory markers, separators, and crow's-foot endpoints now render; `linetype ortho` is accepted as a no-op rather than a true routing switch.
2. **YAML** — 6✅/8 (75%). Parser-backed maps/sequences plus highlights, highlight styles, Creole scalars, and family projections; global node/arrow styling remains partial.
3. **JSON** — 10✅/15 (67%). serde_json parsing plus highlight/style/scalar rendering and projection boxes; global node/arrow styling remains partial.
4. **ArchiMate** — 6✅/9 (67%).
5. **Preprocessing** — 18✅/28 (64%). The repo's strongest large area: full `!if/!elseif/!ifdef/!while/!foreach/!function/!procedure` with `!local/!global/!unquoted`, broad builtin set, `!include/_many/_once/url/import`.
6. **MindMap** — 9✅/14 (64%).
7. **Math** — 3✅/5 (60%).
8. **Unicode** — 3✅/5 (60%). Numeric and `<U+...>` escapes decode; charset and full emoji catalogue remain partial.
9. **Use Case** — 10✅/17 (59%).
10. **WBS** — 6✅/12 (50%).
11. **nwdiag** — 8✅/17 (47%). Networks/groups/addresses solid; peer links absent.
12. **Sequence** — 22✅/48 (46%). Strong on participants/arrows/notes/groups/activations; partial on theming and exotic arrows.
13. **Timing** — 13✅/29 (45%). Advanced numeric timing, anchors, cross-lane messages, hidden/color states, analog ranges, scale, and compact mode now work; date axes, timing `<style>`, notes, and analog customization remain gaps.
14. **Common Commands** — 9✅/21 (43%).
15. **Salt (Wireframe)** — 10✅/24 (42%). Wide widget coverage, weakest at Creole-in-cells.
16. **Deployment** — ~19✅/47 (40%). Most node-shape keywords missing.
17. **Creole** — 12✅/31 (39%). Inline only; all block-level formatting missing.
18. **Gantt** — 15✅/39 (38%). Solid core; many verbal-form date constructs missing.
19. **Object** — 3✅/8 (38%).
20. **Skinparam** — 5✅/14 (36%).
21. **Component** — 6✅/18 (33%).
22. **State** — 7✅/25 (28%). `note … of` is a hard error.
23. **Class** — 9✅/45 (20%). Generics, member-qualified refs, hide/remove, and a large stereotype-skin matrix all missing.
24. **Activity (new)** — 3✅/30 (10%).
25. **Activity (legacy)** — 0✅/12 (0%). Effectively unsupported — migrate to new syntax.
26. **Sprites** — 0✅/8 (0%). Cascades into stdlib icons (see cross-cutting #1 below).

## Cross-cutting findings — the patterns that explain everything

These themes recur across many chapters. Fixing any one of them lifts the support number in several diagram families at once.

### 1. Sprites are completely absent

No `sprite $name [w*h] {...}` definition, no `<$name>` reference, no `listsprites`, no `-encodesprite`. Only Salt-internal placeholder stubs. **Knock-on effect:** every stdlib icon library (AWS, Azure, GCP, Material, Office, tupadr3) parses cleanly but renders as plain stereotyped boxes — so chapter 27 looks much worse than it ought to.

### 2. `<style>` blocks unwired

PlantUML's modern theming language (CSS-like selectors inside `<style>...</style>`) is still ignored across most chapters that mention it (component, activity, state, timing, common, deployment). JSON/YAML are a narrow exception for highlight/default-highlight styling only; node and connector selectors still need a shared style engine.

### 3. Stereotype-scoped skinparam `<<X>>` overrides

`skinparam class<<Trait>> { … }` and equivalents on usecase/object/component/state are dropped — only the unscoped form is applied.

### 4. Hide / remove / `$tag` / `@unlinked` model-pruning system

Class, component, deployment all support `hide @unlinked`, `remove $tag`, `hide <Foo>`. None of this is wired. Major gap for filtered diagrams.

### 5. Creole block-level

Only inline Creole (`**bold**`, `//italic//`, `__under__`, `--strike--`, `[[url]]`, `<color:>`, `<size:>`, `<font:>`) is broadly honored, and Unicode escape decoding now covers `&#nnn;`, `<U+XXXX>`, and a small emoji subset. **Block-level forms are still missing:** lists `*`/`#`, headings `=`/`==`, horizontal rules `----`/`====`, tables `|= |`, tree `|_`, `<sub>`/`<sup>`, `<plain>`, `<back:>`, richer `<font:>` blocks, `<img:>`, and the full emoji catalogue. This affects every diagram that renders user text.

### 6. Direction modifiers

`left to right direction` works in some normalizers (usecase, class), not in others. `top to bottom direction` is largely unmapped. The family pipeline doesn't propagate it consistently.

### 7. `allowmixing` + embedded JSON/YAML inside other diagrams

JSON/YAML projection nodes now parse and render in family diagrams, including nested YAML sequences. The remaining gap is PlantUML's broader `allowmixing` semantics across usecase/component/deployment/state diagrams and relation integration with those projection boxes.

### 8. IE arrowheads are now native; `linetype` remains shallow

Chapter 20's `entity` blocks and crow's-foot endpoint markers are now parsed/rendered natively. Remaining IE-adjacent work is narrower: `skinparam linetype ortho` is accepted as a no-op, and broader class-diagram cardinality/endpoint polish still depends on the shared relation renderer.

### 9. `mainframe`

Sequence + common. Missing entirely.

### 10. Exotic arrowheads

Sequence (`-//`, `->o`, `->x`, doubled-slash forms), class (`#--`, `x--`, `}--`, `+--`, `^--`), deployment (`--@`, `-->>`, `0)--(0`, `-(0)-`) all fall through to plain arrows.

### 11. Note-on-state and inline state color cause hard errors

`note left/right/top/bottom of Foo` in a state diagram triggers a hard `E_STATE_MIXED` error (normalizer wildcard arm at `src/normalize/state.rs:141`). Inline `state Foo #pink` is dropped. These should degrade gracefully, not 500.

### 12. Timing is no longer MVP-only, but date/style/note parity is still thin

Numeric timing now covers anchors, relative offsets, cross-lane messages, hidden/color states, analog ranges, `scale`, `hide time-axis`, and compact mode. The main remaining blockers are date/wall-clock axes, `use date format`, timing `<style>` selectors / signal stereotypes, participant notes, `has` ordering, and analog tick/height customization.

## Top remediation priorities

Ranked by leverage (features unlocked per unit of work):

1. **Sprite system** (ch23) — biggest single unlock. Enables 5+ stdlib libraries.
2. **`<style>` block parser + selector engine** (ch21, ch24) — unlocks modern theming across all families.
3. **Class diagram generics + `extends`/`implements` keywords + member-qualified refs `Foo::field`** (ch03) — class is currently the weakest big-family chapter.
4. **Activity (new) `ActivityStep` enrichment** (ch06) — adding color/arrow-style/connector slots to the AST is shallow and unlocks ~14 features.
5. **Timing date/style/note finish** (ch10) — add date/wall-clock axis parsing, timing `<style>` selectors, participant notes, and analog customization on top of the now-functional numeric renderer.
6. **Deployment node-shape variants** (ch08) — 15 missing `FamilyNodeKind` variants (actor/agent/queue/stack/etc.). Mostly mechanical.
7. **State diagram `note … of` degraded handling** (ch09) — turn the hard error into a graceful skip; unblocks many existing diagrams.
8. **Timing robust polish** (ch10) — `has` value ordering, constraint-arrow glyphs, `@clk*N` period semantics, and per-event comments remain after the parity wave.
9. **Stereotype-scoped skinparam dispatch** (cross-cutting #3) — extends theme table.
10. **JSON/YAML global node/arrow style integration** (ch11, ch12) — highlights and highlight styles work; node/connector selectors still use hard-coded defaults.

## Where to dig deeper

- Per-chapter audits live in [`audit/`](audit/). Each entry has feature description, syntax example, status tag, and file:line evidence.
- The full TOC skeleton (every subsection, no annotations) was the seed for the audits.

## Maintenance

When a feature ships, update the relevant per-chapter audit and bump the counts in the matrix above. When a new PlantUML version is published, re-run the extract + fan-out workflow (see `scripts/regen-spec-audit.sh` — to be authored if this becomes a recurring exercise).
