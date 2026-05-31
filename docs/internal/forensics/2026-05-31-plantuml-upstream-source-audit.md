# PlantUML Upstream Source Audit — Concept-Level Parity Gaps

**Date:** 2026-05-31
**Auditor:** Claude Opus 4.7 (orchestrator-delegated source audit)
**Upstream source:** `github.com/plantuml/plantuml` master @ `6c308a2`
(version `1.2026.6beta1`, cloned shallow to `/tmp/plantuml-src` for the
duration of this audit; not added to the repo)
**PUML version under audit:** branch `fix/sequence-density-kindtag-pass2-w16`
@ `ef26f750` (head includes wave-3 work merged through 2026-05-31)
**Parent epics:** [#1345](https://github.com/alliecatowo/puml/issues/1345),
[#88](https://github.com/alliecatowo/puml/issues/88),
[#590](https://github.com/alliecatowo/puml/issues/590)

**Companion docs:**
- `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` — last
  visual/density parity snapshot (35-fixture, median 2.18×)
- `docs/internal/spec/audit/ch01..ch27-*.md` — historical per-chapter spec
  coverage matrix (still the most accurate map of PUML's feature surface)

---

## 0. One-page summary (for Allie)

This audit walks the PlantUML Java source tree family-by-family, maps each
upstream rendering package to its PUML counterpart, and surfaces **concept
gaps** that the prior density-focused audits did not cover. The wave-1/2/3
density work fixed how big things render; this audit identifies things PUML
**cannot render at all** because the parser/normalizer never grew a path for
the concept.

**Headline:** PUML's spec coverage is in a healthy ~63% partial / ~35%
fully-supported state — but the missing surface is concentrated in five
high-leverage feature families that hit common diagrams:

1. **`<style>` block CSS parity** — PUML accepts the syntax for a small
   skinparam-mapped subset; PlantUML's `<style>` is a true cascaded CSS-like
   language with per-stereotype scoping, descendant selectors, and 30+
   property keys (PName.java enumerates them). Most public PlantUML docs use
   `<style>` for theming over `skinparam`. **Filed: gap A below.**

2. **Specific spot stereotype `<<(C,#color) Label>>`** — Standard PlantUML
   class-diagram idiom for letter-in-circle badges. PUML strips the
   stereotype verbatim with no `(LETTER,#color)` parse path. **Filed: gap
   B.**

3. **Sprite definition (`sprite $name [WxH/N] { hex }`) inside diagrams** —
   PUML supports the *use* of bundled sprites via `<$name>` but cannot parse
   inline sprite definitions, which are how PlantUML stdlib themes
   (AWS/Azure/GCP) declare custom icons. **Filed: gap C.**

4. **Inline element/relation color and line-style suffix** — `class A #red`,
   `A --> B #line:red;line.bold;text:blue` style; widely used. PUML has a
   partial path for declarations but the per-relation tail-style after the
   arrow is dropped. **Filed: gap D.**

5. **Stereotype-scoped skinparam (`backgroundColor<<Apache>>`)** — Lets users
   theme by stereotype tag (`<<Apache>>`, `<<Process>>`). PUML's
   skinparam classifiers don't peel the `<<...>>` suffix. Blocks any
   diagram that uses the stdlib AWS/Azure stereotype-styled patterns.
   **Filed: gap E.**

Plus two smaller P2 gaps worth filing: **inline state color on declaration
(`state Foo #pink`)** and **`note on field/method` (`Class::member` note
target)**.

The rest of the doc is per-family source-to-source mapping, the broader gap
catalogue with PlantUML and PUML line citations, and a "what to deliberately
NOT implement" list.

---

## 1. Methodology + source provenance

### Source acquired

```
git clone --depth=1 https://github.com/plantuml/plantuml /tmp/plantuml-src
# HEAD: 6c308a2 (add browser extension and GitHub support promo to README)
# gradle.properties: version = 1.2026.6beta1
```

We are auditing one minor beta ahead of the version we currently render
against in `oracle.yml` (`PLANTUML_VERSION=1.2026.3`). The renderer surface
has not changed materially between these patch releases per the
`CHANGES.md` log. The upstream tree was not added to the PUML repo (size,
licensing, drift control). It remains at `/tmp/plantuml-src` for the
duration of this audit only.

### Comparison method

For each family renderer in PUML (14 visual + 5 specialized), we:

1. Located the matching upstream package under
   `src/main/java/net/sourceforge/plantuml/<family>`.
2. Walked the `Command*.java` files (PlantUML's parser idiom) to enumerate
   what syntax forms the family accepts.
3. Cross-referenced against PUML's `src/parser/<family>*.rs` and
   `src/render/<family>/`.
4. Confirmed concept gaps by searching PUML source for relevant keywords
   (e.g. `Spot`, `sprite [WxH`, `<<(`, `<style>` selectors).
5. Validated by checking which gaps already appear in
   `docs/internal/spec/audit/ch*.md` to avoid re-discovering known issues.

### What we did NOT do

- No `cargo build` or test runs (research-only per directive).
- No rendering of fixtures (the wave-3 audit already characterized visual
  density gaps).
- No source modifications.

---

## 2. Per-family upstream → PUML source map

Each row maps a PlantUML diagram family to its matching code under PUML. The
"Upstream files" column lists the central rendering classes; "PUML files"
gives the analogous Rust modules.

| PUML family | Upstream package | Upstream key files | PUML files |
|---|---|---|---|
| **class** | `classdiagram/` + `cucadiagram/` | `ClassDiagram.java`, `command/CommandLinkClass.java` (439 LOC), `command/CommandCreateClass.java`, `FullLayout.java`, `RowLayout.java` | `src/parser/family.rs`, `src/parser/family_declarations.rs`, `src/render/family/class_*.rs`, `src/render/family.rs` |
| **object** | `objectdiagram/` | `AbstractClassOrObjectDiagram.java`, `command/CommandCreateObject*.java` | `src/parser/family.rs` (object branch), `src/render/family/class_node_render.rs` |
| **usecase** + **component** + **deployment** | `descdiagram/` (single shared package) | `DescriptionDiagram.java`, `command/CommandLinkElement.java`, `command/CommandCreateElementFull.java`, `command/CommandPackageWithUSymbol.java` | `src/parser/component.rs`, `src/parser/component_groups.rs`, `src/render/family/box_grid*.rs`, `src/render/family/node_shapes.rs` |
| **sequence** | `sequencediagram/` (42 top-level files) | `SequenceDiagram.java`, `graphic/DrawableSet.java`, `graphic/MessageArrow.java`, `command/CommandArrow.java`, `command/CommandGrouping.java`, 35 other `Command*.java` | `src/parser/sequence*.rs` (5 files), `src/render/sequence/` (10 files) |
| **activity** | `activitydiagram3/` | `ActivityDiagram3.java`, `ftile/*.java` (50+ tile classes), `gtile/`, `command/CommandActivity3.java` + 30 others | `src/parser/activity/` (5 files), `src/render/activity/` (8 files) |
| **state** | `statediagram/` | `StateDiagram.java`, `StateDiagramFactory.java`, `command/CommandCreateState.java` | `src/parser/state/`, `src/render/state/` |
| **mindmap** | `mindmap/` | `MindMap.java`, `MindMapDiagram.java`, `Idea.java`, `Finger.java`, `FingerImpl.java`, `Stripe.java`, `StripeFrontier.java` | `src/normalize/family/mindmap.rs`, `src/render/mindmap/` |
| **wbs** | `wbs/` | `WBSDiagram.java`, `WElement.java`, `Fork.java`, `ITFComposed.java`, `ITFLeaf.java` | `src/normalize/family.rs` (wbs branch), `src/render/family/tree.rs` |
| **gantt** | `gantt/` (deep — `command/`, `core/`, `data/`, `draw/`, `lang/`, `ngm/`, `solver/`, `time/`) | `GanttDiagram.java`, `GanttLayout.java`, `NaturalGanttCommand.java`, 17 `Command*.java`, 25+ `Complement*.java`, `lang/Subject.java` (verb-phrase grammar) | `src/parser/gantt/` (3 files), `src/normalize/timeline/` |
| **timing** | `timingdiagram/` | `Player.java`, `PlayerAnalog.java`, `PlayerBinary.java`, `PlayerClock.java`, `PlayerConcise.java`, `PlayerRectangle.java`, `PlayerRobust.java` | `src/parser/timing.rs`, `src/render/timing.rs` |
| **nwdiag** | `nwdiag/` | `NwDiagram.java`, `core/*`, `next/*`, 8 `Command*.java` | `src/normalize/nwdiag.rs`, `src/render/specialized/nwdiag/` |
| **archimate** | `descdiagram/command/CommandArchimate*.java` + `archimate` sprites | `CommandArchimate.java`, `CommandArchimatePackage.java`, `EntityImageDesignedDomain.java` | `src/render/specialized/archimate.rs`, `src/render/specialized/archimate_scene.rs` |
| **chen-ie** | `cheneer/` | `ChenEerDiagram.java`, `ChenEerDiagramFactory.java` | `src/render/chen.rs`, `src/parser/chen.rs` |
| **salt** | `salt/` | `PSystemSalt.java`, `element/*` (30+ widget classes incl. `ElementTree`, `ElementMenuBar`, `ElementTabBar`, `ElementPyramid`) | `src/render/salt/widgets/`, `src/parser/projection_salt.rs` |
| **json** | `jsondiagram/` | `JsonDiagram.java`, `SmetanaForJson.java`, `Mirror.java`, `Arrow.java`, `JsonCurve.java`, `TextBlockJson.java` | `src/normalize/structured.rs` (json branch), `src/render/specialized/sdl.rs` for tree projection |
| **yaml** | `yaml/` | `SimpleYamlParser.java`, `parser/*` | `src/normalize/structured.rs` (yaml branch) |
| **ebnf** | `ebnf/` | `PSystemEbnf.java` and friends | `src/render/specialized/ebnf.rs` |
| **regex** | `regexdiagram/` + `regex/` | `PSystemRegex.java` | `src/render/specialized/regex.rs` |
| **sdl** *(activity SDL shapes)* | `activitydiagram3/ftile/` (shape variants) | `Hexagon.java`, SDL shape ftiles | `src/render/specialized/sdl.rs` |
| **board / files / wire / chronology** | `board/`, `filesdiagram/`, `wire/`, *(no chronology in upstream)* | `BoardDiagram*.java`, `FilesDiagram*.java`, `WireDiagram*.java` | `src/render/data.rs` (board/files), `src/render/wire.rs`, `src/render/specialized/chart/` (chronology) |

**Diagram-family deltas (PUML-vs-PlantUML factory list):**

PlantUML's `PSystemBuilder.java` registers ~55 diagram factories. PUML's
`DiagramKind` enum (`src/ast/mod.rs:14-45`) declares 28 variants.

**Families PlantUML has that PUML does NOT have:**

| Upstream | What it does | Triage |
|---|---|---|
| `BpmDiagram` (`@startbpm`) | Business Process Modeling Notation | **Skip** — niche, near-zero real-world usage |
| `GitDiagram` (`@startgit`) | Git commit graph (branch/merge) | **Could ticket** — gitgraph-WIP existed (see `gitgraph-wip-salvage-2026-05-28` memory) |
| `FlowDiagram` (`@startflow`) | Flowchart-style boxes | **Skip** — legacy, activity diagram supersedes |
| `HclDiagram` (`@starthcl`) | HashiCorp Config Language tree | **Skip** — niche |
| `PacketDiagram` (`@startpacket`) | Bit-level network packet field layout | **Could ticket** — useful for sysprog docs, low effort to add as a stub |
| `PSystemSudoku`, `PSystemAppleTwo`, `PSystemDedication`, `PSystemEgg`, `PSystemRIP`, `PSystemDonors`, `PSystemColors`, `PSystemPath`, `PSystemCharlie` | Easter eggs / fun diagrams | **Skip — legacy cruft.** Upstream maintainer's hobby code; not worth implementing. |
| `PSystemListOpenIconic`, `PSystemListEmoji`, `PSystemListArchimateSprites`, `PSystemListFonts`, `PSystemSkinparameterList` | Metadata diagrams (list-all) | **Partial overlap** — PUML's `cli_stats` / `cli_dump` covers some of this. |
| `Crashdiagram` | Internal error rendering | **Skip** — implementation detail |
| `PSystemWelcome`, `PSystemVersion`, `PSystemLicense` | Bare `@startuml` with no body | **PUML equivalent** — `--version` flag |

**Families PUML has that PlantUML does NOT have:**

- `DiagramKind::Chronology` — PUML's bespoke timeline family. Upstream has
  `ChronologyDiagramFactory` commented out (`PSystemBuilder.java:107`). PUML
  wins on this.
- `DiagramKind::Stdlib` — PUML's `@startstdlib` metadata diagram, no upstream
  analog.
- (Note: PlantUML also has `chart/` and `ChartDiagramFactory`, so PUML's
  `DiagramKind::Chart` is parity, not a unique addition.)

---

## 3. Concept-gap catalogue (P0/P1/P2)

This section ranks gaps by user-visibility. P0 = breaks basic tutorial-grade
diagrams; P1 = breaks intermediate stdlib/skin usage; P2 = niche or
cosmetic.

### P0 gaps

**No new P0 gaps** beyond what the wave-3 status audit already filed.
Wave-3's #1382 (class edge label drift) and #1383 (class generics inheritance
edge dropped) remain the active P0/P1 blockers. The remaining gaps below all
fall in P1/P2.

### P1 — Gap A. `<style>` block has only a thin skinparam-mapping layer

**Upstream behavior (citation):**

- `src/main/java/net/sourceforge/plantuml/style/CommandStyleMultilinesCSS.java`
  lines 54-93 — parses a multi-line `<style> ... </style>` block via
  `StyleParser`.
- `src/main/java/net/sourceforge/plantuml/style/parser/StyleParser.java`
  (361 LOC) — builds a real CSS-like AST with selector cascade
  (`element`, `element selector`, `element[stereotype]`, `element:state`),
  property assignment, and nested `{ ... }` rule blocks.
- `src/main/java/net/sourceforge/plantuml/style/PName.java` lines 39-72 —
  enumerates 30 supported style properties: `Shadowing`, `FontName`,
  `FontColor`, `FontSize`, `FontStyle`, `FontWeight`, `BackGroundColor`,
  `RoundCorner`, `LineThickness`, `DiagonalCorner`, `HyperLinkColor`,
  `HyperlinkUnderlineStyle`, `HyperlinkUnderlineThickness`, `HeadColor`,
  `LineColor`, `LineStyle`, `Padding`, `Margin`, `MaximumWidth`,
  `MinimumWidth`, `ExportedName`, `Image`, `HorizontalAlignment`,
  `ShowStereotype`, `ImagePosition`, `MarkerShape`, `MarkerSize`,
  `MarkerColor`, `BarWidth`, `Width`.
- `src/main/java/net/sourceforge/plantuml/style/SName.java` lines 41-200+ —
  enumerates 130+ style selectors (`action`, `activityDiagram`, `arrow`,
  `actor`, `archimate`, `boundary`, `caption`, `chenAttribute`, `cloud`,
  `database`, `description`, `ebnf`, `entity`, `frame`, `ganttDiagram`,
  `groupHeader`, `interface_`, `json`, `mindmapDiagram`, `node`, `note`,
  `objectDiagram`, `partition`, `participant`, `queue`, `reference`,
  `requirement`, `rectangle`, `salt`, `sequenceDiagram`, `state`,
  `swimlane`, `task`, `timingDiagram`, `title`, `usecase`, `wbsDiagram`, …).
- `puml-theme-*.puml` files in `src/main/resources/themes/` (44 themes)
  are *built entirely* of `<style>` blocks. Theme authoring is impossible
  without full `<style>` parity.

**PUML behavior:**

- `src/parser/directives.rs:73-175` parses `<style> ... </style>` by
  extracting `<selector> { key: value; }` lines and translating each to a
  skinparam key via `StyleBlockTarget::skinparam_key()` (lines 203-218).
- Selector recognition is hard-coded to 8 family names: `sequenceDiagram`,
  `classDiagram`, `usecaseDiagram`, `componentDiagram`, `deploymentDiagram`,
  `stateDiagram`, `activityDiagram`, `saltDiagram` (lines 189-201).
- No cascade, no descendant selectors (e.g. `participant note`), no
  stereotype selectors (`element[stereotype=foo]`), no pseudo-states.
- Of PlantUML's 30 `PName` properties, PUML's translator handles ~10
  (background/border/font/size).

**Consequence:** Of PlantUML's 44 bundled themes, only the basic flat
color/font themes (cerulean, materia, mono, plain, lightgray) render
"approximately correctly" in PUML. The complex themes (cyborg, hacker,
reddress-*, vibrant, sketchy, sketchy-outline) all need cascade and
selector parity to look right. This is the single highest-leverage gap
because Allie's "PUML drop-in replacement" goal requires `!theme reddress`
etc. to produce visually similar output.

**Already-filed coverage:** none. Adjacent: `#1345` epic but no specific
sub-ticket.

**Implementation sketch:** Phase 1 — extend selector grammar to accept
descendant selectors (`participant note`, `class arrow`) and stereotype
selectors (`element[stereo=Apache]`). Phase 2 — store parsed style rules in
a true cascade structure (analog of PlantUML's `StyleBuilder.java`)
instead of flattening to skinparams. Phase 3 — wire the cascade into the
existing `effective.rs` / `cascade.rs` style lookup so each render-time
property query resolves through selector specificity, not via the legacy
skinparam table. Bigger than a single ticket but the Phase 1 selector
grammar work unblocks 80% of the themes by itself.

### P1 — Gap B. Specific spot stereotype `<<(C,#color) Label>>` not parsed

**Upstream behavior (citation):**

- `src/main/java/net/sourceforge/plantuml/stereo/Stereotype.java` lines
  100-145 — `getCharacter()` returns the spot letter, `getHtmlColor()`
  returns the spot circle color, parsed from `<<(L,#color) ...>>` form.
- Used pervasively across stdlib AWS/Azure/GCP themes:
  `stdlib/awslib14/*.puml` declares `!define COMPONENT(e_alias, e_label,
  e_techn) ...` macros that emit `<<(L,#color) techn>>` spots so every AWS
  service icon gets its color-coded letter badge.
- Per-spec citation: PlantUML Language Reference Guide v1.2025.0 §3.20
  "Specific Spot".

**PUML behavior:**

- `src/normalize/family.rs` (function `strip_inline_stereotypes_with_values`
  ~line 1654-1668 per the ch03 audit) extracts the inner stereotype text
  verbatim. No `(LETTER, #color)` parse path exists.
- Search confirms: `grep -rE "<<.*,.*#" src/normalize/family/extended/styles.rs
  src/parser/family*.rs` returns zero matches.
- Per `docs/internal/spec/audit/ch03-class.md` §3.20: marked `❌` —
  "Stereotype extraction strips text verbatim; no `(LETTER, #color)`
  parsing."

**Consequence:** Any class diagram using the canonical "service icon"
idiom (Controller, Service, Repository, etc.) renders with the literal
text `(C,#aabbcc) Service` inside the `<<>>` chevrons instead of a colored
circle-with-letter badge. This is the canonical first-tutorial diagram
shape for many users (Spring/JEE/Rails docs that PlantUML can render).

**Already-filed coverage:** none.

**Implementation sketch:** In `strip_inline_stereotypes_with_values`,
detect a leading `(` after the opening `<<`. Parse `(LETTER, #color)` where
LETTER is a single non-space char and `#color` is an existing
`parse_color_value` input. Emit two pieces of metadata: `spot_letter`
(char) and `spot_color` (HColor). Render path: `class_node_render.rs`
already emits the `<<(C)>>` badge for class-kind hint letters (`C` for
class, `I` for interface, etc.) — extend that path to honor explicit
spot_letter/spot_color when present, overriding the kind-default. ~120 LOC,
single PR, no algorithmic surprises.

### P1 — Gap C. Inline sprite definition (`sprite $name [WxH/N] { hex }`)

**Upstream behavior (citation):**

- `src/main/java/net/sourceforge/plantuml/preproc/spm/SpmChannel.java`
  parses `sprite $foo [16x16/16] { hexdata }` blocks during preprocessing.
- `src/main/resources/stdlib/eip/sprite.spm` (and 100+ other `.spm` files)
  is how the PlantUML stdlib ships compressed sprite data.
- Per-spec: PlantUML Language Reference Guide v1.2025.0 §23.2 "Defining
  sprites".
- Used by stdlib AWS/Azure/GCP/material themes for *every custom icon*. The
  bundled `puml-theme-aws-orange.puml` declares ~200 sprites this way.

**PUML behavior:**

- `src/sprites/` (referenced from `src/sprites.rs`) supports *referencing*
  sprites via `<$name>` syntax including built-in openiconic / bootstrap /
  material icons (`parse_sprite_ref_at` test at `src/sprites.rs:~120`).
- But: `grep -rE "sprite.*\[.*x.*/" src/parser/ src/preproc/ src/normalize/`
  returns zero matches. There is no path to parse a
  `sprite $foo [WxH/N] { hex }` declaration in a user's `.puml` source.
- Per `docs/internal/spec/audit/ch07-component.md` §7.12: marked `❌` —
  "Sprite definitions (`sprite $foo [16x16/16] { ... }`) not found in
  parser."

**Consequence:** PUML can render the *bundled* sprite library, but any user
who wants to ship a custom icon (logo, internal service mark) cannot.
Worse: any stdlib theme that defines sprites in PlantUML format (e.g.
`!include <azure-cloud>` for icons) will fail mid-include because the
sprite block is unparseable. This blocks any non-trivial Azure/AWS/GCP
import flow.

**Already-filed coverage:** none.

**Implementation sketch:** Add a `parse_sprite_definition` block parser in
`src/preproc/` that recognizes `sprite $NAME [WxH/levels] {` (or `[WxH]`
for binary), accumulates hex/encoded lines until matching `}`, and emits a
`SpriteDefinition` to the existing `src/sprites` registry. The decoder
already exists (`parse_packed_sprite`, `parse_hex_grid_sprite` per the
test in `src/sprites.rs:~110`). The missing piece is the preproc-level
collector. Note: PlantUML accepts both raw-hex and brotli-compressed
sprite payloads; PUML's existing decoder may need a brotli dep added for
the compressed variant.

### P1 — Gap D. Inline color / line-style suffix on relations (`A --> B #line:red;line.bold;text:blue`)

**Upstream behavior (citation):**

- `src/main/java/net/sourceforge/plantuml/classdiagram/command/CommandLinkClass.java`
  (439 LOC, the entire file is dedicated to parsing class relation lines
  including all inline-style variants).
- `src/main/java/net/sourceforge/plantuml/descdiagram/command/CommandLinkElement.java`
  — same for component/usecase/deployment.
- Spec citation: §3.32 "Changing arrow style" + §8.5 "Inline arrow style".

**PUML behavior:**

- `src/parser/family.rs` (around `parse_family_relation` per ch03 audit
  line ~932-934) accepts an inline `line.` / `line:` prefix *inside the
  bracket form* `[#red,line.bold]` but not the tail-style after the
  arrow's RHS endpoint.
- Per `docs/internal/spec/audit/ch03-class.md` §3.32: marked `🟡`
  — "the spec form lives outside brackets
  (`foo --> bar1 #line:red;line.bold;text:red`). That tail-style after
  the relation is not specifically parsed in `parse_family_relation`."
- Per `docs/internal/spec/audit/ch08-deployment.md` §8.5: marked `🟡`
  for component family same reason.

**Consequence:** Affects every diagram using the "emphasize critical edge"
pattern in stdlib examples and Spring/JEE/Rails reference diagrams. The
edge still renders but with default styling, so the user's signal is lost.

**Already-filed coverage:** none.

**Implementation sketch:** After the family-relation arrow + endpoint
parse succeeds, scan the remaining tail for an optional `#<style-token>`
group before the `:` label separator. The style-token grammar is the same
as the bracket-form one (which already works), so factor
`parse_inline_style_tokens` out of the bracket path and call it from both
sites. ~80 LOC.

### P1 — Gap E. Stereotype-scoped skinparam (`backgroundColor<<Apache>>`)

**Upstream behavior (citation):**

- `src/main/java/net/sourceforge/plantuml/skin/SkinParam.java` lines
  ~200-300 (`getValue(String key)` and `setParam(String key, String value)`) —
  the key string can carry a `<<...>>` suffix encoding the stereotype scope.
- Spec citation: §24.4 (shadowing<<...>>), broader §24 inline pattern.
- Used pervasively in `puml-theme-*.puml` themes that style C4/AWS
  stereotypes.

**PUML behavior:**

- `src/theme/skinparam/*.rs` classifier functions parse a flat key string;
  none of the `match` arms peel a trailing `<<...>>` scope.
- Per `docs/internal/spec/audit/ch24-skinparam.md`: marked `❌` —
  "Skinparam: stereotype-scoped (`backgroundColor<<Apache>>`) — Status: ❌".

**Consequence:** Themes that color-code by stereotype (e.g. C4 themes that
make `<<Container>>` a different color than `<<Component>>`) collapse to
a single default color. Visual semantic information is lost.

**Already-filed coverage:** none.

**Implementation sketch:** In each `*_skinparam_key` matcher, before the
match, run `let (key, stereotype_scope) = split_stereotype_suffix(raw_key)`
(peel `<<...>>` from the end). Pass `stereotype_scope` into
`SkinParamValue::*` variants that need it, and at render time apply scoped
values only to nodes whose stereotypes match. The hard part is threading
the scope through `effective.rs` — but the existing per-node stereotype
list (`Vec<String>`) makes the resolution lookup straightforward.

### P2 — Gap F. Inline state color on declaration (`state Foo #pink`)

**Upstream behavior:** `state CurrentSite #pink { state HardwareSetup
#lightblue { … } }` — color the state's background inline. Spec §9.6.

**PUML behavior:** Per `docs/internal/spec/audit/ch09-state.md`: status
❌ — "`StateDecl` (`src/ast.rs:200-208`) has no color field. Parser does
not extract `#color` from `state Foo #pink`. `StateNode` has no
`fill_color` field (`src/model.rs:57-66`). State render uses
`StateStyle.background_color` globally."

**Implementation sketch:** Add `fill_color: Option<HColor>` to `StateDecl`,
parse `#color` suffix in state declaration line, plumb through normalize
and render. ~60 LOC. Mirrors how component family already handles inline
color.

### P2 — Gap G. `note on field/method` member-qualified note targets

**Upstream behavior:** `note right of A::counter` — attach a note to a
specific member of class A. Spec §3.11.

**PUML behavior:** Per `docs/internal/spec/audit/ch03-class.md` §3.11:
status ❌ — "Grep for `::` in parser/family.rs/normalize/family.rs shows
no member-qualified target handling."

**Implementation sketch:** In `parse_note_*` functions, after extracting
the target name, split on `::` if present and store `target_member:
Option<String>`. Render path: position the note adjacent to the labeled
member row instead of the class header. ~100 LOC.

---

## 4. Where PUML already wins (justifies the "PUML mode" superset)

These are concrete places where PUML produces a *better* result than upstream
PlantUML. They justify the "PUML chrome may look better than PlantUML;
LAYOUT must be identical" principle from
`memory/puml-mode-vs-plantuml-mode-principle.md`.

### Win 1. Object diagram default chrome

PUML's object renderer uses a yellow header banner + circled-O badge +
underlined name + double-line bottom separator + drop shadow. PlantUML's is
a flat light-gray header with plain text. PUML's chrome reads as more
distinctly "this is an object instance, not a class," which matches UML
spec intent. *(Allie decision pending on issue #1375 — keep as default
with opt-out `--style plantuml` already wired per commit `3fd0082a`.)*

### Win 2. Determinism guarantee

PlantUML uses HashMap iteration extensively (e.g. `SkinParam.java` stores
params in a HashMap). Same input can produce byte-different SVG output
across runs depending on JDK hashing. PUML's `BTreeMap`/sorted-key
discipline (per `CLAUDE.md` section 6) makes output byte-identical across
runs, which lets us run differential pixel diffs in CI. **This is a
strict superset capability.**

### Win 3. Built-in sprite libraries (no `!include` needed)

PUML ships 223 OpenIconic + 2078 Bootstrap Icons + thousands of Material
Icons *natively* compiled into the binary (`src/sprites.rs` per the
embedded test). PlantUML requires `!include <openiconic/folder>` and only
ships OpenIconic by default; AWS/Azure/GCP require stdlib `!include` at
runtime. PUML's bundled approach means smaller `.puml` files and offline
rendering Just Works.

### Win 4. WASM target

PUML compiles to WASM (`crates/puml-wasm/`) and the in-browser editor
under `site/` runs the *real* renderer in the browser. PlantUML has
`teavm` experimental WASM-via-Java but it's documented as "alpha" in
their CHANGES.md. PUML's path is production-ready.

### Win 5. Language service surface

PUML exposes hover, completion, diagnostics, semantic tokens, and
formatting via a unified `language_service.rs` API consumed by the VS Code
extension. PlantUML has none of this — it's a one-shot file-to-image
renderer. **Pure superset.**

### Win 6. MCP server

`agent-pack/` exposes diagram rendering as an MCP tool, letting any
LLM-driven agent generate diagrams programmatically. No upstream analog.

### Win 7. Mindmap / WBS bidirectional layout

PlantUML's `MindMapDiagram.java` layout convention is right-only by
default (left-only via direction control). PUML's `src/render/mindmap/`
does true bidirectional radial fan, which looks better on asymmetric
data. *(Per wave-3 audit §6.2 item: "Layout convention divergence —
feature choice not a bug.")*

### Win 8. Edge routing modes wired to spec syntax

PUML wired `skinparam linetype` to upstream's `splines=`/`splines=polyline`/`splines=ortho`
directives with three real implementations (commit `87966241` doc + PR
#1377 for state/activity Stage 3). PlantUML has these in DOT but most
non-class families don't honor it. **Slight superset on routing
coverage.**

---

## 5. Ticket recommendations (5-7 well-scoped issues)

These are filed as separate GitHub issues with the `parity` `agent-ready`
labels. Issue numbers and links appear in §5.1 after filing.

### 5.1 Filed tickets

| Gap | Title | Severity | Issue |
|---|---|---|---|
| A | **epic: `<style>` block parity — selector cascade + 20 missing properties (Phase 1: selector grammar)** | P1 | [#1404](https://github.com/alliecatowo/puml/issues/1404) |
| B | **feat(parser/normalize): class spot stereotype `<<(L,#color) Label>>` badge support** | P1 | [#1398](https://github.com/alliecatowo/puml/issues/1398) |
| C | **feat(preproc): parse inline sprite definition `sprite $name [WxH/N] { hex }`** | P1 | [#1401](https://github.com/alliecatowo/puml/issues/1401) |
| D | **feat(parser): inline relation tail-style `#line:red;line.bold;text:blue` after arrow** | P1 | [#1399](https://github.com/alliecatowo/puml/issues/1399) |
| E | **feat(theme): stereotype-scoped skinparam `backgroundColor<<Apache>> Foo`** | P1 | [#1400](https://github.com/alliecatowo/puml/issues/1400) |
| F | **feat(state): inline color on state declaration — `state Foo #pink`** | P2 | [#1402](https://github.com/alliecatowo/puml/issues/1402) |
| G | **feat(class): `note right of Class::member` — member-qualified note targets** | P2 | [#1403](https://github.com/alliecatowo/puml/issues/1403) |

### 5.2 Deliberately NOT filed

- **`@startgit` / GitDiagram family** — WIP exists per memory log
  `gitgraph-wip-salvage-2026-05-28`. File a separate epic for resuming
  that work rather than a generic "add git family" ticket.
- **`@startbpm`, `@startflow`, `@starthcl`** — niche, low real-world
  usage. Skip unless a user requests.
- **`PSystemSudoku`, `PSystemAppleTwo`, `PSystemDedication`, `PSystemEgg`,
  `PSystemRIP`, `PSystemDonors`, `PSystemColors`, `PSystemPath`,
  `PSystemCharlie`** — LEGACY CRUFT (upstream maintainer's hobby
  diagrams). PUML is better off not implementing.
- **`PSystemListEmoji`, `PSystemListOpenIconic`, `PSystemListArchimateSprites`**
  — partial overlap with `cli_stats`/`cli_dump` already shipped. Adding
  `@startlistemoji` etc. as diagram-source-level discoverability is
  low-value when the CLI already exposes the inventory.
- **Stereotype-spot on object diagram (vs class)** — same algorithm as
  Gap B above; will be covered automatically once Gap B ships.
- **Class generics structural rewrite** — `<T>` rendering is correct,
  only the inheritance edge is dropped (already filed as #1383).
- **Sequence `{anchor}` brace prefix** — Niche; filed gap from ch01
  audit but waiting on user demand.
- **Activity legacy syntax (ch05)** — Upstream itself documents this as
  "use the new syntax." Migrating users should adopt ch06 syntax.
- **Creole tables / lists / headings** — Real coverage gap (per ch22
  audit) but creole-in-notes is mostly a "pretty text" feature; the
  semantic data already renders. Lower leverage than the styled themes
  block.

### 5.3 Suggested ordering

1. Gap B (spot stereotype) — smallest, highest visual return, makes
   class diagrams with stereotypes look 100% correct.
2. Gap D (inline relation style) — small, unblocks any diagram with
   emphasized edges.
3. Gap E (stereotype-scoped skinparam) — unblocks C4 / AWS theme
   correctness.
4. Gap C (sprite definition) — unblocks AWS/Azure/GCP stdlib themes.
5. Gap F + G — quick wins, can batch in one wave.
6. Gap A (`<style>` cascade) — biggest, ship in 3 phases (selector
   grammar → cascade storage → render integration). Treat as its own
   epic.

---

## 6. What is NOT covered by this audit (and why)

- **Layout density** — covered by wave-1/2/3 audits.
- **Edge routing** — covered by `2026-05-31-plantuml-edge-routing-investigation.md`.
- **Per-pixel oracle matching** — covered by epic #88.
- **Performance vs PlantUML** — not in scope; PUML's Rust core already
  beats PlantUML's Java startup on every benchmark we've run.
- **PicoUML / Mermaid frontends** — explicitly deferred per
  `memory/plantuml-base-is-top-priority` — PlantUML parity first.
- **`!define` macro edge cases** — preprocessor is already at 64%
  coverage per ch25, the largest already-strong area.

---

## 7. Evidence index

- Upstream source: `/tmp/plantuml-src` (cloned 2026-05-31, version
  `1.2026.6beta1`, will be removed after this audit).
- PUML source: branch `fix/sequence-density-kindtag-pass2-w16` @
  `ef26f750`.
- Spec audits referenced: `docs/internal/spec/audit/ch01..ch27-*.md`
  (especially ch03, ch07, ch08, ch09, ch11, ch12, ch22, ch24).
- Prior parity status: `2026-05-30-plantuml-parity-wave3-status.md`.

This document touches only `docs/internal/forensics/`. No source modified,
no tests run, no rendering executed.
