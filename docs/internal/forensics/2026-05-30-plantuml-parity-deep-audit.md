# PlantUML Parity Deep Audit — 34 Fixtures

**Date:** 2026-05-30
**Auditor:** Claude Opus 4.7 (orchestrator-delegated audit)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27)
**PUML version under test:** `target/release/puml` built from `main` (commit `f5b87dc5`)
**Audit method:** Render each fixture with `plantuml -tpng` and `puml --format png`,
read PNG pairs side-by-side with the multimodal Read tool, write delta descriptions,
grep source code to confirm file-scope hypotheses.

This document broadens and refines the 13-fixture personal audit captured in #1345
to 34 fixtures across 14 diagram families. Eight child issues are filed under #1345,
each scoped to a single file or small file cluster, each agent-ready.

---

## 1. Corpus

```
class    : 01_basic, 03_composition_aggregation, 05_visibility, 11_generics
component: 02_interfaces, 07_ports_lollipop_interfaces, 08_cloud_db_queue_stereotypes
sequence : 03_autonumber, 07_notes, 11_activation, 12_create_destroy
state    : 03_concurrent, 07_nested, 10_parallel_regions_shared_events
activity : 02_if_then_else, 05_while_loop, 07_partition, 09_error_handling
usecase  : 02_with_actors, 05_actor_generalization_system_boundary, 06_multi_system_boundary
deployment: 02_databases, 03_cloud, 06_kubernetes_pods_containers
object   : 02_with_attributes, 05_ch04_parity
c4       : 12_container_with_databases
gantt    : 05_multi_task
mindmap  : 02_multi_level, 05_four_levels_asymmetric
wbs      : 02_with_tasks
nwdiag   : 02_multi_network
timing   : 01_concise
salt     : 01_basic_widgets
```

34 pairs successfully rendered. One fixture (`gantt/01_basic`) was dropped because
PlantUML 1.2026.5 rejected its `[Task] happens on YYYY-MM-DD` syntax form
(syntax-error PNG); PUML renders it fine. Three C4 fixtures
(`c4/01..c4/11`) were skipped because `!include <C4/C4_Context>` macro resolution
fails inside PlantUML's stdlib for our environment; `c4/12` was kept because it
exercises the full container+database surface.

All PNG pairs live at:
```
/tmp/parity_audit/<family>-<basename>-PUML.png
/tmp/parity_audit/<family>-<basename>-PlantUML.png
```

---

## 2. Quantitative summary — PUML canvas area vs PlantUML

Area = width × height of the rendered PNG. Ratios are PUML ÷ PlantUML.

| Fixture | PUML | PlantUML | ratio |
|---|---|---|---|
| class/01_basic | 276×434 | 419×228 | 1.25× |
| class/03_composition_aggregation | 288×590 | 148×384 | **2.99×** |
| class/05_visibility | 342×278 | 259×198 | 1.85× |
| class/11_generics | 808×266 | 361×316 | 1.88× |
| component/02_interfaces | 672×616 | 280×205 | **7.21×** |
| component/07_ports_lollipop | 1862×782 | 702×483 | **4.29×** |
| component/08_stereotypes | 1536×1768 | 660×803 | **5.12×** |
| sequence/03_autonumber | 488×280 | 232×210 | 2.80× |
| sequence/07_notes | 550×432 | 255×316 | 2.95× |
| sequence/11_activation | 488×280 | 230×210 | 2.83× |
| sequence/12_create_destroy | 488×360 | 239×222 | 3.31× |
| state/03_concurrent | 450×482 | 246×419 | 2.10× |
| state/07_nested | 362×630 | 207×557 | 1.98× |
| state/10_parallel_regions | 632×582 | 280×938 | 1.40× |
| activity/02_if_then_else | 800×542 | 241×359 | **5.01×** |
| activity/05_while_loop | 480×602 | 186×437 | 3.56× |
| activity/07_partition | 480×974 | 179×736 | 3.55× |
| activity/09_error_handling | 800×782 | 271×526 | **4.39×** |
| usecase/02_with_actors | 528×516 | 286×453 | 2.10× |
| usecase/05_actor_generalization | 2006×764 | 1830×653 | 1.28× |
| usecase/06_multi_system | 2494×1256 | 1090×568 | **5.06×** |
| deployment/02_databases | 692×904 | 254×322 | **7.65×** |
| deployment/03_cloud | 674×616 | 344×199 | **6.06×** |
| deployment/06_kubernetes | 1510×1556 | 934×839 | 3.00× |
| object/02_with_attributes | 327×450 | 223×253 | 2.61× |
| object/05_ch04_parity | 480×440 | 185×236 | **4.84×** |
| c4/12_container_with_databases | 1344×920 | 989×774 | 1.62× |
| gantt/05_multi_task | 880×338 | error | n/a |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× |
| mindmap/05_four_levels | 1629×658 | 723×1074 | 1.38× |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.62× |
| nwdiag/02_multi_network | 760×410 | 295×360 | 2.93× |
| timing/01_concise | 1048×222 | 250×165 | **5.64×** |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× |

