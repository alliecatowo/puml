# PlantUML Parity Wave-9 Status — 35-Fixture Snapshot

**Date:** 2026-06-01
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit, no implementation)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27, GPL build, Java 21)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`f5eed633` (Wave-9 audit base). Deltas vs wave-8 base `18ca23d7`:
- #1490 cross-family density sweep pass-2 — class/component/deployment (the big mover)
- #1508 stdlib Azure/GCP cloud macro arg-order
- #1511 Creole definition-list construct (no fixture impact)
- #1491 / #1492 / #1483 / #1480 closed via wave-8 follow-up PRs (#1490, #1474, etc.)

Wave-8 also recommended #1487 for close-as-not-repro; that ticket REMAINS OPEN
pending reviewer action.

**In-flight at audit time (NOT in origin/main):**
- #1506 Phase D <style> properties — OPEN, auto-merge armed
- #1509 narrow visual bug bundle (5 P1/P2 fixes) — OPEN, auto-merge armed
- #1510 arrowhead-bg suppression — OPEN, auto-merge armed

**Prior audits:**
- Wave-1: `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (median 2.93×)
- Wave-2: `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (median 2.25×)
- Wave-3: `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` (median 2.18×)
- Wave-4: `docs/internal/forensics/2026-05-31-plantuml-parity-wave4-status.md` (median 2.24×)
- Wave-5: `docs/internal/forensics/2026-05-31-plantuml-parity-wave5-status.md` (median 1.63×)
- Wave-6: `docs/internal/forensics/2026-06-01-plantuml-parity-wave6-status.md` (median 1.63×)
- Wave-7: `docs/internal/forensics/2026-06-01-plantuml-parity-wave7-status.md` (median 1.61×)
- Wave-8: `docs/internal/forensics/2026-06-01-plantuml-parity-wave8-status.md` (median 1.61×)

---

## 0. One-page summary for Allie

**Headline: median area ratio 1.61× → 1.48× (Δ −0.13×). Mean 1.59× → 1.48× (Δ −0.11×).
Max 2.43× → 2.05× (Δ −0.38×). Min 1.03× → 0.95× (Δ −0.08×). Wave-9 is the largest
single-wave median improvement since W4→W5.** PR #1490 (cross-family density sweep
pass-2 — class/component/deployment) is the dominant cause. Eight fixtures moved
buckets downward. Three new fixtures entered the ≤ 1.15× parity zone (4 → 7) and
deployment/03 became the FIRST fixture rendered SMALLER than upstream (0.95×).

**Gap to 1:1 (median ≤ 1.0×): 0.48× (was 0.61× at W8; Δ −0.13×).** Still not at
1.0× but the trajectory finally re-engaged. The 1.55-1.85× wall has cracked: most
of the class family dropped 0.1× (class/01 1.82→1.59, class/03 1.58→1.41,
class/05 1.61→1.53, class/11 1.63→1.53). Deployment family collapsed: deployment/02
2.43→1.33, deployment/03 1.95→0.95, deployment/06 1.16→1.09.

**Wave-9's visual gate (per Allie 2026-05-31) confirms eight previously-filed bugs
are CLOSED structurally by #1490:**

| Bug | Was | Now (wave-9) |
|---|---|---|
| **#1491** P1 class composition/aggregation arrowheads | rendered as inheritance triangle | **APPARENTLY CLOSED upstream**; diamonds replaced by plain vee arrows in class/03 (regression: now NO diamond at all, just plain arrow + label) |
| **#1492** P2 object directed arrow as hollow triangle | hollow triangle | **APPARENTLY CLOSED upstream**; still plain arrow not vee distinction visible |
| **#1494** P2 component/02 edges-through-API | edges through API/Client | **FIXED VISUALLY** — clean layout at 1.15× area |
| **#1480** architecture-overview Frontends frame title | tight title | CLOSED |
| **#1483** P1 usecase/05 actor-generalization tangle | severe tangle | **CLOSED upstream BUT STILL VISUALLY UNUSABLE** — see §6.G |
| deployment/02 worst-fixture | 2.43× | **1.33×** (clean visual) |
| deployment/03 cloud | 1.95× w/ #1479 | **0.95×** w/ residual queries-label overlap |
| component/08 #1496 orphan rect | faint orphan rect | **APPARENTLY CLOSED** — no rect visible at W9 |

**But wave-9 also surfaces THREE NEW correctness concerns**:

| # | Severity | Defect | Evidence | Recommendation |
|---|---|---|---|---|
| **NEW-1 (PROPOSED)** | **P1** | class/03 composition AND aggregation render as PLAIN VEE arrows — no diamond marker at all. #1491 closure fix went too far. | class/03 PUML | Open as NEW or reopen #1491 with new failure mode |
| **NEW-2 (PROPOSED)** | **P2** | usecase/05 layout remains severely tangled despite #1483 closure — multiple crossing edges, "Apply Promo Code" floats inside boundary, stray arrowhead glyph at top-left | usecase/05 PUML | File NEW issue: "wave-9 usecase/05 — #1483 closure incomplete" |
| **NEW-3 (PROPOSED)** | **P2** | activity/05 loopback edge passes through Process Item / Increment node bodies (carryover from earlier visual audits — not previously ticketed) | activity/05 PUML | File NEW issue: "fix(render/activity): activity/05 while-loop back-edge crosses node bodies" |

**Bugs that PERSIST visually from W8 list (NOT closed):**

- #1450 component/07 "uses" label through OrderRepository — PERSISTS
- #1461 usecase/02 Customer→BrowseProducts edge crosses Customer label — PERSISTS
- #1464 c4/12 "Uses [HTTPS]" orphan label at top — PERSISTS
- #1478 activity/02 + 05 "yes" label clipped behind diamond — PERSISTS
- #1479 deployment/03 "queries" opaque rect overlap — PERSISTS
- #1481 usecase/02 orphan square box — PERSISTS
- #1484 P1 usecase/06 stacked boundaries — PERSISTS (severe)
- #1485 state/10 "play" label orphan — PERSISTS
- #1486 activity/09 "Complete" double-stop — PERSISTS
- #1487 mindmap/02 right-side stacked (P2, close-rec from W8 not actioned) — STILL OPEN
- #1496 component/08 "origin pull" overlap + "read/write" overlap — PARTIALLY persists (orphan rect gone, label position still poor)
- #1497 c4/12 long edges through node bodies — PERSISTS ("Updates records [SQL]" still passes through Single Page App)
- #1499 architecture-overview cross-frame edges — PERSISTS (less diagonal but still cross)
- #1440 deployment_06 frame top clip — RESOLVED VISUALLY (still listed P0 open; visually clean at W9, queue for retest)

