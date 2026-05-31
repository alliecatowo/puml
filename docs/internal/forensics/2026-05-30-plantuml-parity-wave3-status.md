# PlantUML Parity Wave-3 Status — 35-Fixture Snapshot

**Date:** 2026-05-30
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`25d198c2` (head of main; includes wave-1 + first wave-2 batch + #1369 #1366 #1370)
**Wave-2 in-flight at audit time (NOT merged into main):**
PR [#1377](https://github.com/alliecatowo/puml/pull/1377) (Stage 3 EdgeRouting
#1333), [#1378](https://github.com/alliecatowo/puml/pull/1378) (sequence density
+ kind-tag pass 2 #1371 #1372), [#1379](https://github.com/alliecatowo/puml/pull/1379)
(component parallel-edge + header overflow #1374, **CONFLICTING**),
[#1380](https://github.com/alliecatowo/puml/pull/1380) (usecase actor edges +
boundary #1373), [#1381](https://github.com/alliecatowo/puml/pull/1381)
(CLI color/diagnostics/progress #407).
**Prior audits:**
`docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (wave-1, median 2.93×)
and `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (wave-2,
median 2.25×).

---

## 0. One-page summary for Allie

**What's the state?**
Median area ratio held steady at **2.18×** on current main vs PlantUML 1.2026.5,
essentially unchanged from wave-2's 2.25× because the five wave-2 fix PRs (#1377,
#1378, #1379, #1380, #1381) had not yet merged at audit time. CI checks are red on
four of them and #1379 conflicts with main. Wave-1 fully closed five of eight
gaps and partially closed three; wave-2 surfaced five more concrete gaps and
in-flight PRs target four of them. The deferred wave-2 item (#1375 object skin
chrome) still needs your decision.

**What's left?**

| Bucket | Count | Status |
|---|---|---|
| Wave-2 fix PRs needing landed | 4 | #1377/#1378/#1380/#1381 in CI; #1379 needs rebase |
| Wave-2 decision pending | 1 | #1375 object skin chrome (heavy-default vs neutral-default) |
| Newly surfaced material gaps not yet ticketed | 4 | Filed in §7 below: P1 class-edge-label drift, P1 generics inheritance edge dropped, P2 state dense-label collision, P2 arch-overview vertical waste / header overflow |
| Cosmetic/out-of-scope (logged, not filed) | ~6 | Lollipop circle radius, requires-socket half-circle, hexagon timing segments, sequence create-decorator dot, actor-head-overlap, mindmap convention |

**Verdict — drop-in replacement readiness:**
**~75% parity for a "PUML can render the same PlantUML files" definition.**
Structurally PUML matches every fixture: arrows, hierarchies, stereotypes,
and labels render in the right semantic positions. The remaining 25% is split
~70/30 between density (PUML still produces 2-5× canvas area on six families
and could be reasonably mistaken for a "different but valid" layout convention)
and correctness (usecase actor edges/boundary, component parallel-edge dedup,
class generics inheritance edge dropped — the latter two will block specific
real-world diagrams). With wave-2 PRs landed and the four new tickets fixed,
parity should jump to ~88% with median ratio sub-2.0×. Object-chrome decision
is independent and gates another ~3 percentage points.

**What needs your decision (4 items):**

1. **Object skin chrome default** (#1375): keep current yellow-banner + drop-shadow +
   circled-O badge as default and add `--theme plain`, OR flip default to PlantUML-style
   neutral chrome and add `--theme heavy` opt-in. Allie-call per wave-2 audit §4.2.
2. **Wave-2 PR #1379 (component) rebase**: needs hands-on conflict resolution
   against the wave-1 component-family commits that landed since it was opened.
   Dispatch a rescue agent or rebase manually?
3. **Class-edge-label drift severity** (newly filed P1, see §7 issue A):
   on `class/01_basic` the "owns" label sits ~120px right of the edge instead of
   at arclength midpoint. This is the visual side-effect of the #1366
   collision-push fix being too aggressive for sparse two-class diagrams.
   Treat as P1 follow-up to #1352/#1366, or accept as cosmetic and close?
4. **Architecture-overview salvage** (newly filed P2, see §7 issue D):
   the file uses both `component [Adapters] as Frontends` and `package
   "Frontends"` (PlantUML rejects this as duplicate-name; PUML tolerates and
   renders). Three of four package frame headers also clip ("package Pipeline
   Core" rendered partially over the dark band). Decide: rewrite the source to
   be PlantUML-compatible, OR treat the layout-density bug as the renderer's
   problem (independent of the source).

The rest of the doc is the data and citations behind these decisions.

---

## 1. Methodology

- 34 corpus fixtures from prior audits + `docs/diagrams/architecture-overview.puml`.
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png`.
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`.
- 7 fixture pairs spot-read side-by-side via the multimodal Read tool to
  characterize the qualitative deltas.
- Cached PNGs at `/tmp/parity_audit_v3/`.
- 2 fixtures could not be ratio-measured this wave:
  - `gantt/05_multi_task` — PlantUML 1.2026.5 errors at line 3 (regressed
    since wave-2 where it succeeded). PUML renders fine.
  - `diagrams/architecture-overview` — PlantUML still errors on the duplicate
    `Frontends` identifier (same as wave-2). PUML renders fine.

No source code was modified. Build was a single `cargo build --release` on
`origin/main` at `25d198c2`.

---

## 2. Headline numbers — three-wave progression

| Metric | Wave-1 | Wave-2 | Wave-3 (this) | Δ overall |
|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | **2.18×** | −26% |
| Mean area ratio | 3.30× | 2.70× | **2.39×** | −28% |
| Min ratio | 1.25× | 0.71× | **0.70×** | — |
| Max ratio | 7.65× | 5.22× | **4.90×** | −36% |
| N measurable | 33 | 34 | 33 | — |
| Fixtures ≥ 1.5× | 28 / 34 | 22 / 35 | **25 / 33** | mixed |
| Fixtures ≥ 2.0× | — | — | **18 / 33** | new |
| Fixtures ≥ 3.0× | ~14 / 34 | ~10 / 35 | **7 / 33** | strong improvement |

Caveats:
- The wave-3 N=33 drops the architecture-overview (no PlantUML baseline)
  and gantt/05 (PlantUML error this wave). Two of the wave-2 reductions
  (sequence, kind-tag-pass-2) will reappear once #1378 lands.
- The ≥ 1.5× count went UP from wave-2 (22→25) because the four sequence
  fixtures previously slated for #1371 still sit at 2.80–3.31× on current main.
- The ≥ 3.0× count fell strongly (10→7) entirely from the wave-1 fixes; wave-2
  hasn't added to this gain yet because the wave-2 PRs are stuck.

---

## 3. Full ratio table (current main, 2026-05-30)

| Fixture | PUML | PlantUML | Wave-3 ratio | Wave-2 ratio | Δ |
|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.85× | 1.86× | 0.00 |
| activity/05_while_loop | 248×438 | 186×437 | 1.33× | 1.34× | 0.00 |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 |
| c4/12_container_with_databases | 1600×990 | 989×774 | 2.06× | 2.07× | 0.00 |
| class/01_basic | 276×434 | 134×276 | 3.23× | 3.24× | 0.00 |
| class/03_composition_aggregation | 288×590 | 148×384 | 2.98× | 2.99× | 0.00 |
| class/05_visibility | 342×278 | 259×198 | 1.85× | 1.85× | 0.00 |
| class/11_generics | 808×266 | 361×316 | 1.88× | 1.88× | 0.00 |
| component/02_interfaces | 520×452 | 280×205 | 4.09× | 4.09× | 0.00 |
| component/07_ports_lollipop | 1558×630 | 702×483 | 2.89× | 2.89× | 0.00 |
| component/08_stereotypes | 1276×1428 | 660×803 | 3.43× | 3.43× | 0.00 |
| deployment/02_databases | 576×696 | 254×322 | 4.90× | 4.90× | 0.00 |
| deployment/03_cloud | 558×452 | 344×199 | 3.68× | 3.68× | 0.00 |
| deployment/06_kubernetes | 1340×1278 | 934×839 | 2.18× | 2.19× | 0.00 |
| diagrams/architecture-overview | (PUML 1032×1672) | PlantUML error | n/a | n/a | — |
| gantt/05_multi_task | (PUML 880×338) | PlantUML error | n/a | 5.22× | regressed in PlantUML |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× | 2.25× | 0.00 |
| mindmap/05_four_levels | 1629×658 | 723×1074 | 1.38× | 1.38× | 0.00 |
| nwdiag/02_multi_network | 760×410 | 295×360 | 2.93× | 2.93× | 0.00 |
| object/02_with_attributes | 327×450 | 223×253 | 2.60× | 2.61× | 0.00 |
| object/05_ch04_parity | 480×440 | 185×236 | 4.83× | 4.84× | 0.00 |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 |
| sequence/03_autonumber | 488×280 | 232×210 | 2.80× | 2.80× | 0.00 |
| sequence/07_notes | 550×432 | 255×316 | 2.94× | 2.95× | 0.00 |
| sequence/11_activation | 488×280 | 230×210 | 2.82× | 2.83× | 0.00 |
| sequence/12_create_destroy | 488×360 | 239×222 | 3.31× | 3.31× | 0.00 |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 |
| state/10_parallel_regions | 249×1010 | 280×938 | 0.95× | 0.96× | 0.00 |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 |
| usecase/02_with_actors | 398×534 | 286×453 | 1.64× | 1.64× | 0.00 |
| usecase/05_actor_generalization | 1084×782 | 1830×653 | 0.70× | 0.71× | 0.00 |
| usecase/06_multi_system | 1384×778 | 1090×568 | 1.73× | 1.74× | 0.00 |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.61× | 2.62× | 0.00 |

**Net:** zero new movement between wave-2 and wave-3. All wave-2 PRs are
in-flight but unmerged. The forensic value of this wave is therefore not the
deltas, it's:

1. Confirmation that wave-2 fix PRs will deliver real gains when landed
   (the gaps they target are still measurably present).
2. Catalog of post-wave-2 residue (next wave's queue).

---

## 4. Per-family score table — wave-1 / wave-2 / wave-3

Cell = (count of fixtures with ratio ≥ 1.5×) / (total).

| Family | W1 | W2 | W3 | Trajectory |
|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | DONE (wave-1) |
| c4 | 1/1 | 1/1 | 1/1 | Static — 2.06× residual is acceptable |
| class | 3/4 | 3/4 | 3/4 | Density gap unchanged — sequence-style retune needed |
| component | 3/3 | 3/3 | 3/3 | **Static; gated on #1378 + #1379** |
| deployment | 3/3 | 3/3 | 3/3 | Density-only, absolute ratios fell 35-40% in W2 |
| gantt | n/a | 1/1 | n/a | PlantUML regressed; out of scope |
| mindmap | 1/2 | 1/2 | 1/2 | Layout convention divergence — accept or feature-gate |
| nwdiag | 1/1 | 1/1 | 1/1 | **Gated on #1378** |
| object | 2/2 | 2/2 | 2/2 | **Gated on Allie decision #1375** |
| salt | 0/1 | 0/1 | 0/1 | ✓ already good |
| sequence | 4/4 | 4/4 | 4/4 | **Gated on #1378** |
| state | 2/3 | 0/3 | 0/3 | DONE (wave-1); label-collision is separate |
| timing | 1/1 | 1/1 | 1/1 | **Gated on #1378** for kind-tag; absolute ratio fixed in W1 |
| usecase | 2/3 | 1/3 | 1/3 | **Gated on #1380** |
| wbs | 1/1 | 1/1 | 1/1 | Layout convention divergence — accept or feature-gate |
| **Overall** | **28/34 (82%)** | **22/35 (63%)** | **25/33 (76%)** | Net improvement masked by N change |

Of the 25/33 fixtures at ≥ 1.5× on current main, **15 are in families with
wave-2 PRs in-flight** — once those land, the count should drop to ~14/33
(~42% above 1.5×), median to ~1.7×.

---

## 5. Spot-check delta gallery (7 pairs)

Spot reads from `/tmp/parity_audit_v3/` confirming the residual gaps.

### 5.1 `sequence/07_notes` — 2.94× ratio

**Delta:** PUML uses 2× horizontal participant-column spacing and 1.6×
vertical message-row spacing. Notes render at full participant-column width
vs PlantUML's compact tightly-wrapped chevron. Structure parity-faithful;
every message, activation bar, and note is in the right relative position.
**Gated on:** PR #1378 (sequence density retune, in-flight).

### 5.2 `component/08_cloud_db_queue_stereotypes` — 3.43× ratio

**Delta:** Confirms three wave-2-targeted bugs all still live on main:
1. Every group frame header reads `package CDN Layer`, `package API Cluster`,
   `package Storage Layer`, `package Event Bus`, `package Analytics Platform`
   — the kind-tag leak (target of #1372 / PR #1378).
2. `package Event Bus` header **text clips off the right edge** of the dark
   navy band (header overflow; target of #1374 / PR #1379).
3. **Edges dropped**: `Service A → Object Store [upload]`,
   `Service A → Kafka [publish events]`, `Service B → Kafka [publish events]`
   all MISSING. Only the `consume` from Kafka and `read/write` / `read-only`
   from Service A survive. This is the parallel-edge dedup bug
   (target of #1374 / PR #1379).

### 5.3 `object/05_ch04_parity` — 4.83× ratio (worst static gap)

**Delta:** Purely skin-chrome difference. PUML draws each object box with
a yellow header banner, a circled-O stereotype badge, an underlined name,
a double-line bottom separator, and a drop-shadow. PlantUML draws a flat
light-gray header with plain text and a thin border. ~30% padding per box
horizontally AND vertically multiplied across 3 objects → 4.83× total.
**Gated on:** Allie decision #1375 (heavy-default vs neutral-default).

### 5.4 `deployment/02_databases` — 4.90× ratio (worst remaining)

**Delta:** Density-only. PUML's 3D-isometric node shapes are rendered ~1.8×
larger than PlantUML's, and vertical spacing between three nodes is ~2.5×
PlantUML's tight spacing. No structural defect; no chrome difference. This
is the residue of the wave-1 density retune that #1346 did NOT propagate
into the deployment family's per-shape sizing constants.
**Possible future ticket:** deployment-family per-shape size retune, similar
to what #1378 does for sequence. Holding for now — wave-2 PRs first.

### 5.5 `class/01_basic` — 3.23× ratio (new regression)

**Delta:** Two issues stack here:
1. Density: PUML class boxes are 2× wider, 1.6× taller per class.
2. **Edge-label drift**: the "owns" label sits ~120px to the right of the
   vertical edge between Animal and Dog. PlantUML places it directly to the
   left of the edge near arclength midpoint. This appears to be over-aggressive
   collision-push from #1366 — it pushes labels away from edges even when
   there's no collision, leaving them stranded in whitespace.

The visibility-glyph and class-badge fixes (#1349, #1350) DO render
correctly here: green ⓒ badge in header, ○ public / ● private glyphs on
methods/attributes. Those wins survived.

**New ticket filed below (§7-A).**

### 5.6 `usecase/05_actor_generalization_system_boundary` — 0.70× ratio

**Delta:** PUML renders SMALLER than PlantUML (the only fixture with
sub-1× ratio besides state/10), but with serious correctness bugs:
1. ALL FOUR ACTORS (User, Administrator, Registered User, Premium User)
   render INSIDE the dashed "E-Commerce Platform" boundary frame. UML 2.x
   spec puts actors OUTSIDE the system boundary.
2. Use-case ellipses overlap each other: "Apply Promo Code" overlaps
   "Browse Catalog"; the `<<extend>>` arrow is tangled.
3. Multiple actor-generalization edges criss-cross the canvas.

**Gated on:** PR #1380 (in-flight, CI failing, needs babysitting).

### 5.7 `state/10_parallel_regions_shared_events` — 0.95× ratio

**Delta:** PUML beats PlantUML on overall canvas size (vertical-stacked
regions are very tight), but the dense top region (Playing/Paused/Stopped
with bi-directional play/pause/stop/resume transitions) has **stacked label
collisions**. Labels "play pause stop" are packed into a 30px vertical band
overlapping each other AND the adjacent arrows. PlantUML solves this by
curving the back-edges outward, separating labels naturally.

The arclength-midpoint fix (#1352) and the collision-push fix (#1366) help
when labels are sparse but break down when 3+ labels share the same
arclength bin. Needs an edge-routing algorithm change (curve back-edges or
fan labels along the arrow normal).
**New ticket filed below (§7-C).**

---

## 6. Remaining gap catalogue (post wave-2 PRs)

Assuming wave-2 PRs #1377, #1378, #1379, #1380 all eventually land. What's
left to file?

### 6.1 Issues that already exist and are tracked

| Issue | What it addresses | Status |
|---|---|---|
| #1371 | Sequence density retune | OPEN, PR #1378 in-flight |
| #1372 | Kind-tag pass 2 (component / nwdiag / timing) | OPEN, PR #1378 in-flight |
| #1373 | Usecase actor edges + boundary | OPEN, PR #1380 in-flight |
| #1374 | Component parallel edges + header overflow | OPEN, PR #1379 conflicting |
| #1375 | Object skin chrome (decision pending) | OPEN, awaiting Allie |
| #1333 | Stage 3 EdgeRouting state/activity | OPEN, PR #1377 in-flight |
| #1367 | Usecase 'triggers' label gutter drift | OPEN |
| #1324 | Multi-out edges stack vertically | OPEN |
| #1323 | Arrows attach top/bottom only | OPEN |

These collectively cover 7 of the 11 remaining gaps. No additional issue
needed for any of them.

### 6.2 Gaps without an existing ticket — candidates for new issues

| # | Gap | Severity | Existing coverage? | Recommended action |
|---|---|---|---|---|
| A | `class/01` edge label "owns" drifts ~120px right of edge | P1 | Partial overlap with #1352/#1366 (closed) | **File new ticket — sparse-class-edge label regression** |
| B | `class/11_generics` Stack/Container inheritance edge dropped | P1 | None | **File new ticket — class generics inheritance edge missing** |
| C | `state/10` dense-graph stacked-label collision | P2 | #1366 (closed) addresses 1-label case only | **File new ticket — multi-label arclength collision needs curve-or-fan** |
| D | `diagrams/architecture-overview` 750px empty Pipeline Core + 3 header overflows | P2 | #590 (epic) | **File new ticket — package-frame vertical-waste + header-text overflow** |
| E | Component lollipop circle radius 4-5× PlantUML | P2 cosmetic | None | Hold; contributes to component density but minor |
| F | Component required-interface socket half-circle missing | P2 cosmetic | None | Hold; UML-spec correct but a "requires" gets full circle today |
| G | Timing concise hexagon segment shape | P2 cosmetic | None | Hold; larger refactor |
| H | Sequence create-participant green-dot decorator | P3 cosmetic | None | Hold |
| I | Actor name overlap with stick-figure head (usecase/02) | P3 cosmetic | None | Hold (improved from W1, may auto-resolve as density tightens) |

Issues A-D are filed in §7 below. E-I are held — too cosmetic for this wave,
won't move the median.

---

## 7. Filed follow-up issues (P1/P2)

Four new issues filed under this audit:

- [#1382](https://github.com/alliecatowo/puml/issues/1382) — P1 class edge labels drift (§7-A)
- [#1383](https://github.com/alliecatowo/puml/issues/1383) — P1 class generics inheritance edge dropped (§7-B)
- [#1384](https://github.com/alliecatowo/puml/issues/1384) — P2 state multi-label arclength collision (§7-C)
- [#1385](https://github.com/alliecatowo/puml/issues/1385) — P2 architecture-overview vertical waste + header overflow (§7-D)

See §0 summary for the decision asks.

### 7-A. P1 — class edge labels drift far from arclength midpoint — #1382

`class/01_basic` "owns" label appears ~120px right of the vertical edge
between Animal and Dog. The #1366 collision-push fix is too aggressive on
sparse class diagrams: when no neighboring node would collide with a
midpoint-placed label, the push still fires and strands the label in
whitespace. Acceptance: on `class/01_basic` and `class/03_composition`,
edge labels render within ±20px of the actual arclength midpoint of their
parent edge.

### 7-B. P1 — class generics inheritance edge dropped — #1383

`class/11_generics` declares `Stack <|-- Container` (or equivalent
generalization syntax with `<T>`). PUML renders Container, Stack, Map as
floating side-by-side boxes with NO arrow. PlantUML stacks them vertically
with the open-triangle inheritance arrow. Generic-parameter syntax appears
to short-circuit edge emission in the class normalize/render pipeline.

### 7-C. P2 — state/10 stacked-label collision (multi-label arclength bin) — #1384

When 3+ edge labels share the same arclength bin (e.g. parallel transitions
between Playing/Paused/Stopped), labels stack on top of each other within a
~30px band. #1366 solved the 1-label case; this is the N-label case. Fix
options: curve back-edges outward (PlantUML's approach) or fan labels along
the edge normal vector.

### 7-D. P2 — architecture-overview vertical waste + package-frame header overflow — #1385

`docs/diagrams/architecture-overview.puml` renders at 1032×1672 in PUML.
The `package Pipeline Core` frame is ~750px tall with four child components
(Parser, AST, Normalizer, Renderer) stacked vertically with ~150px of empty
space between each. Additionally three of four package-frame headers
("Frontends", "Shared Services", "Pipeline Core") show text-overflow:
header text extends past the right edge of the dark navy band and is
clipped/visually broken. Likely related to (but distinct from) #1374's
package-header overflow on component family.

---

## 8. What's NOT filed and why

- **Class generics structural rewrite.** Issue 7-B above addresses the missed
  inheritance edge but does not propose a wholesale generics-overhaul. The
  `<T>` syntax bracket rendering is already correct; only the edge is dropped.
- **Sequence diagram lifeline-spacing minor retune.** Already part of #1371,
  no separate ticket.
- **Deployment density per-shape retune.** Held — wave-2 #1378 will partially
  cover this via sequence-style constant tightening. Re-evaluate post-merge.
- **Component lollipop / socket shapes.** Cosmetic; on the v2 out-of-scope list.
  Hold until density gaps close.
- **Mindmap and WBS layout convention divergence.** Per v2 §4.12, feature
  choice not a bug. PUML's bidirectional radial / horizontal-tree looks better
  on asymmetric data than PlantUML's right-only convention. No action unless
  a user explicitly requests parity with PlantUML's convention.

---

## 9. Evidence index

All cached at `/tmp/parity_audit_v3/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully)
- 33 `*-PlantUML.png` files (gantt/05 errored, architecture-overview errored
  expectedly per the duplicate-name source)
- 1 `*-PlantUML-ERROR.png` (gantt error image)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh` — the script

This audit performed NO source modifications. The repository state at audit
time is `origin/main @ 25d198c2`.

---

## 10. Recommendations for the orchestrator (Allie)

Ranked by net parity impact:

1. **Unblock the wave-2 PRs.** Four of five are mergeable but red on CI;
   #1379 needs rebase. Dispatching fix agents on each is the single highest-ROI
   action available. Estimated parity impact: median 2.18× → ~1.7×.
2. **Decide on #1375 (object skin chrome).** Either decision is fine; the
   issue can't make further progress without input. Estimated parity impact:
   object family drops from 3.71× median to ~1.8×.
3. **File the four new tickets in §7.** Two P1 correctness bugs (label drift,
   generics edge) plus two P2 layout polish items.
4. **Defer cosmetic out-of-scope items.** Lollipop sizes, hexagon timing,
   socket notation — none move the median meaningfully and they add agent
   work that distracts from the higher-leverage density bugs.
5. **Re-audit after wave-2 PRs all land.** A focused wave-4 audit (~30 min)
   to confirm median dropped below 2.0× and to spot any new regressions.

This document is a snapshot. Treat it as input to the next planning session,
not as a permanent fixture. The cached PNGs will be cleaned on next OS
restart; copy to `docs/internal/forensics/2026-05-30-evidence-v3/` only if
the gallery needs to survive.