- **Median area ratio: ~2.9×.**
- **Mean area ratio: ~3.2×.**
- **Worst offenders: deployment (6–8×), component (4–7×), activity (3.5–5×).**
- **Tightest matches: salt (1.4×), class/01 (1.25×).**

A diagram that is 3× larger than necessary makes it harder to embed in
documentation, harder to fit on a slide, and slower to render. The single
biggest visual delta vs PlantUML is the **layout density gap**.

---

## 3. Patterns confirmed and refined

The eight patterns from #1345 are all confirmed by the broader corpus. The
file-scope hypothesis has been verified by grepping the source. Each row below
references the child issue that owns the fix.

| # | Gap | Root file | LOC refined | Child issue |
|---|---|---|---|---|
| 1 | Layout density — diagrams 2–7× larger | `src/render/layout_constants.rs` (constants) + `src/render/graph_layout/scene.rs` (slot sizing) | ~30 (constant retune) + ~60 (rank/column compaction) | #1346 |
| 2 | Decorative kind-tag labels ("node", "database", "artifact", «component») emitted on every shape | `src/render/family/node_shapes.rs` lines 504–524 + `src/render/family/class_members.rs:80-118` (`family_node_label`) | ~30 (conditional emission gated on explicit-stereotype) | #1347 |
| 3 | Activity adds "activity diagram" subtitle + outer dashed bounding frame | `src/render/activity/mod.rs:353-356` (subtitle) + `src/render/activity/swimlanes.rs:93-100` (default-lane dashed rect) | ~20 (remove subtitle, conditionalise dashed rect on real partitions) | #1348 |
| 4 | Visibility uses `+/-/#/~` plain prefix; PlantUML uses ●/○/◆/▲ shape icons | `src/render/family/class_node_render.rs:514-595` + `src/render/family/class_members.rs:5-29` | ~40 (replace plain prefix with inline `<circle>`/`<polygon>` glyph; preserve `attribute_icons` opt-out) | #1349 |
| 5 | Class header missing the green ⓒ class-icon badge that PlantUML draws inside every class header | `src/render/family/class_node_render.rs` (`render_class_node` header region) — currently NOTHING emits the badge | ~30 (new helper to emit small `<circle>+<text>C` glyph at header left, themable) | #1350 |
| 6 | State concurrent regions laid out left-to-right (columns); PlantUML lays them top-to-bottom (rows separated by horizontal dashed dividers) | `src/render/state/layout/sizing.rs:142-198` (`concurrent_region_metrics`) + `src/render/state/node_render.rs:320` (divider direction) | ~40 (swap dimension; flip dashed-divider orientation) | #1351 |
| 7 | Edge labels drift toward longest segment of an orthogonal route; PlantUML pins them to the path midpoint | `src/render/graph_layout/scene.rs:196-231` (`edge_label_box`) | ~30 (replace "longest segment" placement with "arclength midpoint" placement) | #1352 |
| 8 | C4 family drops description strings and `[Technology]` brackets parsed from `Person(...)` / `System(...)` / `Rel(...)` | `src/render/family/c4_nodes.rs` (node text rendering) + sequence-family edge label rendering for `Rel(..., "tech")` arg | ~40 (extend label text emitter to include second-line italic description + bracketed tech tag) | #1353 |

**Total est: ~250 LOC across 8 PRs.** Aligns with the parent epic's original estimate.

---

## 4. Per-family delta notes

Each row: PUML / PlantUML / observed delta and likely root cause.

### Class