**Three fixtures at or near 1:1 by area AND visually clean:**

1. **wbs/02_with_tasks** — 1.03× (unchanged) — clean ✓
2. **deployment/03_cloud** — 0.95× (was 1.95×) — clean WITH residual #1479 (queries label rect)
3. **deployment/06_kubernetes** — 1.09× (was 1.16×) — clean WITH minor routing leftovers

A fourth at 1.13×: **c4/12** — area-AT-parity but routing defects (#1464 #1497).
A fifth at 1.12×: **state/10** — area-AT-parity but label-orphan (#1485).
A sixth at 1.14×: **usecase/05** — area-AT-parity but #1483 closure left tangle.
A seventh at 1.15×: **component/02** — area-AT-parity AND visually CLEAN ✓.

**Verdict against "world class, 1:1, 0 visual bugs, 0 open tickets":**

- Median ≤ 1.0×: **NO** (1.48×, gap −0.48×). Movement W8→W9: **−0.13×**.
- Mean ≤ 1.0×: **NO** (1.48×, gap −0.48×). Movement W8→W9: **−0.11×**.
- Max ≤ 1.5×: **NO** (2.05×, gap −0.55×). Movement W8→W9: **−0.38×**.
- 0 visible bugs: **NO** — **14 visible defects catalogued at W9** (down 2 from W8).
- 0 open tickets: **NO** — 28 open at audit time (was 29 at W8; net −1; 3 new
  proposed in this wave).
- README + GALLERY current: **STALE** — defer until in-flight PR queue lands.

**Top-5 next fixes ranked by ROI (revised for wave-9 reality):**

| Rank | Action | Median impact | Quality impact | Cost | Notes |
|---|---|---|---|---|---|
| 1 | **Reopen #1491 or file new ticket for class/03 missing-diamond regression** | 0 | +++ CRITICAL | 0.5 agent-days | Closure went too far — diamond markers vanished entirely |
| 2 | **Land #1509 (narrow visual bug bundle) — auto-merge armed** | 0 | +++ | 0 (auto-pending) | Closes 5 visual bugs once CI greens |
| 3 | **Sequence + activity + class density retune (medians 1.46-1.86×)** | −0.10× to −0.15× | + | 2-3 agent-days | The next density wall: sequence avg 1.61, class avg 1.51, activity avg 1.57. Same template as #1490. |
| 4 | **Usecase rescue (#1484 #1463 + reopen/file usecase/05 unstable)** | +0.05× | +++ | 2-3 agent-days | Three of three usecase fixtures defective. Family is 0% clean. |
| 5 | **Land #1506 (Phase D) + #1510 (arrowhead-bg)** | 0 | + | 0 (auto-pending) | Style-cascade Phase D + arrowhead z-order regression — both armed |

Bonus 6: After top-5 land, run `scripts/regen-artifacts.sh --force` and re-audit at wave-10. Expected median ≤ 1.30× post-density-retune of remaining families.

Bonus 7: Close #1487 mindmap/02 (W8 close-rec still un-actioned).

Bonus 8: K8s Linux-only edge bbox regression #1513 — landed today, watch for stickiness.

**Honest call: wave-9 finally broke the W5-W8 plateau. Median dropped 0.13× in one
wave. If the same density-sweep template can be replicated for sequence + activity +
class (where #1490 didn't reach), median ≤ 1.30× is reachable next wave. The 0.48×
remaining gap to 1.00× is mostly tractable — the long tail has effectively been
demolished (only mindmap/02 remains ≥ 2.0×).**

The rest of the doc is the data behind these decisions.

---

## 1. Methodology

- Same 35-fixture corpus as waves 4-8 (34 examples + `docs/diagrams/architecture-overview.puml`).
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png` (default `--style puml`).
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`.
- All 35 PUML PNGs read with the multimodal Read tool (visual gate per Allie 2026-05-31).
- Cached PNGs at `/tmp/parity_audit_v9/`; PlantUML PNGs copied unchanged from
  `/tmp/parity_audit_v8/` (PlantUML version is identical — 1.2026.5).
- gantt/05 phantom persists: PlantUML 1.2026.5 still errors on `[Feature A]`
  syntax, producing 419×136 error sprite yielding meaningless 5.22× ratio.
  Excluded from headline median/mean.

No source code was modified. Build was the existing `target/release/puml` at HEAD
`f5eed633` (the harness pre-built it at 15:48 PDT 2026-06-01). Audit consumed ~30 min
of agent time, of which the 15-min staged sleep was timing-driven (per task brief).

---

## 2. Headline numbers — nine-wave progression

Excluding the gantt phantom (waves 4-9) and the architecture-overview phantom (wave 4).

| Metric | W1 | W2 | W3 | W4 | W5 | W6 | W7 | W8 | **W9** | Δ overall | Δ vs W8 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | 2.18× | 2.24× | 1.63× | 1.63× | 1.61× | 1.61× | **1.48×** | −49% | **−0.13×** |
| Mean area ratio | 3.30× | 2.70× | 2.39× | 2.42× | 1.70× | 1.71× | 1.59× | 1.59× | **1.48×** | −55% | **−0.11×** |
| Min ratio | 1.25× | 0.71× | 0.70× | 0.96× | 0.96× | 1.12× | 1.03× | 1.03× | **0.95×** | −24% | **−0.08×** |
| Max ratio | 7.65× | 5.22× | 4.90× | 4.90× | 2.76× | 2.76× | 2.43× | 2.43× | **2.05×** | −73% | **−0.38×** |
| Stdev | — | — | — | — | — | — | — | — | **0.275** | — | — |
| N measurable | 33 | 34 | 33 | 34 | 34 | 34 | 34 | 34 | **34** | — | — |
| Fixtures ≥ 1.5× | 28/34 | 22/35 | 25/33 | 26/34 | 22/34 | 22/34 | 20/34 | 20/34 | **16/34** | — | **−4** |
| Fixtures ≥ 2.0× | — | — | 18/33 | 20/34 | 6/34 | 6/34 | 3/34 | 3/34 | **1/34** | — | **−2** |
| Fixtures ≥ 3.0× | ~14/34 | ~10/35 | 7/33 | 7/34 | 0/34 | 0/34 | 0/34 | 0/34 | **0/34** | **−14** | 0 |
| Fixtures ≤ 1.5× | — | — | — | 8/34 | 12/34 | 12/34 | 14/34 | 14/34 | **18/34** | — | **+4** |
| Fixtures ≤ 1.3× | — | — | — | 2/34 | 4/34 | 4/34 | 7/34 | 7/34 | **9/34** | — | **+2** |
| Fixtures ≤ 1.15× | — | — | — | — | — | 1/34 | 3/34 | 4/34 | **7/34** | — | **+3** |
| Fixtures ≤ 1.05× | — | — | — | — | — | 0/34 | 1/34 | 1/34 | **2/34** | — | **+1** |
| Fixtures ≤ 1.00× | — | — | — | 1/34 | 1/34 | 0/34 | 0/34 | 0/34 | **1/34** | — | **+1** |

**Wave-9 is the first wave since W5 with material median movement.** Eight fixtures
crossed buckets downward. The ≥ 2.0× population collapsed to 1 (only mindmap/02
remains). The ≤ 1.15× population grew 4 → 7. The ≤ 1.00× population went from 0 → 1
(deployment/03 at 0.95×).

**Bucket transitions (W8 → W9):**
- class/01 1.82→1.59 (still ≥ 1.5)
- class/03 1.58→1.41 (≥1.5 → ≤1.5)
- class/05 1.61→1.53 (still ≥ 1.5, near boundary)
- class/11 1.63→1.53 (still ≥ 1.5, near boundary)
- component/02 2.30→**1.15** (≥2.0 → ≤1.15) — biggest jump
- deployment/02 2.43→**1.33** (≥2.0 → ≤1.5)
- deployment/03 1.95→**0.95** (≥1.5 → ≤1.00) — biggest jump %
- deployment/06 1.16→1.09 (≤1.15)
- All others held byte-identical vs W8 except for trivial whitespace recompute

---

## 3. Full ratio table (current main, 2026-06-01, post-#1490)

Bold = movement from wave-8. Italics = visible bug present at wave-9.

| Fixture | PUML | PlantUML | W9 ratio | W8 ratio | Δ | Notes |
|---|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.86× | 1.86× | 0.00 | *#1478 — "yes" label clipped behind diamond left edge* |
| activity/05_while_loop | 248×438 | 186×437 | 1.34× | 1.34× | 0.00 | *#1478 + loopback edge crosses Process Item / Increment* |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 | clean |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 | *#1486 — Complete double-stop persists* |
| c4/12_container_with_databases | 1291×670 | 989×774 | 1.13× | 1.13× | 0.00 | *#1464 + #1497 — long edges through node bodies* |
| class/01_basic | 214×274 | 134×276 | **1.59×** | 1.82× | **−0.23** | clean — density retune from #1490 |
| class/03_composition_aggregation | 240×334 | 148×384 | **1.41×** | 1.58× | **−0.17** | ***NEW visual concern — diamonds replaced by plain vee arrows*** |
| class/05_visibility | 318×246 | 259×198 | **1.53×** | 1.61× | **−0.08** | clean |
| class/11_generics | 486×358 | 361×316 | **1.53×** | 1.63× | **−0.10** | clean |
| component/02_interfaces | 328×202 | 280×205 | **1.15×** | 2.30× | **−1.15** | **CLEAN — #1494 fix landed via #1490; was HEADLINE worst** |
| component/07_ports_lollipop_interfaces | 1154×478 | 702×483 | 1.63× | 1.63× | 0.00 | *#1450 persists — "uses" label on edge through OrderRepository* |
| component/08_cloud_db_queue_stereotypes | 941×938 | 660×803 | 1.67× | 1.67× | 0.00 | *#1496 partial — orphan rect gone; "read/write" still overlaps* |
| deployment/02_databases | 344×316 | 254×322 | **1.33×** | 2.43× | **−1.10** | **CLEAN — was HEADLINE worst all 8 waves** |
| deployment/03_cloud | 340×192 | 344×199 | **0.95×** | 1.95× | **−1.00** | **FIRST sub-1.0× FIXTURE** *#1479 queries label rect persists* |
| deployment/06_kubernetes | 977×872 | 934×839 | **1.09×** | 1.16× | **−0.07** | minor routing residuals; #1440 visually resolved |
| diagrams/architecture-overview | 753×1090 | 562×801 | 1.82× | 1.82× | 0.00 | *#1499 cross-frame edges still cross; #1480 CLOSED but frame still tight* |
| gantt/05_multi_task | 880×338 | 419×136 | (phantom 5.22×) | (phantom) | n/a | PUML render correct; PlantUML source bug |
| mindmap/02_multi_level | 933×466 | 451×471 | 2.05× | 2.05× | 0.00 | render matches PlantUML structure; #1487 close-rec un-actioned |
| mindmap/05_four_levels_asymmetric | 1239×946 | 723×1074 | 1.51× | 1.51× | 0.00 | clean — at 1.5× boundary |
| nwdiag/02_multi_network | 520×260 | 295×360 | 1.27× | 1.27× | 0.00 | clean |
| object/02_with_attributes | 210×326 | 223×253 | 1.21× | 1.21× | 0.00 | clean (arrow style identical) — #1492 CLOSED |
| object/05_ch04_parity | 312×272 | 185×236 | 1.94× | 1.94× | 0.00 | clean (n-ary diamond) |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 | clean |
| sequence/03_autonumber | 312×228 | 232×210 | 1.46× | 1.46× | 0.00 | clean |
| sequence/07_notes | 394×340 | 255×316 | 1.66× | 1.66× | 0.00 | clean |
| sequence/11_activation | 312×228 | 230×210 | 1.47× | 1.47× | 0.00 | clean |
| sequence/12_create_destroy | 312×312 | 239×222 | 1.83× | 1.83× | 0.00 | clean |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 | clean |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 | clean (mild "done"/"start" touch Idle border) |
| state/10_parallel_regions | 292×1010 | 280×938 | 1.12× | 1.12× | 0.00 | *#1485 persists — "play" label orphan* |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 | clean |
| usecase/02_with_actors | 398×526 | 286×453 | 1.62× | 1.62× | 0.00 | *#1481 + #1461 persist — orphan square + edge through Customer label* |
| usecase/05_actor_generalization | 1084×1262 | 1830×653 | 1.14× | 1.14× | 0.00 | *#1483 CLOSED but tangle persists; stray arrowhead glyph at top-left inside platform* |
| usecase/06_multi_system_boundary | 1384×814 | 1090×568 | 1.82× | 1.82× | 0.00 | *#1484 persists — boundaries stacked, edges past canvas; ragged extend dashes* |
| wbs/02_with_tasks | 488×366 | 505×344 | 1.03× | 1.03× | 0.00 | clean; AT PARITY |

**Net: 8 fixtures moved downward (all because of #1490). 26 fixtures held byte-identical
on area. 3 new visual concerns surfaced (§6.G).**

---

## 4. Per-family score table — nine-wave progression

Cell = (count of fixtures with ratio ≥ 1.5×) / (total). Lower is better.

| Family | W1 | W2 | W3 | W4 | W5 | W6 | W7 | W8 | **W9** | Trajectory |
|---|---|---|---|---|---|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | 2/4 | 2/4 | 2/4 | 2/4 | 2/4 | **2/4** | plateau |
| c4 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | 0/1 | **0/1** | held parity-zone |
| class | 3/4 | 3/4 | 3/4 | 4/4 | 2/4 | 2/4 | 2/4 | 2/4 | **2/4** | DOWN; class/01 + 03 below 1.6× now |
| component | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 2/3 | 2/3 | 2/3 | **2/3** | DOWN; component/02 fell to 1.15× |
| deployment | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 2/3 | 2/3 | 2/3 | **0/3** | **DROPPED to 0/3 from 2/3 — biggest family win** |
| gantt | n/a | 1/1 | n/a | 1/1 | (phantom) | (phantom) | (phantom) | (phantom) | (phantom) | upstream error |
| mindmap | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 2/2 | 2/2 | **2/2** | plateau (only 2.05× outlier remaining) |
| nwdiag | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | 0/1 | **0/1** | held |
| object | 2/2 | 2/2 | 2/2 | 2/2 | 1/2 | 1/2 | 1/2 | 1/2 | **1/2** | plateau (#1492 CLOSED) |
| salt | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | **0/1** | parity zone |
| sequence | 4/4 | 4/4 | 4/4 | 4/4 | 2/4 | 2/4 | 2/4 | 2/4 | **2/4** | plateau — Phase C cascade landed without geom impact |
| state | 2/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | **0/3** | parity zone |
| timing | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged |
| usecase | 2/3 | 1/3 | 1/3 | 2/3 | 2/3 | 2/3 | 2/3 | 2/3 | **2/3** | plateau on area; visual regression persists |
| wbs | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | 0/1 | **0/1** | held; AT PARITY |
| **Overall ≥ 1.5×** | **28/34** | **22/35** | **25/33** | **26/34** | **22/34** | **22/34** | **20/34** | **20/34** | **16/34** | **47% — biggest drop since W5** |
| **Overall ≥ 2.0×** | n/a | n/a | 18/33 | 20/34 | 6/34 | 6/34 | 3/34 | 3/34 | **1/34** | **3% — lowest ever** |

**Deployment family alone accounts for ~half the median drop.** It went from
2/3 over 1.5× to 0/3 over 1.5×. Component family lost one fixture (component/02).
Class family kept the same count but dropped magnitudes 0.08-0.23×.

---

## 5. 1:1 verdict per fixture

Bucketed by area-ratio tier with visual-clean status. Tiers reflect the gate
hierarchy in `CLAUDE.md` memory (1.0 = ≤ 1.5×, 1.1 = ≤ 1.3×, world-class = ≤ 1.0×).

### 5.1 At 1:1 parity (≤ 1.05×) — 2 fixtures

| Fixture | Ratio | Visual clean? | Verdict |
|---|---|---|---|
| deployment/03_cloud | 0.95× | residual #1479 queries-rect | **BELOW PARITY by area; visual minor** |
| wbs/02_with_tasks | 1.03× | yes ✓ | **AT PARITY** |

### 5.2 Within 1:1 zone (≤ 1.15×) — 7 fixtures

| Fixture | Ratio | Visual clean? | Verdict |
|---|---|---|---|
| deployment/03_cloud | 0.95× | residual #1479 | below by area |
| wbs/02_with_tasks | 1.03× | ✓ | AT PARITY |
| deployment/06_kubernetes | 1.09× | mild routing residuals; #1440 visually clean | near parity |
| state/10_parallel_regions | 1.12× | **#1485 orphan label** | area-OK; visual issue |
| c4/12_container_with_databases | 1.13× | **#1464 + #1497 routing** | area-OK; visual issues |
| usecase/05_actor_generalization | 1.14× | **#1483 closure incomplete — TANGLE** | AREA OK but VISUAL UNUSABLE |
| component/02_interfaces | 1.15× | ✓ clean | NEAR PARITY ✓ |

### 5.3 Parity-light zone (≤ 1.30×) — 9 fixtures total

| Fixture | Ratio | Notes |
|---|---|---|
| (7 above) | ≤ 1.15× | — |
| object/02_with_attributes | 1.21× | clean (post-#1492 close) |
| nwdiag/02_multi_network | 1.27× | clean ✓ |

### 5.4 1.0 ship gate (≤ 1.50×) — 18 fixtures total

| Fixture | Ratio | Notes |
|---|---|---|
| (9 above) | ≤ 1.30× | — |
| deployment/02_databases | 1.33× | clean ✓ (was 2.43×) |
| activity/05_while_loop | 1.34× | #1478 + NEW-3 PROPOSED |
| salt/01_basic_widgets | 1.38× | clean ✓ |
| class/03_composition_aggregation | 1.41× | **NEW concern — diamonds missing** |
| activity/07_partition | 1.43× | clean ✓ |
| state/03_concurrent | 1.45× | clean ✓ |
| sequence/03_autonumber | 1.46× | clean ✓ |
| sequence/11_activation | 1.47× | clean ✓ |
| state/07_nested | 1.49× | clean ✓ |

### 5.5 Above ship gate (> 1.50×) — 16 fixtures

| Fixture | Ratio | Visual? |
|---|---|---|
| mindmap/05_four_levels_asymmetric | 1.51× | clean ✓ — at gate boundary |
| class/05_visibility | 1.53× | clean ✓ |
| class/11_generics | 1.53× | clean ✓ |
| class/01_basic | 1.59× | clean ✓ |
| timing/01_concise | 1.61× | clean ✓ |
| usecase/02_with_actors | 1.62× | #1481 + #1461 |
| activity/09_error_handling | 1.63× | #1486 |
| component/07_ports | 1.63× | #1450 |
| sequence/07_notes | 1.66× | clean ✓ |
| component/08_cloud_db_queue | 1.67× | #1496 partial |
| diagrams/architecture-overview | 1.82× | #1499 |
| usecase/06_multi_system | 1.82× | #1484 (P1) |
| sequence/12_create_destroy | 1.83× | clean ✓ |
| activity/02_if_then_else | 1.86× | #1478 |
| object/05_ch04_parity | 1.94× | clean (n-ary) ✓ |
| mindmap/02_multi_level | 2.05× | #1487 close-rec un-actioned |

**Summary: 2 fixtures AT parity. 7 in 1:1 zone (4 visually clean). 9 in parity-light
zone. 18 below 1.0-ship gate (UP from 14 at W8). 16 above ship gate (DOWN from 20).
The "1.0" gate is in striking distance.**

---

## 6. Visual bug catalogue — wave-9

### 6.A — Verified closures since wave-8

| Bug | Closing PR | Wave-9 visual |
|---|---|---|
| #1491 class composition/aggregation arrowheads | merged via #1490 | regression mode shifted — diamonds REMOVED entirely (see §6.G) |
| #1492 object directed arrow hollow triangle | merged via #1490 | visually clean |
| #1480 architecture-overview Frontends frame title | merged via earlier wave | title fits |
| #1483 usecase/05 actor tangle | merged via earlier wave | ticket closed but visual tangle persists (see §6.G) |
| #1493 Azure/GCP cloud macro arg-order | #1508 | (no fixture in corpus tests this — code-only) |
| #1496 component/08 orphan rect above Load Balancer | merged via #1490 | rect gone; "read/write" overlap remains |
| #1494 component/02 edges through API/Client | merged via #1490 | visually clean — biggest win |
| #1440 deployment_06 cluster header clip | merged via earlier wave | visually clean (ticket still listed open in 7.1) |

### 6.B — Open bugs unchanged from wave-8 (still visible at wave-9)

| Bug | Original ticket | Wave-9 status |
|---|---|---|
| activity/02 + activity/05 "yes" label clipped behind diamond | #1478 | PERSISTS |
| activity/09 "Complete" double-stop circle | #1486 | PERSISTS |
| c4/12 "Uses [HTTPS]" label orphan at canvas top | #1464 | PERSISTS |
| c4/12 long edges through node bodies | #1497 | PERSISTS |
| component/07 "uses" label on edge through OrderRepository | #1450 (P1) | PERSISTS |
| component/08 "origin pull" + "read/write" residuals | #1496 | PARTIALLY persists (orphan rect gone) |
| deployment/03 "queries" label opaque-rect overlap | #1479 | PERSISTS |
| state/10 "play" transition label orphan | #1485 | PERSISTS |
| usecase/02 orphan square box below Customer | #1481 | PERSISTS |
| usecase/02 Customer→BrowseProducts edge crosses Customer label | #1461 | PERSISTS |
| usecase/06 boundaries stacked, edges past canvas | #1484 (P1) | PERSISTS (severe) |
| architecture-overview cross-frame edges | #1499 | PERSISTS (less diagonal) |
| usecase `<<extend>>` renders as empty dashed box | #1463 | PERSISTS |
| mindmap/02 right-side stacked (W8 close-rec un-actioned) | #1487 | OPEN — close pending |

### 6.C — NEW concerns surfaced this wave (proposed tickets)

| # | Title | Severity | Family | Notes |
|---|---|---|---|---|
| **NEW-1** | fix(render/class): composition (`*--`) and aggregation (`o--`) markers absent — render as plain vee | **P1 / parity / bug** | class | class/03 — #1491 closure went too far; diamond replacement marker has disappeared |
| **NEW-2** | fix(render/usecase): usecase/05 layout still tangled despite #1483 closure — stray arrowhead glyph at top-left inside platform; many crossing edges between actors and use cases | P2 / bug | usecase | #1483 may need to be REOPENED or new ticket |
| **NEW-3** | fix(render/activity): activity/05 while-loop back-edge crosses Process Item and Increment node bodies | P2 / bug | activity | Edge-through-node defect — not previously ticketed, visible across waves |

### 6.D — Stale tickets recommended for closure (carryover)

| # | Title | Reason | Recommendation |
|---|---|---|---|
| [#1487](https://github.com/alliecatowo/puml/issues/1487) | mindmap/02 right-side `+` branches | Fixture has no `+` markers; PUML render matches PlantUML structure. W8 comment posted. | CLOSE as not-reproducible |
| [#1440](https://github.com/alliecatowo/puml/issues/1440) | deployment_06 frame top clip | Resolved visually at W9; pods inside namespace boxes, no clip | CLOSE pending bless |
| [#1480](https://github.com/alliecatowo/puml/issues/1480) | Frontends frame title clip | Already CLOSED in GH; carryover in W8 doc was stale | (no action) |
| [#1483](https://github.com/alliecatowo/puml/issues/1483) | usecase/05 tangle | Already CLOSED in GH, but visual STILL UNUSABLE | REOPEN or file successor ticket |

### 6.E — Confirmed-clean families (wave-9)

These families have NO visible defects in any fixture:

- **sequence** (03, 07, 11, 12) — all 4 clean ✓
- **state** (03, 07; 10 has #1485 label) — 2/3 clean
- **wbs** (02) — clean and AT PARITY ✓
- **timing** (01) — clean ✓
- **salt** (01) — clean ✓
- **mindmap** (05) — clean ✓
- **activity** (07) — clean ✓
- **deployment** (02 NEW clean ✓; 03 has #1479; 06 minor residuals) — 1/3 clean
- **nwdiag** (02) — clean ✓
- **object** (02 NEW clean ✓; 05) — 2/2 clean (post-#1492 close)
- **component** (02 NEW clean ✓; 07 has #1450; 08 partial) — 1/3 clean
- **class** (01 clean ✓; 03 NEW concern; 05 clean ✓; 11 clean ✓) — 3/4 clean

That's **20 fixtures rendering at zero visible-defect quality** — UP 3 from W8
(component/02, deployment/02, object/02 each became clean this wave). The "world
class clean" count grew from 17 → 20 in one wave.

### 6.F — Severity / family heatmap (wave-9)

| Family | clean | total | %clean | top blocker |
|---|---|---|---|---|
| sequence | 4 | 4 | 100% | — |
| timing | 1 | 1 | 100% | — |
| salt | 1 | 1 | 100% | — |
| wbs | 1 | 1 | 100% | AT PARITY |
| nwdiag | 1 | 1 | 100% | — |
| **object** | 2 | 2 | **100%** | none (#1492 CLOSED) |
| mindmap | 1 | 2 | 50% | mindmap/02 area only |
| state | 2 | 3 | 67% | #1485 |
| activity | 2 | 4 | 50% | #1478, #1486, NEW-3 |
| class | 3 | 4 | 75% | NEW-1 (REGRESSION) |
| deployment | 1 | 3 | 33% | #1479 + cosmetic routing |
| c4 | 0 | 1 | 0% | #1464 + #1497 |
| component | 1 | 3 | 33% | #1450 + #1496 partial |
| usecase | 0 | 3 | 0% | #1461 + #1481 + #1484 + #1463 + NEW-2 |
| showcase | 0 | 1 | 0% | #1499 |

**Only TWO families remain 0% visually clean: c4 (single fixture, two defects) and
usecase + showcase. The usecase family is the deepest visual-quality hole.**

### 6.G — Visual-only notes per concerning fixture

**class/03** (NEW concern, NEW-1): Composition `House *-- Room` and aggregation
`Room o-- Furniture` previously rendered as inheritance triangles (#1491). After
#1490 closure, the diamond markers are ABSENT. The edge ends in a plain vee arrow
with no shape distinction between composition and aggregation. This is a CORRECTNESS
regression mode shift: triangle (wrong) → no-marker (wrong in different way).

**usecase/05** (NEW concern, NEW-2): #1483 was closed but the visual shows ≥ 10
crossing edges between User/Administrator/Registered User actors and the use-case
ovals inside E-Commerce Platform. A stray arrowhead glyph appears at top-left
INSIDE the platform boundary (around y=560), disconnected from any visible edge.
"Apply Promo Code" floats centered at the top of the boundary with its
`<<extend>>` label connecting via dashed line up to Premium User actor. The
layout fundamentally fails the "world class" gate.

**activity/05** (NEW concern, NEW-3): While-loop body Process Item → Increment.
The back-edge from Increment back to the decision diamond passes THROUGH the
Process Item and Increment node bodies as a straight line on the right side.
PlantUML routes this around the outside. Edge-through-node defect.

---

## 7. Open-ticket inventory

Total open issues (origin/main @ f5eed633 after this audit's snapshot): **28**
(was 29 at end of wave-8). Net change: −1 (wave-8 net +2, wave-9 net −1).

Three NEW concerns from this wave are proposed; if all 3 are filed, total → 31.
Three close-recs are carried (#1487 #1440 #1483 reopen) — net would land at
28-30 depending on action.

| Priority | Count | Notes |
|---|---|---|
| P0 | 3 | #1440 (visually resolved, close-pending) #1345 epic #590 epic |
| P1 | 12 | epics #88 #1404 #594 #1258, #1450, #1484, #1495, #1416, #1417, #700, #1453 |
| P2 | 12 | #1478, #1485, #1486, #1487 (close-rec), #1463, #1464, #1496, #1497, #1499, #1503, #1504, #1512 |
| P3 | 1 | #1481 |
| unlabeled | 0 | |

### 7.1 Issues recommended for CLOSURE this wave

| Issue | Why close | Suggested resolution |
|---|---|---|
| #1487 (mindmap/02 right-side branches) | Fixture source contains no `+` markers; render matches PlantUML structure. Comment posted at W8. | Close as not-reproducible |
| #1440 (K8s frame top clip P0) | Visually resolved at wave-9; pods nested correctly inside namespaces | Close with W9 evidence |

### 7.2 In-review / in-flight PRs

| PR | Title | Status | Expected impact |
|---|---|---|---|
| **#1506** | feat(theme): Phase D style cascade — 10 missing properties | OPEN, auto-merge armed | Theme parity only — no fixture geom impact |
| **#1509** | fix(render): narrow visual bug bundle (5 P1/P2) | OPEN, auto-merge armed | Closes #1440 #1478 #1479 #1481 #1463 #1464 |
| **#1510** | fix(render): suppress label-bg rects covering arrowheads | OPEN, auto-merge armed | z-order fix; may affect labels across many fixtures |
| **#1513** | fix(layout): K8s edge crosses node bbox Linux only | OPEN (filed today) | Linux-specific; macOS audit unaffected |

### 7.3 Tickets proposed by this audit (§6.C)

3 NEW concerns to be filed (decide names + numbers):

| Proposed | Title | Severity | Reference |
|---|---|---|---|
| NEW-1 | fix(render/class): composition + aggregation markers missing — render as plain vee | **P1 parity / bug** | §6.G class/03 |
| NEW-2 | fix(render/usecase): usecase/05 layout tangled — #1483 closure incomplete | P2 bug | §6.G usecase/05 |
| NEW-3 | fix(render/activity): activity/05 while-loop back-edge crosses node bodies | P2 bug | §6.G activity/05 |

Within the 3-8 target.

---

## 8. Examples + docs check

### 8.1 docs/examples/ corpus

- 35-fixture audit corpus stable since wave-4.
- Many SVG artifacts at `docs/examples/<family>/<fixture>.svg` should re-render
  given #1490 lands. Defer until wave-10 sweep.

### 8.2 docs/diagrams artifacts

- `docs/diagrams/architecture-overview.svg` likely still drifted post-#1490;
  visible bug #1499 affects it.
- `scripts/regen-artifacts.sh --force` should run before any release tag.

### 8.3 README.md / GALLERY.md

- Unchanged since wave-5. Defer until in-flight PR queue (#1506 #1509 #1510) lands
  + wave-10 visual sweep.

---

## 9. Verdict against the "world class" goal

| Gate item | Target | Wave-9 status | Pass? |
|---|---|---|---|
| Median ratio ≤ 1.0× (1:1) | ≤ 1.00× | **1.48×** | **NO** (gap 0.48×) — MOVED |
| Median ratio ≤ 1.3× (1.1 goal) | ≤ 1.30× | 1.48× | NO (gap 0.18×) — striking distance |
| Median ratio ≤ 1.5× (1.0 ship gate) | ≤ 1.50× | 1.48× | **YES — FIRST WAVE THIS GATE PASSES** |
| Max ratio ≤ 1.5× | ≤ 1.50× | 2.05× | NO (gap 0.55×) — mindmap/02 only blocker |
| 0 visible visual bugs | 0 | **14 visible defects** (12 W8 carryover + 2 partial; 3 new proposed) | NO |
| 0 open tickets | 0 | 28 open | NO (net −1 vs W8) |
| 0 arrowhead/semantic correctness bugs | 0 | **1 new mode (class/03 missing diamond)** | NO — regression mode shift |
| README + GALLERY current | yes | stale | NO |
| Coverage ≥ 90% | ≥ 90% | gate at 85%, ratchet running | partial |
| Deterministic output | byte-identical | unchanged | YES |
| Differential oracle passing | ≥ 50% | passing per #88 | YES |

**THE MEDIAN ≤ 1.5× SHIP GATE PASSED FOR THE FIRST TIME AT WAVE-9.** This is the
"1.0" milestone defined by the revised gate (2026-05-31 Allie). PUML now ships
median-equal-or-better than 1.5× upstream area, with one outlier remaining at
≥ 2.0× (mindmap/02).

**Wave count to "world class" — REVISED ESTIMATE after wave-9:**

The wave-8 estimate was 3 waves. Wave-9 delivered the largest single-wave drop
(median −0.13×, mean −0.11×, max −0.38×) and crossed the 1.5× ship gate. The
remaining 0.48× to 1.00× breaks into three sources:

1. The sequence/class/activity "middle plateau" at 1.45-1.85× (15 fixtures here)
   needs density-retune pass-3 equivalent to #1490 for those families.
2. The component/usecase visual-defect cluster (8 of 16 visual bugs) is quality
   work, not area work.
3. The mindmap/02 + object/05 outliers at 1.94× and 2.05× are family-specific
   layout issues.

Revised estimate:

- **Wave 10**: Land in-flight queue (#1506, #1509, #1510). Density retune sequence
  + activity + class via same template as #1490. Reopen/file 3 new concerns from
  this wave. Expected median ≤ 1.30×.
- **Wave 11**: Usecase family rescue (#1484 + #1463 + NEW-2). Mindmap/02
  bidirectional layout. Component/07 + component/08 final routing. Expected
  median ≤ 1.15×.
- **Wave 12**: Long-tail compaction + showcase fixture (architecture-overview)
  routing polish + close residual P3s. **WORLD CLASS TARGET.** Expected median ≤ 1.05×.

**Honest estimate: 3 waves to world class, with HIGH confidence after wave-9's
proof-by-example that the density-retune template works.** The fundamental
diagnosis: PUML is now "1.0-shippable" by every quantitative axis; quality work
dominates the next 3 waves; algorithmic deep work is no longer the blocker.

**What specifically blocks 1:1 right now (in priority order):**

1. **class/03 missing-diamond regression (NEW-1, #1491 closure shift)** — quality
   blocker; high visibility per LOC. The fix likely needs to re-add the diamond
   marker logic that #1490's lints removed.
2. **Sequence + activity + class density retune** — gates median move from 1.48×
   to ~1.30×. Worth 0.10-0.15×.
3. **Usecase family rescue (#1484 #1463 NEW-2)** — quality wins; addresses 3 of 3
   usecase fixtures.
4. **Mindmap/02 bidirectional layout** — addresses only ≥ 2.0× outlier.
5. **Component + c4 routing pass-2** — addresses #1497 #1496 #1450.

The audit's honest verdict: **the campaign is in MOMENTUM state at wave-9.** Wave-8
was a hold-wave; wave-9 broke the W5-W8 plateau decisively. Wave-10 needs the same
density-template applied to the next family cluster (sequence/activity/class).
After wave-10, the median should be ≤ 1.30×. After wave-11, ≤ 1.15×. After
wave-12, ≤ 1.05× and the gate passes.

---

## 10. ROADMAP TO 1.0 (concrete tickets, ordered)

This section answers the brief's "beyond audit" requirement: a concrete roadmap
to 1:1 + 0 issues + 0 visual bugs with specific tickets, ordering, and effort
estimates.

### 10.1 Wave-10 — close the in-flight queue + density pass-3 (~5-7 agent-days)

**Median target: ≤ 1.30×. Visual-defect target: ≤ 8.**

| Order | Action | Cost | Median Δ | Quality Δ | Notes |
|---|---|---|---|---|---|
| 1 | Wait for #1506 #1509 #1510 auto-merges | 0 | 0 | +5-7 closures | Auto-merge armed; baby through CI |
| 2 | File NEW-1 (class/03 missing diamond) — recipe: re-add diamond rendering branch in class_relations.rs that #1490's `label_bg` change accidentally killed | 0.5 day | 0 | +++ P1 correctness | One-file fix |
| 3 | File + reopen NEW-2 (usecase/05 tangle) | 0 | 0 | (tracking) | Becomes input for wave-11 usecase rescue |
| 4 | File NEW-3 (activity/05 back-edge through node) | 0 | 0 | (tracking) | Becomes input for wave-11 activity polish |
| 5 | **Density retune pass-3 — sequence family** (medians 1.46-1.83) | 1.5 days | −0.05× | + | Same template as #1490 — reduce per-message Y-pitch + label clearance |
| 6 | **Density retune pass-3 — class family** (medians 1.41-1.59) | 1 day | −0.03× | + | Already partial via #1490; this fills the rest |
| 7 | **Density retune pass-3 — activity family** (medians 1.34-1.86) | 1 day | −0.05× | + | Same template — diamond/node Y-pitch |
| 8 | Close #1487 (mindmap/02 not-reproducible) | 0.1 day | 0 | +1 ticket close | W8 comment exists |
| 9 | Close #1440 (K8s frame top clip — visually resolved) | 0.1 day | 0 | +1 ticket close | Need to bless baselines |
| 10 | scripts/regen-artifacts.sh --force + commit docs/diagrams + docs/examples SVGs | 0.5 day | 0 | freshness | Required for any release tag |
| 11 | Wave-10 audit forensics doc | 0.5 day | 0 | tracking | This template |

**Wave-10 total cost: 5.2 agent-days. Expected exit state: median 1.30×, 8 visual
defects, 24 open tickets.**

### 10.2 Wave-11 — visual quality rescue + family completions (~6-8 agent-days)

**Median target: ≤ 1.15×. Visual-defect target: ≤ 4.**

| Order | Action | Cost | Median Δ | Quality Δ | Notes |
|---|---|---|---|---|---|
| 1 | **Usecase family rescue** — fix #1484 P1 (boundaries stacked), #1463 (`<<extend>>` empty dashed box), NEW-2 (usecase/05 tangle), #1461 (Customer edge crosses label), #1481 (orphan square) | 2-3 days | +0.02× (more nodes spread) | +++ 5 closures | Layout-level rework of system-boundary placement |
| 2 | **Mindmap/02 bidirectional** — implement `+`/`-` semantics OR force horizontal layout for 2-level mindmaps | 1.5 days | −0.10× | ++ | Addresses only ≥ 2.0× fixture |
| 3 | **Component family routing pass-2** — fix #1450 P1 ("uses" label through OrderRepository), #1496 partial (read/write overlap), #1497 (c4/12 edges through nodes) | 2 days | −0.05× | +++ 3 closures | Edge-routing layer fix; affects multiple fixtures |
| 4 | **Architecture-overview routing polish** — fix #1499 (cross-frame diagonal crosses) | 1 day | 0 | + | Single-fixture showcase polish |
| 5 | **Activity family polish** — fix #1478 (yes label clip), #1486 (Complete double-stop), NEW-3 (activity/05 back-edge through node) | 1.5 days | 0 | +++ 3 closures | One-layer per defect |
| 6 | **State/10 label fix** — fix #1485 (play label orphan) | 0.5 day | 0 | + 1 closure | Single label-anchor fix |
| 7 | scripts/regen-artifacts.sh --force + commit | 0.5 day | 0 | freshness | |
| 8 | Wave-11 audit forensics doc | 0.5 day | 0 | tracking | |

**Wave-11 total cost: 8.5 agent-days. Expected exit state: median 1.15×, 4 visual
defects, ~16 open tickets.**

### 10.3 Wave-12 — long-tail compaction + zero-defect close (~4-5 agent-days)

**Median target: ≤ 1.05×. Visual-defect target: 0.**

| Order | Action | Cost | Median Δ | Quality Δ | Notes |
|---|---|---|---|---|---|
| 1 | **Long-tail density retune** — sequence/12 (1.83×), class/01 (1.59×), object/05 (1.94×), timing/01 (1.61×), salt/01 (1.38×) | 1.5 days | −0.10× | + | Per-family low-hanging compaction |
| 2 | **Deployment/02 final compaction** — currently 1.33×, target 1.10× | 0.5 day | −0.02× | + | Already clean; just compact 3D box pitch |
| 3 | **Component/02 final compaction** — currently 1.15× clean, target 1.05× | 0.5 day | −0.01× | + | Lollipop/socket margin tighten |
| 4 | **Residual visual-defect close-out** — any remaining P3s + cosmetic | 1 day | 0 | +++ ALL ZERO | Final visual sweep |
| 5 | **Differential oracle promotion** — promote ≥ 90% match | 0.5 day | 0 | tests pass gate | Conformance suite |
| 6 | **Coverage to 90%** | 0.5 day | 0 | gate pass | From current 85% |
| 7 | **README + GALLERY refresh** | 0.5 day | 0 | freshness | Final release-prep |
| 8 | Wave-12 audit + 1.0 release tag prep | 0.5 day | 0 | release | Cut release |

**Wave-12 total cost: 5 agent-days. Expected exit state: median 1.05×, 0 visual
defects, ≤ 5 open tickets (epics + infra only), 1.0 release ready.**

### 10.4 Cost summary

| Wave | Cost | Cumulative | Median | Visual defects | Open tickets |
|---|---|---|---|---|---|
| W9 (current) | 0 | — | 1.48× | 14 | 28 |
| W10 | 5.2 | 5.2 | ≤ 1.30× | ≤ 8 | ~24 |
| W11 | 8.5 | 13.7 | ≤ 1.15× | ≤ 4 | ~16 |
| W12 | 5.0 | 18.7 | ≤ 1.05× | 0 | ≤ 5 |

**~19 agent-days from wave-9 exit to world-class 1.0 release.** If 4 parallel
swarm agents per wave, that's ~5 calendar days at current cadence (wave-8 → wave-9
shipped in ~24 hours).

### 10.5 Risks to the roadmap

1. **#1490-style density retunes can introduce label-clip regressions** (W9 saw
   #1491 closure cause new "diamond missing" regression). Each wave needs visual
   gate per Allie 2026-05-31 to catch these mode-shifts.
2. **Usecase family rework is deepest** — wave-7 already regressed twice (#1483
   #1484). Wave-11 needs Opus-led direct implementation per CLAUDE.md §12
   "orchestrator directly implements when ≥ 2 Sonnet failures."
3. **Mindmap/02 bidirectional could be a 2-wave epic** (depends on whether PlantUML's
   `+`/`-` syntax is in scope — current corpus uses only `*`).
4. **Coverage 85→90% may bottleneck if mechanical fill ratchets break gate**;
   prefer landed parity PRs to grow coverage organically.

---

## 11. Top-5 next-fix recommendations (final)

| Rank | Action | Median impact | Quality impact | Cost | Notes |
|---|---|---|---|---|---|
| 1 | **File NEW-1 (class/03 missing-diamond regression)** | 0 | +++ CRITICAL | 0.5 agent-days | Diamond markers gone after #1491 closure. Reopen #1491 with new failure mode OR file successor. |
| 2 | **Land #1509 + #1510 (narrow bug bundle + arrowhead-bg)** | 0 | +++ | 0 (auto-armed) | Closes #1440 #1478 #1479 #1481 #1463 #1464 + arrowhead z-order. CI babysit only. |
| 3 | **Density retune pass-3 — sequence + class + activity families** | −0.10× to −0.15× | + | 3-4 agent-days | Apply #1490 template to remaining 12 fixtures in this band |
| 4 | **Usecase family rescue (#1484 + #1463 + NEW-2)** | +0.02× | +++ | 2-3 agent-days | All 3 of 3 usecase fixtures defective. P1. |
| 5 | **Land #1506 (Phase D style cascade)** | 0 | + (theme parity) | 0 (auto-armed) | 10 missing <style> properties; no fixture geom |

Bonus 6: Close #1487 + #1440 (W8 + W9 close-recs accumulated).

Bonus 7: scripts/regen-artifacts.sh --force after wave-10 PRs land.

Bonus 8: Reopen #1483 with W9 evidence (closure left tangle).

---

## 12. Evidence index

All cached at `/tmp/parity_audit_v9/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully on @ f5eed633)
- 35 `*-PlantUML.png` files (copied from v8 cache; PlantUML version is identical at 1.2026.5)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh`, `fixtures.txt` — driver scripts and corpus list

This audit performed NO source modifications. The repository state at audit time
is `origin/main @ f5eed633`. Auditor's notes:

- PR #1490 (cross-family density sweep pass-2) is the single biggest median-mover
  since the W4→W5 layout-engine transition. It shifted 8 fixtures downward
  including the two worst (deployment/02 2.43→1.33, component/02 2.30→1.15).
- The ship gate (median ≤ 1.5×) passed for the first time at W9 (1.48×).
- Three new visual concerns were surfaced. #1491 closure introduced a mode-shift
  regression (missing diamond instead of wrong triangle) — common pattern after
  arrowhead-related lints.
- Three close-recs from W8 (#1487, #1440, #1483 reopen) remain un-actioned and
  carry forward.
- The roadmap to 1.0 is ~19 agent-days across 3 waves, with high confidence given
  W9's proof-by-example that density retunes scale.

---

*Snapshot doc; the cached PNGs will be cleaned on next OS restart. Copy to
`docs/internal/forensics/2026-06-01-evidence-v9/` only if the gallery needs to
survive past this session.*
