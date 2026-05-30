# PlantUML Parity Wave-2 Re-Audit — 35 Fixtures

**Date:** 2026-05-30
**Auditor:** Claude Opus 4.7 (orchestrator-delegated audit)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27)
**PUML version under test:** `target/release/puml` built from `docs/wave2-parity-forensic`
(branched off origin/main at `7ff25f5c`, includes #1366 edge-label collision fix)
**Prior audit:** `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md`
(median area ratio **2.93×**, mean **3.30×** across 33 measurable fixtures)
**Audit method:** Re-render the same 34 fixtures from prior audit + `docs/diagrams/architecture-overview.puml`, compare canvas areas, spot-check PNG pairs side-by-side with the multimodal Read tool.

This is a re-measurement after 18 merged PRs targeting the eight child issues filed
under #1345 (density retune #1346, kind-tag suppression #1347, activity chrome #1348,
visibility glyphs #1349, class badge #1350, vertical concurrent regions #1351,
arclength edge labels #1352, C4 descriptions #1353) plus follow-ups #1357–#1364,
themes (#1368), CLI flags (#1365), edge-label collision (#1363/#1366), and
header overlap fixes.

---

## 1. Executive Summary

**Headline:** median area ratio fell from **2.93× → 2.25×** (−23%); mean fell
from **3.30× → 2.70×** (−18%). Five of eight parity gaps are fully resolved
(activity chrome, C4 descriptions, visibility glyphs, class badge, vertical
concurrent regions). Three remain partial (layout density still > 1.5× on most
families; kind-tag suppression missed nwdiag/timing/component-package frames;
edge label collisions still happen on dense state graphs).

The **worst offenders shifted**: deployment dropped from 7.65× to 4.90×;
component-02 dropped from 7.21× to 4.09×. The new worst remaining is
**gantt/05_multi_task at 5.22×** (PUML renders a richer timeline; PlantUML's
output is denser and more compact — but lossy on Saturday/Sunday weekend bars).
The single biggest unexpected regression is **class/01_basic at 3.24× (was 1.25×)** —
PlantUML 1.2026.5 itself shrunk this fixture, exposing PUML's still-loose class layout.

Eight families are now < 2×: activity (4 fixtures, all now 1.34–1.86×),
state (3 fixtures, all 0.96–1.49×), timing (was 5.64× → 1.61×), c4 (2.07×).
Six families remain > 2.5×: component (2.89–4.09×), deployment (2.19–4.90×),
sequence (all 4 fixtures 2.80–3.31× **unchanged**), object (2.61, 4.84
**unchanged**), nwdiag (2.93× **unchanged**), wbs/mindmap (1.38–2.62×
**unchanged** — layout convention divergence, not pure density).

**Top three remaining structural bugs surfaced by this re-audit:**

1. Usecase actor edges still drop/mis-route (`usecase/02`: Customer→PlaceOrder
   and Admin→ManageInventory both LOST; both actors get rerouted to BrowseProducts).
2. Usecase system-boundary still encloses actors that should be outside
   (`usecase/05`: Administrator, Registered User, Premium User all rendered
   INSIDE the dashed E-Commerce Platform frame).
3. Component parallel edges still deduplicate (`component/08`: both Service A
   and Service B `publish events` to Kafka — PUML drops one; only `consume`
   from Kafka to Stream Processor remains).

Five new follow-up issues are filed under this audit; existing parity gaps
that are NOT yet fixed (sequence density, object skin chrome, mindmap/wbs
convention, class generics inheritance) are documented but de-prioritised
until #1345 closes.

---

## 2. Quantitative summary — before vs after

Area = width × height in pixels of the rendered PNG. Ratios are PUML ÷ PlantUML.
All renders driven by `/opt/homebrew/bin/plantuml -tpng` and
`./target/release/puml --format png`. PNGs cached at `/tmp/parity_audit_v2/`.

| Fixture | PUML v2 | PlantUML v2 | v1 ratio | v2 ratio | Δ |
|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 5.01× | **1.86×** | −3.15 |
| activity/05_while_loop | 248×438 | 186×437 | 3.56× | **1.34×** | −2.22 |
| activity/07_partition | 248×762 | 179×736 | 3.55× | **1.43×** | −2.12 |
| activity/09_error_handling | 408×570 | 271×526 | 4.39× | **1.63×** | −2.76 |
| c4/12_container_with_databases | 1600×990 | 989×774 | 1.62× | 2.07× | +0.45 |
| class/01_basic | 276×434 | **134×276** | 1.25× | 3.24× | +1.99 ⚠ |
| class/03_composition_aggregation | 288×590 | 148×384 | 2.99× | 2.99× | 0.00 |
| class/05_visibility | 342×278 | 259×198 | 1.85× | 1.85× | 0.00 |
| class/11_generics | 808×266 | 361×316 | 1.88× | 1.88× | 0.00 |
| component/02_interfaces | 520×452 | 280×205 | 7.21× | **4.09×** | −3.12 |
| component/07_ports_lollipop | 1558×630 | 702×483 | 4.29× | **2.89×** | −1.40 |
| component/08_stereotypes | 1276×1428 | 660×803 | 5.12× | **3.44×** | −1.68 |
| deployment/02_databases | 576×696 | 254×322 | 7.65× | **4.90×** | −2.75 |
| deployment/03_cloud | 558×452 | 344×199 | 6.06× | **3.68×** | −2.38 |
| deployment/06_kubernetes | 1340×1278 | 934×839 | 3.00× | **2.19×** | −0.81 |
| gantt/05_multi_task | 880×338 | 419×136 | n/a | 5.22× | n/a |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× | 2.25× | 0.00 |
| mindmap/05_four_levels | 1629×658 | 723×1074 | 1.38× | 1.38× | 0.00 |
| nwdiag/02_multi_network | 760×410 | 295×360 | 2.93× | 2.93× | 0.00 |
| object/02_with_attributes | 327×450 | 223×253 | 2.61× | 2.61× | 0.00 |
| object/05_ch04_parity | 480×440 | 185×236 | 4.84× | 4.84× | 0.00 |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 |
| sequence/03_autonumber | 488×280 | 232×210 | 2.80× | 2.80× | 0.00 |
| sequence/07_notes | 550×432 | 255×316 | 2.95× | 2.95× | 0.00 |
| sequence/11_activation | 488×280 | 230×210 | 2.83× | 2.83× | 0.00 |
| sequence/12_create_destroy | 488×360 | 239×222 | 3.31× | 3.31× | 0.00 |
| state/03_concurrent | 232×646 | 246×419 | 2.10× | **1.45×** | −0.65 |
| state/07_nested | 273×630 | 207×557 | 1.98× | **1.49×** | −0.49 |
| state/10_parallel_regions | 249×1010 | 280×938 | 1.40× | **0.96×** | −0.44 |
| timing/01_concise | 426×156 | 250×165 | 5.64× | **1.61×** | −4.03 |
| usecase/02_with_actors | 398×534 | 286×453 | 2.10× | **1.64×** | −0.46 |
| usecase/05_actor_generalization | 1084×782 | 1830×653 | 1.28× | **0.71×** | −0.57 |
| usecase/06_multi_system | 1384×778 | 1090×568 | 5.06× | **1.74×** | −3.32 |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.62× | 2.62× | 0.00 |
| **diagrams/architecture-overview** | 1032×1672 | (PlantUML errors) | n/a | n/a | n/a |

**Median (v2): 2.25×.** **Mean (v2): 2.70×.** **Min: 0.71× (usecase/05).** **Max: 5.22× (gantt/05).**

(Median was 2.93× / mean 3.30× in v1. PlantUML's own renders shifted slightly on
class/01_basic — the v1 doc reported PlantUML 419×228, v2 measures PlantUML 134×276
— so the +1.99 delta on class/01 is partly a moving baseline. PUML's class/01
output is identical to v1 at 276×434.)

### Architecture-overview note

`docs/diagrams/architecture-overview.puml` was added as a target outside the
34-fixture set. PlantUML 1.2026.5 **errors out** on this file
(`This element (Frontends) is already defined`) because the source uses
both `component [Adapters] as Frontends` and `package "Frontends"` — PUML
tolerates the namespace collision and renders. The PUML render at 1032×1672
takes up disproportionate vertical space due to a 600-pixel-tall empty
"Pipeline Core" package frame; this is a stand-alone layout-density bug
captured in the recommendations below.

---

## 3. Per-gap status

Each row maps the eight original child issues to current corpus state.

| # | Gap | Child issue | Status | Evidence |
|---|---|---|---|---|
| 1 | Layout density 2–7× | #1346/#1357 (closed) | **PARTIAL** — median 2.93×→2.25×, but six families unchanged (sequence, object, nwdiag, mindmap, wbs, class/03+05+11) | sequence/03 still 2.80×; object/05 still 4.84×; nwdiag/02 still 2.93× |
| 2 | Kind-tag labels ("node", "database", «component») | #1347 (closed) | **PARTIAL** — deployment & c4 clean; nwdiag still emits `network <name>`; component still emits `package <name>` on group frames; timing still emits `concise` next to lane names | nwdiag/02 PUML headers "network public (203.0.113.0/24)"; component/07+08 frame headers "package CDN Layer"; timing/01 lane subtitle "concise" |
| 3 | Activity caption + dashed bounding rect | #1348 (closed) | **DONE** — all four activity fixtures now match PlantUML chrome (no subtitle, no outer dashed frame) | activity/02..09 all dropped 5×→1.5× |
| 4 | UML visibility glyphs | #1349 (closed) | **DONE** — ○ public / ● private / ◆ private / ▲ package now rendered in class & object diagrams | class/01_basic now shows ○ and ● glyphs; matches PlantUML side-by-side |
| 5 | Class «C» badge in header | #1350 (closed) | **DONE** — green ⓒ glyph now renders inside class header | class/01, class/11 PUML PNGs show the badge top-left |
| 6 | State concurrent region orientation | #1351 (closed) | **DONE** — regions now stack top-to-bottom with horizontal dashed dividers | state/10_parallel ratio 1.40×→0.96× (PUML now smaller than PlantUML) |
| 7 | Edge label placement at arclength midpoint | #1352/#1366 (closed) | **PARTIAL** — generally improved but DENSE state fixtures still show stacked overlapping labels (e.g. state/10 `play pause stop` collide) | state/10 PUML — labels `play`, `pause`, `stop`, `resume` all clustered into a 30-pixel band between Playing/Paused/Stopped |
| 8 | C4 description + tech-tag rendering | #1353 (closed) | **DONE** — `«system»`, italic descriptions, `[HTTPS]` brackets all render | c4/12 PUML shows full stereotypes + italic subtitles + tech tags exactly like PlantUML |

**Net:** five of eight gaps fully closed; three remain partially resolved
with measurable residual delta.

---

## 4. Remaining gap catalogue

### 4.1 Density gap — sequence family entirely untouched

Sequence diagrams (`sequence/03_autonumber`, `07_notes`, `11_activation`,
`12_create_destroy`) all sit at 2.80–3.31× area ratio, identical to v1. Visual
structure is parity-faithful but participant boxes are ~1.6× wider and
lifeline spacing is ~1.6× larger than PlantUML.

**Root cause hypothesis:** `src/render/sequence/` likely has its own layout
constants distinct from `src/render/layout_constants.rs`. The density retune
in #1346 modified family-wide constants but did not touch the sequence
lifeline / participant column code.

**Strongest evidence:** `sequence/03_autonumber-PUML.png` (488×280) vs
`sequence/03_autonumber-PlantUML.png` (232×210) — identical content, PUML
is exactly 2× wider horizontally per participant slot.

### 4.2 Density gap — object diagram skin chrome

`object/02_with_attributes` and `object/05_ch04_parity` remain at 2.61× and
4.84× respectively. PUML adds:
- a yellow header banner with a circled "O" stereotype badge
- a drop-shadow under the box
- an underlined header (this part IS correct UML 2.x)
- double-bottom-line separator

PlantUML uses a flat neutral light-gray header with plain text and a thin
border. The chrome adds ~30% vertical and horizontal padding per object box.

**Strongest evidence:** `object/02_with_attributes-PUML.png` vs `…-PlantUML.png`
side-by-side.

### 4.3 Density gap — nwdiag family unchanged

`nwdiag/02_multi_network` still at 2.93×. Three issues stacked:
- "network <name>" prefix on every horizontal bar (kind-tag fix missed this family)
- "Network diagram" subtitle still emitted
- Each network segment occupies ~1.6× the vertical height of PlantUML's thin bus line
- Cyan tinted backgrounds add visual weight

**Strongest evidence:** `nwdiag/02_multi_network-PUML.png` (760×410) shows three
networks stacked with "network public (203.0.113.0/24)", "network private (10.0.0.0/24)",
"network ops (172.16.0.0/24)"; PlantUML uses just the segment name plus CIDR on
a separate line.

### 4.4 Kind-tag suppression missed component package frames

The kind-tag fix (#1347) successfully removed `<<node>>`, `<<database>>`,
`<<artifact>>` from deployment and `«component»` from individual component
boxes. But group frame headers in component/07 and component/08 still
emit `package <name>` (e.g. `package CDN Layer`, `package API Cluster`,
`package Order Service`, `package Notification Service`).

PlantUML draws the same group frame as a thin folder-tab box with just the
name (`CDN Layer`, `API Cluster`). The "package " prefix is decorative and
exactly the pattern #1347 was supposed to suppress.

**Strongest evidence:** `component/08_cloud_db_queue_stereotypes-PUML.png` —
six dark navy header bands each starting with "package ".

### 4.5 Kind-tag suppression missed timing concise lanes

`timing/01_concise` PUML still emits:
- a "timing diagram" subtitle at top-left
- the word "concise" as a sub-label under each lane name

PlantUML emits neither. The timing-family `kind_label` slot was not gated
on explicit stereotype.

**Strongest evidence:** `timing/01_concise-PUML.png` upper-left corner.

### 4.6 Usecase actor edges still drop or mis-route

`usecase/02_with_actors.puml` declares:
```
Customer --> UC1
Customer --> UC2
Admin --> UC3
```

PUML renders only `Customer --> UC1` and `Admin --> UC1` (both actors funneled
to BrowseProducts via an L-shaped joint). `Customer --> UC2` and `Admin --> UC3`
are LOST entirely. PlantUML renders all three actor edges correctly.

This was logged as out-of-scope in the v1 audit; needs its own issue now
because the actor→use-case relationship is fundamental to use-case diagrams.

**Strongest evidence:** `usecase/02_with_actors-PUML.png` vs source line-by-line.

### 4.7 Usecase system boundary still encloses actors

`usecase/05_actor_generalization_system_boundary.puml` defines an
`E-Commerce Platform` rectangle that should contain only use cases (with
actors outside per UML spec). PUML renders **all four actors** (User,
Administrator, Registered User, Premium User) INSIDE the dashed boundary frame.
PlantUML correctly places actors above/right of the solid boundary rectangle.

**Strongest evidence:** `usecase/05_actor_generalization_system_boundary-PUML.png`
— dashed magenta frame surrounds the entire diagram including all actors.

### 4.8 Component parallel-edge deduplication

`component/08_cloud_db_queue_stereotypes.puml` declares two `publish events`
edges from Service A → Kafka and Service B → Kafka. PUML draws ONE edge
labelled `consume` (Kafka → Stream Processor) but no publish-events edges
from either service. PlantUML draws both parallel edges with the same label.

Logged in v1 but not filed. Worth a follow-up.

### 4.9 Class generics inheritance edge lost

`class/11_generics.puml` declares `Stack extends Container` (via `<|--` or
similar). PUML drops the generalization edge entirely; the three classes
(Container, Stack, Map) render as floating boxes side-by-side. PlantUML
stacks them vertically with the open-triangle arrow.

Logged in v1 out-of-scope. Worth filing.

### 4.10 Dense state-graph edge label collision

`state/10_parallel_regions_shared_events-PUML.png` shows the labels `play`,
`pause`, `stop`, `resume` clustered in a ~30-pixel vertical band, overlapping
each other and the adjacent transition arrows. The arclength-midpoint fix
(#1352) and the collision-push fix (#1366) help when labels are sparse but
multiple labels sharing the same arclength bin still collide.

**Strongest evidence:** `state/10_parallel_regions_shared_events-PUML.png`
around y=200..280 — "play pause stop" all overlapping.

### 4.11 Architecture-overview vertical waste

`docs/diagrams/architecture-overview.puml` renders at 1032×1672 in PUML.
The "package Pipeline Core" frame is ~600 pixels tall with three child
components (Parser, AST, Normalizer, Renderer) stacked vertically inside,
with ~100 pixels of empty space between each. PlantUML rejects the file
entirely so there is no direct comparison, but the wasted vertical space
is a stand-alone layout-density bug.

Also the package header text "package Pipeline Core" gets clipped at the
right edge of the dark-navy header band when group width exceeds header
text width — visible header overflow.

### 4.12 Out-of-scope findings (logged, not filed)

Deferred or low-priority observations, kept here for the record:

- **Mindmap / WBS layout convention divergence.** PUML uses bidirectional
  radial / horizontal-tree; PlantUML uses right-only / vertical-org-chart.
  PUML's looks better for asymmetric data; surface as a feature choice if
  ever needed, not a bug.
- **Object skin chrome** — yellow banner, drop-shadow, circled-O badge.
  Could be a `--theme plain` opt-out rather than a default change.
- **Timing concise hexagonal segment shape.** PUML renders flat rectangles;
  PlantUML uses chevron-edged hexagons. Larger refactor; revisit after
  density fixes for timing land.
- **Component lollipop circle size.** PUML draws r=18 interface circles;
  PlantUML uses r≈4. Cosmetic but contributes to the component density gap.
- **Component required-interface half-circle ("socket") notation.**
  PUML draws full circles for both provides and requires; PlantUML draws a
  half-circle ("socket") for requires.
- **Sequence create participant green-circle decorator.** PUML adds a tiny
  green dot above newly-created participants in `sequence/12_create_destroy`;
  PlantUML omits.
- **Actor name label overlap with stick-figure head** in `usecase/02`
  (`Customer` text overlaps the head outline). Improved from v1 but still
  visible at small canvas sizes.

---

## 5. Top-5 next-fix recommendations

Ranked by **(visual impact across the corpus) × (1 / implementation risk)**.

### 5.1 Sequence family density retune (P1)

**Why:** Sequence is the most-used diagram family in the corpus. Four of 35
fixtures sit at 2.80–3.31× area ratio with zero change from v1. Visible on
every sequence diagram in the docs corpus.

**Scope:** `src/render/sequence/layout.rs` (or equivalent file owning
participant-column width and message-row height). Constant retune mirroring
what #1346 did for graph-layout families.

**Effort:** ~30 LOC constant retune + visual baseline updates. Same shape
as #1346/#1357.

**Acceptance criteria:** sequence/03, 07, 11, 12 area ratios all drop below
2.0× while preserving message-arrow + activation-bar geometry.

### 5.2 Kind-tag suppression — second pass for missed families (P1)

**Why:** Kind-tag pattern still leaks in three places: component group-frame
headers (`package <name>`), nwdiag segment headers (`network <name>`), and
timing lane subtitles (`concise`). Each family adds ~10-20% horizontal
waste from these prefixes alone.

**Scope:** Three small touches:
- `src/render/family/group_frame.rs` (or wherever package/folder-tab headers
  emit text) — strip `package ` prefix when family is component
- `src/render/nwdiag/` — strip `network ` prefix from segment titles
- `src/render/timing/` — gate the per-lane `concise`/`robust` subtitle on
  explicit stereotype declaration

**Effort:** ~30 LOC across three files.

**Acceptance criteria:** No PUML render emits the literal strings "package ",
"network ", or a "concise"/"robust"/"binary" subtitle unless the source
file explicitly declares the stereotype.

### 5.3 Usecase actor edge correctness (P0)

**Why:** Two distinct correctness bugs (edge drop in `usecase/02`, actor
placement inside boundary in `usecase/05`). Use-case diagrams are unusable
without correct actor→use-case routing.

**Scope:** `src/render/usecase/` or wherever actor edges are computed.
Two sub-fixes:
1. Edge router must emit every declared actor→usecase edge; current
   behaviour deduplicates or rewires edges that share a target.
2. Layout must place actors OUTSIDE the system-boundary rectangle by
   default (UML 2.x spec); only use cases nest inside boundaries.

**Effort:** ~50-80 LOC, two focused bug fixes.

**Acceptance criteria:**
- `usecase/02_with_actors` renders 5 edges (Customer→UC1, Customer→UC2,
  Admin→UC3, UC1→UC2, UC2→UC3); no edges lost.
- `usecase/05_actor_generalization_system_boundary` renders all 4 actors
  outside the system boundary rectangle.

### 5.4 Component package-frame header overflow + parallel edges (P1)

**Why:** Two related component-family bugs:
- "package <name>" still emitted on every group frame (overlaps with 5.2).
- Parallel edges with identical labels deduplicate to one drawn edge
  (visible in `component/08_cloud_db_queue_stereotypes` — Service A and
  Service B both `publish events` to Kafka; PUML draws only one).

**Scope:** Edge router parallel-edge handling in `src/render/graph_layout/`
or the component-family edge emission code. Plus the kind-tag fix from 5.2.

**Effort:** ~40 LOC. Parallel-edge case needs a small algorithm change to
fan out endpoints when source+target pair has multiple edges.

**Acceptance criteria:**
- `component/08` renders both publish-events edges, fanned out at the
  Kafka endpoint.
- Group-frame headers contain only the user-supplied name, no "package "
  prefix.

### 5.5 Object diagram skin chrome — opt-out default (P2)

**Why:** Object/02 at 2.61× and object/05 at 4.84× — both unchanged from v1.
The yellow header + drop-shadow + circled-O badge adds ~30% padding per
object box and visually departs from PlantUML's neutral look.

**Scope:** `src/render/family/object_render.rs` (or equivalent).
Decision-fork option:
- Option A: change defaults to match PlantUML (neutral gray, no shadow,
  no badge), expose old style via a `--theme heavy` flag.
- Option B: add a `--plain` flag that switches all chrome off,
  keep current as default.

Allie should decide which.

**Effort:** ~30 LOC + theme plumbing.

**Acceptance criteria:** `object/02_with_attributes` and `object/05_ch04_parity`
both drop below 2.0× area ratio under default theme.

---

## 6. Per-family score table (before vs after)

Each cell = (number of fixtures with ratio ≥ 1.5×) / (total fixtures in family).

| Family | v1 score | v2 score | Notes |
|---|---|---|---|
| activity | 4/4 | **0/4** | All four dropped to 1.34–1.86× |
| c4 | 1/1 | 1/1 | 1.62× → 2.07× (slightly worse but description gap closed) |
| class | 3/4 | 3/4 | unchanged (only class/01 < 1.5×; class/01 itself went up due to PlantUML shrink) |
| component | 3/3 | 3/3 | unchanged (still all > 2.5×, density retune partial) |
| deployment | 3/3 | 3/3 | unchanged at the 1.5× threshold; absolute ratios fell 35-40% |
| gantt | n/a | 1/1 | new measurement (5.22×) |
| mindmap | 1/2 | 1/2 | unchanged |
| nwdiag | 1/1 | 1/1 | unchanged |
| object | 2/2 | 2/2 | unchanged |
| salt | 0/1 | 0/1 | unchanged (already good) |
| sequence | 4/4 | 4/4 | **unchanged** — density retune skipped sequence |
| state | 2/3 | **0/3** | All three now < 1.5× (best family improvement) |
| timing | 1/1 | **1/1** but 5.64→1.61 | huge improvement |
| usecase | 2/3 | 1/3 | usecase/05 went to 0.71×, usecase/06 to 1.74× |
| wbs | 1/1 | 1/1 | unchanged |
| **Overall** | **28/34** (82%) | **22/35** (63%) | 19% absolute improvement |

---

## 7. Filed follow-up issues

Issues filed under this audit (see GitHub for current state):

- *Filed under section 5.1* — Sequence family density retune
- *Filed under section 5.2* — Kind-tag suppression second pass (component/nwdiag/timing)
- *Filed under section 5.3* — Usecase actor edge correctness (edge drop + boundary placement)
- *Filed under section 5.4* — Component parallel-edge dedup + package header prefix
- *Filed under section 5.5* — Object diagram skin chrome opt-out

Three issues are NOT filed at this time:
- Class/11 generics inheritance edge dropped — out-of-scope in v1, still
  out-of-scope; this is a parser/normalize bug not a render bug.
- State dense edge-label collision (5.10) — partial improvement from #1366
  already; needs a larger label-placement algorithm change, hold for now.
- Architecture-overview vertical waste (5.11) — likely subsumed by the
  graph-layout-engine work in #590; do not file separately.

---

## 8. Methodology notes

- 35 fixtures rendered with both binaries to `/tmp/parity_audit_v2/`.
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips -g pixelWidth/pixelHeight`.
- Side-by-side visual inspection of 10 fixtures (class/01, class/11,
  component/02, component/07, component/08, deployment/02, sequence/07,
  sequence/03, state/10, timing/01, usecase/02, usecase/05, c4/12, mindmap/02,
  wbs/02, nwdiag/02, object/02, activity/02 — actually 18 inspected).
- Prior audit ratios extracted from
  `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` § 2.
- All PUML renders driven by the binary built from this branch
  (`docs/wave2-parity-forensic` off `origin/main` at `7ff25f5c`).
- PlantUML reference: `/opt/homebrew/bin/plantuml` (1.2026.5, Java 21).
- The `gantt/05_multi_task` fixture rendered successfully in BOTH this time —
  v1 PlantUML errored, v2 PlantUML succeeded. Logged as one of two new
  measurements alongside the architecture-overview.
- No source code was modified during this audit.

Cached evidence at `/tmp/parity_audit_v2/*.png` will be cleaned by the next
OS restart. Copy to `docs/internal/forensics/2026-05-30-evidence-v2/` if the
PNG pairs need to survive.