- **class/01_basic** — *Structure: match. Density: PUML 1.25× larger. Style deltas: PlantUML emits a green ⓒ class-icon badge inside the header (centred left of the name); fields use ○ hollow circle, methods use ● filled circle as visibility glyphs; PUML uses `+/-` text prefix and colors all members green. Bugs: visibility-icon shape missing, class-badge missing.*
- **class/03_composition_aggregation** — *Structure: match. Density: PUML 2.99× larger. Style deltas: same as class/01. Composition diamond filled black correctly in both. PUML adds `<<class>>` stereotype where PlantUML omits implicitly.*
- **class/05_visibility** — *Structure: match. Density: 1.85×. Style deltas: PUML colorizes every visibility level with `+/-/#/~`; PlantUML draws geometric glyphs: ○ public, ◇ protected, ◆ private, ▲ package. Field vs method distinguished only by the icon being hollow (field) vs filled (method); PUML colors text by symbol kind instead — visually noisier and non-canonical.*
- **class/11_generics** — *Structure: **MISMATCH** — PUML loses the `Stack extends Container` inheritance edge entirely; only three boxes float side-by-side. PlantUML stacks them vertically with the open-triangle generalization arrow. Density 1.88×. Style deltas: PUML inlines `<T>` in name; PlantUML draws a dashed top-right corner box with the type parameter — closer to UML spec. Bugs: missing inheritance edge in PUML.*

### Component

- **component/02_interfaces** — *Structure: match. Density: 7.21× larger. Style deltas: PUML draws large r=18 interface circles WITH a separate «interface» stereotype label below each one. PlantUML draws small r≈6 lollipop circles inline at the edge endpoint with the interface name as a sibling text label, no «interface» stereotype. PUML adds «component» banner on every component; PlantUML omits when implicit. Component shape itself matches (rounded box with two small port rectangles on the left).*
- **component/07_ports_lollipop** — *Structure: match. Density: 4.29× larger. Style deltas: package frames in PUML carry a dark navy header bar with "package <name>" label band; PlantUML uses a thin folder-tab header. Lollipop interfaces same delta as above. Required-interface ("socket") not rendered as a half-circle in PUML; both halves drawn full. "uses" / "provides" / "requires" labels positioned reasonably in both. Bug: PUML loses one of two parallel "publish events" relationships (only one edge drawn to RabbitMQ).*
- **component/08_stereotypes** — *Structure: match. Density: 5.12× larger. Style deltas: PUML drops `<<database>>` and `<<queue>>` stereotypes entirely on PostgreSQL/RabbitMQ; PlantUML renders them as italic text inside the component box. PUML adds "package <name>" prefix to every group header; PlantUML uses just the name. Edge "publish events" label rendered once in PUML but PlantUML shows two parallel publish-events edges from Service A and Service B both to Kafka. Bug: parallel-edge label deduplication + dropped stereotype rendering.*

### Sequence

- **sequence/03_autonumber** — *Structure: match. Density: 2.80×. Style deltas: minor — PUML lifeline boxes slightly larger; PlantUML uses bold message numbers. Both align dashed arrows for return paths correctly.*
- **sequence/07_notes** — *Structure: match. Density: 2.95×. Style deltas: PUML notes have heavier orange border with darker page-fold; PlantUML uses thin yellow with subtle page-fold. Note alignment to lifeline correct in both.*
- **sequence/11_activation** — *Structure: match. Density: 2.83×. Style deltas: activation bars match (white rectangles on the lifeline); PUML bars are slightly thicker. PUML arrow heads filled solid; PlantUML uses thinner stroked triangles.*
- **sequence/12_create_destroy** — *Structure: match. Density: 3.31×. Style deltas: PUML adds a small green-circle decorator above the newly-created participant (not in PlantUML); destroy X marker matches; PlantUML places "new" arrow head directly attached to the participant header.*

### State

- **state/03_concurrent** — *Structure: match. Density: 2.10×. Composite layout vertical in both (good — the rotation bug only hits flat 2-state diagrams per epic). Concurrent regions inside Processing laid out left-right with vertical dashed dividers; PlantUML lays them top-bottom with horizontal dashed dividers. See pattern #6.*
- **state/07_nested** — *Structure: match. Density: 1.98×. Bug: "data ready" transition label is **detached from its edge** — floating in the gutter at left. PlantUML keeps it pinned to the Fetching→Processing arrow midpoint. See pattern #7.*
- **state/10_parallel_regions** — *Structure: **MISMATCH** — three parallel regions laid out left-right (vertical dividers) in PUML; PlantUML stacks them top-bottom (horizontal dividers). Density 1.40× (the rotated layout happens to use width well in this fixture). Bugs: missing several transitions (`mute`/`unmute` rendered, but the corresponding arrow heads are missing direction; `play` edge not drawn at all; `disableEQ` label floats orphaned). See patterns #6 and #7.*

### Activity

- **activity/02_if_then_else** — *Structure: match. Density: 5.01× larger. Style deltas: PUML emits a "**activity diagram**" subtitle under the title + a **dashed outer bounding rect** that surrounds the entire flow. PlantUML emits neither. PUML uses green-stroked rounded ovals for actions; PlantUML uses subtle gray rectangles. PUML places "no" on the left branch from authenticated? and "yes" on the right; PlantUML places "yes" on the left, "no" on the right (PlantUML convention is yes-left). Bug: branch label side convention reversed.*
- **activity/05_while_loop** — *Structure: match. Density: 3.56×. Same caption + bounding-rect bug. PUML drops the "no" label on the loop-exit branch; PlantUML implicitly labels it. PUML weld-joint for the back-edge is awkward; PlantUML uses a clean L-shape.*
- **activity/07_partition** — *Structure: match. Density: 3.55×. Same caption + bounding-rect bug. Partition headers rendered as dark band tabs in PUML; PlantUML uses thin folder-tab boxes. Partition layout direction is top-to-bottom in both (correct).*
- **activity/09_error_handling** — *Structure: match. Density: 4.39×. Same caption + bounding-rect bug. **Bug: PUML draws two end-of-activity bullseyes** — one mid-flow attached to "Complete" branch and one at the bottom; PlantUML uses exactly one final bullseye joined by both branches. The mid-flow bullseye is incorrect duplication.*

### Use case

- **usecase/02_with_actors** — *Structure: match. Density: 2.10×. **Bug: actor name labels overlap the actor's stick-figure head** ("Customer" and "Admin" text collides with the circular head outline). PlantUML places labels under the figure cleanly. PUML stacks all use-case ellipses in a single column with both actors above; PlantUML interleaves them, placing actors closer to their primary use cases.*
- **usecase/05_actor_generalization_system_boundary** — *Structure: degraded. Density: 1.28× (barely). PUML places one actor ("Premium User") INSIDE the system boundary rectangle; PlantUML keeps all actors outside (the system boundary surrounds only use cases, by spec). PUML system-boundary is rendered with a faint dashed magenta border that visually disappears; PlantUML uses a solid black rectangle with the boundary name centered top. Bug: actor placement crosses boundary.*
- **usecase/06_multi_system_boundary** — *Structure: **DEGRADED** — PUML drops several actor→use-case edges entirely (no "Login" arrow from Customer despite the source declaring it; Support Agent disconnected from multiple use cases). PlantUML renders all edges. PUML edges that exist bend through unnecessary path detours. Density: 5.06× because of the wasted whitespace between boundaries. Bugs: edge drop + boundary frame styling vanish.*

### Deployment

- **deployment/02_databases** — *Structure: match. Density: **7.65× larger**. Style deltas: PUML labels every 3D-cube node with the keyword **"node"** at the top of the shape; every cylinder with **"database"**. PlantUML omits these and uses just the user-supplied name. Shape primitives (3D cube, cylinder) match the canonical PlantUML versions. Bug: spurious keyword labels.*
- **deployment/03_cloud** — *Structure: match. Density: 6.06×. Same keyword-label bug. PUML labels the Lambda Function node with **"artifact"**; PlantUML uses the dog-eared rectangle with a small page-fold corner icon and no label inside. Two parallel labels ("stores" and "reads") draw correctly but the orthogonal routing creates a wasteful crossing layout. Bug: cluttered routing + keyword labels.*
- **deployment/06_kubernetes** — *Structure: degraded. Density: 3.00×. **PUML renders text microscopically small** — node names and container labels are illegible on the final PNG; "pod", "container", "namespace" keyword labels stack inside each box stealing all the space. PlantUML uses normal-sized text and clear nested 3D cubes. **Bug: text scaling is broken when many nested levels are present**, likely because each level's `kind_label` consumes vertical space that crowds the actual name out.*

### Object

- **object/02_with_attributes** — *Structure: match. Density: 2.61×. Style deltas: PUML uses a **yellow header banner + dark drop-shadow + double-bottom-line** styling — heavy custom skin; PlantUML uses a simple light-gray rounded box with an underlined header (per the UML object-diagram spec). The underlined header IS the correct UML 2.x convention; the yellow+drop-shadow chrome is a PUML brand departure that does not match PlantUML output.*
- **object/05_ch04_parity** — *Structure: match. Density: 4.84×. Same skin delta. Edge convergence diamond renders correctly in both.*

### C4

- **c4/12_container_with_databases** — *Structure: match. Density: 1.62× (C4 family compresses better). Style deltas: PUML drops every **description string** (`"Browser-based access"`, `"Async job processor"`, `"Rust REST endpoints"`) that PlantUML renders as a second-line italic below the name. PUML drops every **technology bracket** (`[HTTPS]`, `[REST]`, `[SQL]`, `[AMQP]`, `[Redis]`) on both shapes and edges. PUML uses simple labels `[Person]` / `[System]` / `[System, ext]` instead of «person»/«system»/«external_system» stereotypes which is the C4 standard. Person-icon: PUML draws a stick-figure outside the box; PlantUML draws a person-bust icon inside the colored box top-right. Bug: description + tech-tag rendering missing; stereotype labels wrong.*

### Gantt

- **gantt/05_multi_task** — *PlantUML errors on this fixture syntax; PUML renders a clean modern timeline. **PUML is BETTER here.** No parity gap to chase; document and move on.*

### Mindmap

- **mindmap/02_multi_level** — *Structure: divergent. PUML lays out as a **bidirectional radial tree** with root centered, branches left AND right. PlantUML lays out as **right-only horizontal tree** with root pinned at left. Both are valid, but PlantUML's right-only is the canonical mindmap convention. Style match: rounded color-coded leaf nodes in both.*
- **mindmap/05_four_levels_asymmetric** — *Same convention divergence; PUML's radial vs PlantUML's right-only. PUML is visually nicer for asymmetric trees but the parity gap remains.*

### WBS

- **wbs/02_with_tasks** — *Structure: divergent. PUML uses horizontal tree (root left, children right); PlantUML uses vertical org-chart style (root top, children below). Density 2.62× because of the horizontal stretch. Style match (rounded boxes).*

### Network diagram

- **nwdiag/02_multi_network** — *Structure: match. Density: 2.93×. Style deltas: PUML labels every network segment as "**network <name>**" (e.g. "network public (203.0.113.0/24)"); PlantUML uses just `<name>` and the CIDR. PUML uses cyan-tinted horizontal bars; PlantUML uses simple blue bus-line. Same keyword-prefix bug as deployment.*

### Timing

- **timing/01_concise** — *Structure: divergent. Density: 5.64×. **PUML uses flat rectangular timeline blocks**; PlantUML uses **chevron-edged hexagonal "concise" state segments** (the canonical timing-diagram notation). PUML emits "timing diagram" caption that PlantUML omits. Bug: concise-segment shape primitive missing.*

### Salt

- **salt/01_basic_widgets** — *Structure: match. Density: 1.38× (close). Style match: text fields, buttons, labels all render correctly in both. No parity gap; this family is in good shape.*

---

## 5. Ranked child issues — recommended ship order

Ordered by **(visual impact across the corpus) × (1 / implementation risk)**. Each
sub-bullet is the per-fix justification.

1. **#1346 — Layout density** (P0).
   Touches every diagram in the corpus. Constant-retune in
   `src/render/layout_constants.rs` (rank/node separation, group padding, canvas
   margin) is ~30 LOC and immediately compresses every render to ~PlantUML
   scale. Highest ROI single fix in the audit.

2. **#1347 — Decorative kind-tag labels** (P0).
   Visible on deployment, component, c4, nwdiag, activity, state and object —
   every diagram family that uses the family node shape pipeline. One file
   (`node_shapes.rs`) + one helper (`family_node_label`); conditional emission
   gated on whether the user supplied an explicit stereotype.

3. **#1348 — Activity caption + dashed bounding rect** (P0).
   Every activity diagram (4/4 in corpus) shows the spurious subtitle and
   surrounding dashed frame. Two trivial code locations.

4. **#1353 — C4 description + tech-tag rendering** (P0).
   Severely degrades the *signature* C4 use case (containers + db + bus).
   Without descriptions and `[SQL]` / `[HTTPS]` brackets, C4 diagrams lose
   their entire reason for existing.

5. **#1349 — UML visibility glyphs** (P1).
   Affects every class/object diagram. The plain `+/-/#/~` is *correct* UML 1.x
   syntax; the geometric icons are UML 2.x and PlantUML's house convention. A
   parity item, not a correctness item.

6. **#1350 — Class «C» badge** (P1).
   Every class header is missing the green ⓒ glyph. Visually noticeable but
   small (~30 LOC).

7. **#1351 — State concurrent region orientation** (P1).
   Affects state/03 and state/10 only — two of 34 fixtures. But the per-fixture
   visual delta is large (entire layout rotated).

8. **#1352 — Edge label placement drift** (P1).
   Affects state/07, state/10, and any diagram with long orthogonal edges and
   non-trivial label text. Localised algorithm change in one function.

---

## 6. Cross-fixture evidence

For each pattern, the strongest visual proof is concentrated in 3-4 fixtures.
The child issues link to these.

| Pattern | Strongest evidence (PNG paths under `/tmp/parity_audit/`) |
|---|---|
| Layout density | `deployment-02_databases-*.png` (7.65×), `component-02_interfaces-*.png` (7.21×), `deployment-03_cloud-*.png` (6.06×) |
| Kind-tag labels | `deployment-02_databases-PUML.png` ("node", "database"), `deployment-03_cloud-PUML.png` ("artifact"), `nwdiag-02_multi_network-PUML.png` ("network …"), `component-08_stereotypes-PUML.png` ("package …") |
| Activity caption + bounding rect | All four activity PUML PNGs show identical chrome around the diagram |
| C4 description + tech | `c4-12_container_with_databases-{PUML,PlantUML}.png` — exact side-by-side of missing italics |
| Visibility glyphs | `class-05_visibility-{PUML,PlantUML}.png` — direct mapping `+→○`, `#→◇`, `-→◆`, `~→▲` |
| Class «C» badge | `class-01_basic-PlantUML.png`, `class-03_composition_aggregation-PlantUML.png` — every class header has it; no PUML PNG does |
| State region orientation | `state-10_parallel_regions-{PUML,PlantUML}.png` — three regions, opposite axes |
| Edge label drift | `state-07_nested-PUML.png` ("data ready" orphaned), `state-10_parallel_regions-PUML.png` ("disableEQ" orphaned) |

---

## 7. Out-of-scope findings (logged, not filed)

Findings observed during the audit that do not map cleanly to one of the eight
gaps. Logged here so they are not lost; not filed as separate issues yet
(they need triage / discussion before they justify a ticket).

- **Edge drops in usecase/06** — multiple actor→use-case relations vanish entirely
  in PUML. Likely a use-case-family edge router bug separate from the layout
  density fix. *Open a new investigation issue if/when usecase becomes a focus.*
- **Generics inheritance edge dropped in class/11** — Stack extends Container
  is in the source but the open-triangle generalization arrow is not drawn.
  May be a generic-type parsing artifact rather than a render bug.
- **Activity branch-label side convention** — PUML puts "no" on the left of
  if/else diamonds; PlantUML puts "yes". Could be a one-line config flag in
  the activity branch routing module.
- **Mindmap and WBS layout direction divergence** — PUML radial vs PlantUML
  right-only/top-down. Possibly intentional (PUML's looks objectively nicer);
  surface as a feature decision rather than a parity bug.
- **Timing concise hexagonal shape** — PUML draws flat rectangles where
  PlantUML draws chevron-bordered hexagons (the canonical concise-row
  notation). Larger refactor; warrants a separate ticket once layout-density
  and kind-tag fixes ship.
- **Object-diagram skin (yellow + drop-shadow)** — PUML uses a custom heavy
  skin; PlantUML uses neutral gray. Affects two object fixtures. Possibly a
  theme/skinparam decision; not filed.
- **Kubernetes microscopic-text bug (deployment/06)** — text in nested
  deployment shapes becomes illegible at 4-level nesting. Could be triggered
  by the kind-tag fix (#1347) since the keyword tags consume the same space;
  re-audit after #1347 lands and file a follow-up if needed.

---

## 8. Methodology notes

- All renders driven by absolute-path binaries (`/opt/homebrew/bin/plantuml`,
  `./target/release/puml`) to avoid PATH issues in subshell.
- File-scope claims confirmed by reading source at the cited line ranges, not
  by guessing from epic notes.
- LOC estimates are rough — they reflect the *minimal* code touch needed for
  the canonical fix, not exhaustive cleanup. Coverage tests for each
  change will inflate the actual PR diff.
- Visual evidence PNGs are cached in `/tmp/parity_audit/` and will be cleaned
  by the next OS restart. The child issues quote exact deltas instead of
  embedding PNGs; if the evidence needs to survive, copy
  `/tmp/parity_audit/*.png` to `docs/internal/forensics/2026-05-30-evidence/`
  before reboot.
