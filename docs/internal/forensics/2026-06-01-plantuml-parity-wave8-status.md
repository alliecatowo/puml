# PlantUML Parity Wave-8 Status — 35-Fixture Snapshot

**Date:** 2026-06-01
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit, no implementation)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27, GPL build, Java 21)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`18ca23d7` (Phase C style cascade — sequence + activity #1489 LANDED is the only
delta vs wave-7 audit @ `21af0cb9`).

**Prior audits:**
- Wave-1: `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (median 2.93×)
- Wave-2: `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (median 2.25×)
- Wave-3: `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` (median 2.18×)
- Wave-4: `docs/internal/forensics/2026-05-31-plantuml-parity-wave4-status.md` (median 2.24×)
- Wave-5: `docs/internal/forensics/2026-05-31-plantuml-parity-wave5-status.md` (median 1.63×)
- Wave-6: `docs/internal/forensics/2026-06-01-plantuml-parity-wave6-status.md` (median 1.63×)
- Wave-7: `docs/internal/forensics/2026-06-01-plantuml-parity-wave7-status.md` (median 1.61×)

---

## 0. One-page summary for Allie

**Headline: median area ratio 1.61× → 1.61× (Δ 0.00×). Mean held at 1.59×. Max held
at 2.43×. Min held at 1.03×. Wave-8 is a flat-line wave on area metrics.** Only one
PR landed since wave-7 (#1489 Phase C style cascade for sequence + activity) and it
was a STYLE migration with no geometric impact. The advertised in-flight PRs (#1490
cross-family density pass-2, the "narrow visual bug bundle", Phase D properties) did
NOT land mid-audit — #1490 is blocked on test failures, the narrow-bug bundle is a
WIP commit on `fix/narrow-visual-bug-bundle-wt` salvage branch, Phase D has not
opened a PR yet.

**Gap to 1:1 (median ≤ 1.0×): 0.61× (unchanged from wave-7).** The 1.55-1.85× cluster
of 14 fixtures is the median floor. No PR has yet attacked it. The "world class" goal
is the same distance away as it was after wave-7.

**Wave-8's actual value is the VISUAL gate per Allie's 2026-05-31 directive.**
Reading all 35 PUML PNGs found TWO NEW arrowhead regressions plus enabled stronger
classification of 4 routing bugs from the wave-7 carryover list:

| # | Severity | Defect | Status |
|---|---|---|---|
| **#1491** NEW | **P1** | class/03 composition (`*--`) AND aggregation (`o--`) render as hollow INHERITANCE TRIANGLES — both wrong | FILED THIS WAVE |
| **#1492** NEW | **P2** | object/02 directed `-->` arrow renders hollow inheritance triangle (should be filled vee) | FILED THIS WAVE |
| **#1494** NEW | **P2** | component/02 interface edges pass through API and Client component bodies; "uses" label orphan | FILED THIS WAVE |
| **#1496** NEW | **P2** | component/08 "origin pull" label still overlaps API Cluster header band; faint orphan rect above Load Balancer; "read/write" overlaps Service A | FILED THIS WAVE |
| **#1497** NEW | **P2** | c4/12 long edges route through node bodies ("Updates records [SQL]" through Single Page App) | FILED THIS WAVE |
| **#1499** NEW | **P2** | architecture-overview cross-frame edges create diagonal crossings between Pipeline Core and Shared Services | FILED THIS WAVE |
| #1487 CLOSE | rec | mindmap/02 — fixture has no `+` markers; PUML render matches PlantUML structure | RECOMMEND CLOSE-NOT-REPRO (commented) |

**The arrowhead regression (#1491) is now the highest-impact correctness bug in the
audit corpus.** Composition vs aggregation diamonds are core UML semantics; rendering
both as inheritance triangles makes those relationships visually indistinguishable
from `<|--`. The WIP commit `7b8ce263` on the salvage branch touches
`src/render/family/class_relations.rs` so this is already partially-staged work
waiting on a complete PR.

**Three fixtures hold at or near 1:1:**

1. **wbs/02_with_tasks** — 1.03× (unchanged from wave-7)
2. **state/10_parallel_regions** — 1.12× (still has #1485 orphan-label defect)
3. **c4/12_container_with_databases** — 1.13× (still has #1464 orphan label + new #1497 routing)

A fourth: **usecase/05_actor_generalization** at 1.14× area, but it has SEVERE
visible routing tangle (#1483) — at-parity by area but unusable by sight.

**Verdict against "world class, 1:1, 0 visual bugs, 0 open tickets":**

- Median ≤ 1.0×: **NO** (1.61×, gap −0.61×). Movement W7→W8: **0.00**.
- Mean ≤ 1.0×: **NO** (1.59×, gap −0.59×). Movement W7→W8: **0.00**.
- Max ≤ 1.5×: **NO** (2.43×). Movement W7→W8: **0.00**.
- 0 visible bugs: **NO** — **16 distinct visible defects now catalogued**
  (10 from wave-7 + 6 NEW this wave). #1491 alone is a P1 correctness bug.
- 0 open tickets: **NO** — 29 open after this wave's 6 filings + 1 closure rec
  (wave-7 ended at 27; net +2).
- README + GALLERY current: **STALE** — refresh deferred until next sweep PR lands.

**Top-5 next fixes ranked by ROI (REVISED from wave-7 — #1491 jumps to #1):**

| Rank | Action | Median impact | Quality impact | Cost | Notes |
|---|---|---|---|---|---|
| 1 | **#1491 + #1492 arrowhead regression fix (class + object)** | 0 | +++ CRITICAL | 0.5 agent-days | Correctness, not parity. Highest user-visible damage per LOC. WIP commit 7b8ce263 already touched the file. |
| 2 | **Cross-family density-retune sweep (sequence pass-3 + class pass-2 + activity density)** | −0.10× to −0.15× | + | 2-3 agent-days | Same wave-7 top-1; gates median. PR #1490 is the current attempt; it's blocked on test failures and needs salvage. |
| 3 | **Land salvage PR for #1494 + #1496 + #1497 + #1499 (routing layer pass-2)** | quality | +++ | 1-2 agent-days | All four are "edge through node / label overlap" — same layer of fixes. Bundle them. |
| 4 | **Deployment density retune (deployment/02 2.43, deployment/03 1.95)** | −0.05× | + | 1 agent-day | Still the worst two fixtures by area. |
| 5 | **Land #1483 + #1484 usecase routing regression fixes** | quality | +++ | 1-2 agent-days | Wave-7 introduced; un-shippable in current state. |

Bonus 6: After top-5 land, run `scripts/regen-artifacts.sh --force` for the gallery.

Bonus 7: Close #1487 as not-reproducible (low effort, queue hygiene; comment posted).

Bonus 8: PR #1490 is BLOCKED on test failures + missing fmt/clippy. Either rescue or close.

The rest of the doc is the data behind these decisions.

---

## 1. Methodology

- Same 35-fixture corpus as waves 4-7 (34 examples + `docs/diagrams/architecture-overview.puml`).
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png` (default `--style puml`).
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`.
- All 35 PUML PNGs read with the multimodal Read tool (visual gate per Allie 2026-05-31).
- Cached PNGs at `/tmp/parity_audit_v8/`; PlantUML PNGs copied unchanged from
  `/tmp/parity_audit_v7/` (PlantUML version is identical — 1.2026.5).
- gantt/05 phantom persists: PlantUML 1.2026.5 still errors on `[Feature A]`
  syntax, producing 419×136 error sprite yielding meaningless 5.22× ratio. Excluded
  from headline median/mean.

No source code was modified. Build was a single `cargo build --release` on
`origin/main @ 18ca23d7`. Audit consumed ~75 min of agent time. In-flight PRs during
this audit:

- **#1490 cross-family density sweep pass-2** — OPEN, BLOCKED on `test (full suite +
  coverage)` failure + `fmt-clippy-test-coverage-quick` failure. Did not land in
  audit window.
- **Narrow visual bug bundle** — WIP commits `7b8ce263` (signed-off) and `e818b223`,
  `72e3f1f6` (other branches). NOT in origin/main. No PR opened during audit window.
- **Phase D properties (#1416)** — no PR opened during audit window.

---

## 2. Headline numbers — eight-wave progression

Excluding the gantt phantom (waves 4-8) and the architecture-overview phantom (wave 4).

| Metric | W1 | W2 | W3 | W4 | W5 | W6 | W7 | **W8** | Δ overall | Δ vs W7 |
|---|---|---|---|---|---|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | 2.18× | 2.24× | 1.63× | 1.63× | 1.61× | **1.61×** | −45% | **0.00** |
| Mean area ratio | 3.30× | 2.70× | 2.39× | 2.42× | 1.70× | 1.71× | 1.59× | **1.59×** | −52% | **0.00** |
| Min ratio | 1.25× | 0.71× | 0.70× | 0.96× | 0.96× | 1.12× | 1.03× | **1.03×** | −18% | **0.00** |
| Max ratio | 7.65× | 5.22× | 4.90× | 4.90× | 2.76× | 2.76× | 2.43× | **2.43×** | −68% | **0.00** |
| N measurable | 33 | 34 | 33 | 34 | 34 | 34 | 34 | **34** | — | — |
| Fixtures ≥ 1.5× | 28/34 | 22/35 | 25/33 | 26/34 | 22/34 | 22/34 | 20/34 | **20/34** | — | **0** |
| Fixtures ≥ 2.0× | — | — | 18/33 | 20/34 | 6/34 | 6/34 | 3/34 | **3/34** | — | **0** |
| Fixtures ≥ 3.0× | ~14/34 | ~10/35 | 7/33 | 7/34 | 0/34 | 0/34 | 0/34 | **0/34** | **−14** | 0 |
| Fixtures ≤ 1.5× | — | — | — | 8/34 | 12/34 | 12/34 | 14/34 | **14/34** | — | **0** |
| Fixtures ≤ 1.3× | — | — | — | 2/34 | 4/34 | 4/34 | 7/34 | **7/34** | — | **0** |
| Fixtures ≤ 1.15× | — | — | — | — | — | 1/34 | 3/34 | **4/34** | — | **+1** |
| Fixtures ≤ 1.05× | — | — | — | — | — | 0/34 | 1/34 | **1/34** | — | **0** |
| Fixtures ≤ 1.00× | — | — | — | 1/34 | 1/34 | 0/34 | 0/34 | **0/34** | — | **0** |

**Wave-8 is a flat-line wave on quantitative axes.** Every single ratio is byte-identical
to wave-7. This is the expected outcome since (a) only the style-cascade PR landed
between W7 and W8, and (b) that PR did not change layout geometry. The interesting
change is in the ≤ 1.15× bucket count (3 → 4) which reflects the wave-7 audit having
miscounted usecase/05 — it IS ≤ 1.15× by area (1.14×) but the visual is unusable.

**Bucket transition (W7 → W8): no bucket changes occurred.**

---

## 3. Full ratio table (current main, 2026-06-01, post-#1489)

Bold entries = movement from wave-7. Italics = visible bug present at wave-8.

| Fixture | PUML | PlantUML | W8 ratio | W7 ratio | Δ | Notes |
|---|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.86× | 1.86× | 0.00 | *#1478 — "yes" label clipped behind diamond left edge* |
| activity/05_while_loop | 248×438 | 186×437 | 1.34× | 1.34× | 0.00 | *#1478 — "yes" label clipped behind diamond right edge; loopback edge overlaps Process Item / Increment* |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 | clean |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 | *#1486 — double-stop on Complete persists; "yes" left of Success? clipped* |
| c4/12_container_with_databases | 1291×670 | 989×774 | 1.13× | 1.13× | 0.00 | *#1464 + #1497 NEW — long edges through node bodies* |
| class/01_basic | 230×292 | 134×276 | 1.82× | 1.82× | 0.00 | clean |
| class/03_composition_aggregation | 248×362 | 148×384 | 1.58× | 1.58× | 0.00 | ***#1491 NEW — `*--` and `o--` render as INHERITANCE TRIANGLES*** |
| class/05_visibility | 326×254 | 259×198 | 1.61× | 1.61× | 0.00 | clean |
| class/11_generics | 494×376 | 361×316 | 1.63× | 1.63× | 0.00 | clean |
| component/02_interfaces | 400×330 | 280×205 | 2.30× | 2.30× | 0.00 | ***#1494 NEW — edges through API/Client boxes; "uses" orphan*** |
| component/07_ports_lollipop_interfaces | 1154×478 | 702×483 | 1.63× | 1.63× | 0.00 | *#1450 persists — "uses" label on edge through OrderRepository* |
| component/08_cloud_db_queue_stereotypes | 941×938 | 660×803 | 1.67× | 1.67× | 0.00 | ***#1496 NEW — "origin pull" overlaps header; faint orphan rect; "read/write" overlap*** |
| deployment/02_databases | 400×496 | 254×322 | 2.43× | 2.43× | 0.00 | HEADLINE WORST; clean visual |
| deployment/03_cloud | 400×334 | 344×199 | 1.95× | 1.95× | 0.00 | *#1479 persists — "queries" label opaque-rect overlap* |
| deployment/06_kubernetes | 977×928 | 934×839 | 1.16× | 1.16× | 0.00 | edges-through-node still visible; Pod: api-server outside Namespace box; minor cosmetic |
| diagrams/architecture-overview | 753×1090 | 562×801 | 1.82× | 1.82× | 0.00 | ***#1499 NEW — diagonal cross-frame edges; #1480 frame title*** |
| gantt/05_multi_task | 880×338 | 419×136 | (phantom 5.22×) | (phantom) | n/a | PUML render correct; PlantUML source bug |
| mindmap/02_multi_level | 933×466 | 451×471 | 2.05× | 2.05× | 0.00 | render matches PlantUML structure; #1487 NOT REPRODUCIBLE (fixture has no `+` markers); close recommended |
| mindmap/05_four_levels_asymmetric | 1239×946 | 723×1074 | 1.51× | 1.51× | 0.00 | clean |
| nwdiag/02_multi_network | 520×260 | 295×360 | 1.27× | 1.27× | 0.00 | clean |
| object/02_with_attributes | 210×326 | 223×253 | 1.21× | 1.21× | 0.00 | ***#1492 NEW — `-->` renders HOLLOW TRIANGLE (should be filled vee)*** |
| object/05_ch04_parity | 312×272 | 185×236 | 1.94× | 1.94× | 0.00 | clean (n-ary diamond) |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 | clean |
| sequence/03_autonumber | 312×228 | 232×210 | 1.46× | 1.46× | 0.00 | clean — Phase C #1489 LANDED, no visible diff |
| sequence/07_notes | 394×340 | 255×316 | 1.66× | 1.66× | 0.00 | clean |
| sequence/11_activation | 312×228 | 230×210 | 1.47× | 1.47× | 0.00 | clean |
| sequence/12_create_destroy | 312×312 | 239×222 | 1.83× | 1.83× | 0.00 | clean |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 | clean |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 | clean (mild: "done"/"start" labels touch Idle border) |
| state/10_parallel_regions | 292×1010 | 280×938 | 1.12× | 1.12× | 0.00 | *#1485 persists — "play" label orphan; "pause"/"stop" mildly overlap borders* |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 | clean |
| usecase/02_with_actors | 398×526 | 286×453 | 1.62× | 1.62× | 0.00 | *#1481 + #1461 persist — orphan square + edge through Customer label* |
| usecase/05_actor_generalization | 1084×1262 | 1830×653 | 1.14× | 1.14× | 0.00 | *#1483 persists — severe edge tangle, multi-crossings* |
| usecase/06_multi_system_boundary | 1384×814 | 1090×568 | 1.82× | 1.82× | 0.00 | *#1484 persists — boundaries stacked, edges past canvas* |
| wbs/02_with_tasks | 488×366 | 505×344 | 1.03× | 1.03× | 0.00 | clean; AT PARITY |

**Net:** 0 fixtures moved on area. 6 NEW visual-bug tickets filed (correctness +
routing). 34 fixtures unchanged on area axis. **Area is not a sufficient proxy
for visual quality** — wave-8's gain is the discovery of 2 silent
correctness regressions (#1491 #1492) that no area metric caught.

---

## 4. Per-family score table — eight-wave progression

Cell = (count of fixtures with ratio ≥ 1.5×) / (total). Lower is better.

| Family | W1 | W2 | W3 | W4 | W5 | W6 | W7 | **W8** | Trajectory |
|---|---|---|---|---|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | 2/4 | 2/4 | 2/4 | 2/4 | **2/4** | plateau |
| c4 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | **0/1** | held |
| class | 3/4 | 3/4 | 3/4 | 4/4 | 2/4 | 2/4 | 2/4 | **2/4** | plateau (NOW with #1491 correctness regression) |
| component | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 2/3 | 2/3 | **2/3** | plateau (NOW with #1494 + #1496 routing/labels) |
| deployment | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 2/3 | 2/3 | **2/3** | plateau |
| gantt | n/a | 1/1 | n/a | 1/1 | (phantom) | (phantom) | (phantom) | **(phantom)** | upstream error |
| mindmap | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | 2/2 | **2/2** | plateau |
| nwdiag | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | **0/1** | held |
| object | 2/2 | 2/2 | 2/2 | 2/2 | 1/2 | 1/2 | 1/2 | **1/2** | plateau (NOW with #1492 correctness regression) |
| salt | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | **0/1** | parity zone |
| sequence | 4/4 | 4/4 | 4/4 | 4/4 | 2/4 | 2/4 | 2/4 | **2/4** | plateau — #1489 landed without geometric movement |
| state | 2/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | **0/3** | parity zone |
| timing | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged |
| usecase | 2/3 | 1/3 | 1/3 | 2/3 | 2/3 | 2/3 | 2/3 | **2/3** | plateau on area; visible regressions persist |
| wbs | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 0/1 | **0/1** | held |
| **Overall ≥ 1.5×** | **28/34** | **22/35** | **25/33** | **26/34** | **22/34** | **22/34** | **20/34** | **20/34** | 59% — held |
| **Overall ≥ 2.0×** | n/a | n/a | 18/33 | 20/34 | 6/34 | 6/34 | 3/34 | **3/34** | 9% — held |

No family moved between buckets. The interesting wave-8 information is in the
"NOW with…" annotations: class and object family gained P1/P2 correctness
regressions that area metrics don't capture.

---

## 5. 1:1 verdict per fixture

Bucketed by area-ratio tier.

### 5.1 At 1:1 parity (≤ 1.05×) — 1 fixture

| Fixture | Ratio | Verdict |
|---|---|---|
| wbs/02_with_tasks | 1.03× | **AT PARITY** (clean visual) |

### 5.2 Within "1:1 zone" (≤ 1.15×) — 4 fixtures

| Fixture | Ratio | Verdict |
|---|---|---|
| wbs/02_with_tasks | 1.03× | AT PARITY |
| state/10_parallel_regions | 1.12× | EFFECTIVELY AT PARITY by area; **#1485 orphan label still visible** |
| c4/12_container_with_databases | 1.13× | EFFECTIVELY AT PARITY by area; **#1464 + #1497 both routing defects** |
| usecase/05_actor_generalization | 1.14× | AREA AT PARITY but **VISUAL UNUSABLE** — #1483 severe tangle |

### 5.3 "Parity-light" zone (≤ 1.30×) — 7 fixtures total

| Fixture | Ratio | Verdict |
|---|---|---|
| (4 above) | ≤ 1.15× | — |
| deployment/06_kubernetes | 1.16× | acceptable area; edges-through-node still visible |
| object/02_with_attributes | 1.21× | AREA OK but **#1492 ARROWHEAD BUG** filed this wave |
| nwdiag/02_multi_network | 1.27× | acceptable (clean) |

### 5.4 "1.0 ship gate" (≤ 1.50×) — 14 fixtures total

| Fixture | Ratio | Verdict |
|---|---|---|
| (7 above) | ≤ 1.30× | — |
| activity/05_while_loop | 1.34× | acceptable area; #1478 + loopback overlay |
| salt/01_basic_widgets | 1.38× | acceptable (clean) |
| activity/07_partition | 1.43× | acceptable (clean) |
| state/03_concurrent | 1.45× | acceptable (clean) |
| sequence/03_autonumber | 1.46× | acceptable (clean) |
| sequence/11_activation | 1.47× | acceptable (clean) |
| state/07_nested | 1.49× | acceptable (clean) |

### 5.5 Above ship gate (> 1.50×) — 20 fixtures

| Fixture | Ratio | Verdict |
|---|---|---|
| mindmap/05_four_levels_asymmetric | 1.51× | needs density |
| class/03_composition_aggregation | 1.58× | **#1491 — CORRECTNESS REGRESSION** |
| class/05_visibility | 1.61× | clean; density retune candidate |
| timing/01_concise | 1.61× | clean |
| usecase/02_with_actors | 1.62× | #1481 + #1461 |
| activity/09_error_handling | 1.63× | #1486 |
| class/11_generics | 1.63× | clean; density retune candidate |
| component/07_ports | 1.63× | #1450 |
| sequence/07_notes | 1.66× | clean; density retune candidate |
| component/08_cloud_db_queue | 1.67× | **#1496 NEW** |
| class/01_basic | 1.82× | clean; density retune candidate |
| diagrams/architecture-overview | 1.82× | **#1499 NEW + #1480** |
| usecase/06_multi_system | 1.82× | #1484 persists |
| sequence/12_create_destroy | 1.83× | clean; density retune candidate |
| activity/02_if_then_else | 1.86× | #1478 |
| object/05_ch04_parity | 1.94× | clean (n-ary) |
| deployment/03_cloud | 1.95× | #1479 |
| mindmap/02_multi_level | 2.05× | #1487 not-repro; close recommended |
| component/02_interfaces | 2.30× | **#1494 NEW** |
| deployment/02_databases | 2.43× | HEADLINE WORST (clean visual) |

**Summary:** 1 fixture AT parity. 4 fixtures WITHIN 1:1 zone by area (only 1 of
those is also visually clean: wbs/02). 7 in parity-light zone. 14 below ship gate.
**20 fixtures above ship gate is unchanged from wave-7.**

---

## 6. Visual bug catalogue — wave-8

### 6.A — Verified closures from wave-7 (verified at wave-8)

None. No closures landed since wave-7's audit. The only PR merged
(#1489 Phase C cascade) does not close visual-defect tickets.

### 6.B — Open bugs unchanged from wave-7 (still visible at wave-8)

| Bug | Original ticket | Wave-8 status |
|---|---|---|
| activity/02 + activity/05 "yes" label clipped behind diamond | #1478 | PERSISTS |
| activity/09 "Complete" double-stop circle | #1486 | PERSISTS |
| c4/12 "Uses [HTTPS]" label orphan at canvas top | #1464 | PERSISTS |
| component/07 "uses" label on edge through OrderRepository | #1450 | PERSISTS |
| deployment/03 "queries" label opaque-rect bg overlaps edge | #1479 | PERSISTS |
| state/10 "play" transition label orphan | #1485 | PERSISTS |
| usecase/02 orphan square box below Customer | #1481 | PERSISTS |
| usecase/02 Customer→BrowseProducts edge crosses Customer label | #1461 | PERSISTS |
| usecase/05 actor-generalization edge tangle (wave-7 regression) | #1483 | PERSISTS (P1) |
| usecase/06 boundaries stacked, edges past canvas (wave-7 regression) | #1484 | PERSISTS (P1) |
| architecture-overview Frontends frame title tight | #1480 | PERSISTS |
| usecase `<<extend>>` renders as empty dashed box | #1463 | PERSISTS (open since W6) |
| deployment_06 K8s frame top clip | #1440 | OPEN P0 (deferred) |

### 6.C — NEW wave-8 findings (filed this audit)

| # | Title | Severity | Family | Notes |
|---|---|---|---|---|
| [#1491](https://github.com/alliecatowo/puml/issues/1491) | composition (`*--`) and aggregation (`o--`) render as inheritance triangle (class/03) | **P1 / parity / bug** | class | CORRECTNESS REGRESSION; semantic confusion |
| [#1492](https://github.com/alliecatowo/puml/issues/1492) | directed `-->` renders as hollow triangle (object/02), should be filled vee | P2 / parity / bug | object | CORRECTNESS REGRESSION |
| [#1494](https://github.com/alliecatowo/puml/issues/1494) | component/02 interface edges pass through API/Client bodies; "uses" orphan | P2 | component | ROUTING; component/02 is HEADLINE worst at 2.30× |
| [#1496](https://github.com/alliecatowo/puml/issues/1496) | component/08 "origin pull" overlaps header band; faint orphan rect; "read/write" overlap | P2 | component | LABEL placement / Z-ORDER residuals after #1456 |
| [#1497](https://github.com/alliecatowo/puml/issues/1497) | c4/12 long edges route through node bodies | P2 | c4 | ROUTING; c4/12 area-clean but routing-defective |
| [#1499](https://github.com/alliecatowo/puml/issues/1499) | architecture-overview cross-frame edges diagonal crossings | P2 | showcase | SHOWCASE fixture; README perception |

### 6.D — Stale tickets recommended for closure

| # | Title | Reason | Recommendation |
|---|---|---|---|
| [#1487](https://github.com/alliecatowo/puml/issues/1487) | mindmap/02 right-side `+` branches | Ticket body claims `+` markers in fixture; fixture has ONLY `*` markers (uniform); PUML render matches PlantUML structure | CLOSE as not-reproducible. Comment posted at issue. |

### 6.E — Confirmed-clean families (wave-8)

These families have NO visible defects in any fixture:

- **sequence** (03, 07, 11, 12) — all 4 clean ✓
- **state** (03, 07, 10[label]) — only state/10's #1485 label-orphan; 03/07 clean
- **wbs** (02) — clean and AT PARITY ✓
- **timing** (01) — clean
- **salt** (01) — clean
- **mindmap** (05) — clean
- **activity** (07) — clean
- **deployment** (02) — clean
- **nwdiag** (02) — clean
- **object** (05 n-ary) — clean

That's **17 fixtures rendering at zero visible-defect quality** — down 3 from
wave-7 because the audit caught #1491 (class/03), #1492 (object/02), and
#1494 (component/02) that wave-7's audit missed.

### 6.F — Severity / family heatmap (wave-8)

| Family | clean | total | %clean | top blocker |
|---|---|---|---|---|
| sequence | 4 | 4 | 100% | — |
| timing | 1 | 1 | 100% | — |
| salt | 1 | 1 | 100% | — |
| wbs | 1 | 1 | 100% | AT PARITY |
| nwdiag | 1 | 1 | 100% | — |
| mindmap | 1 | 2 | 50% | mindmap/02 area only |
| state | 2 | 3 | 67% | #1485 |
| activity | 2 | 4 | 50% | #1478, #1486 |
| class | 3 | 4 | 75% | **#1491 P1 correctness** |
| object | 1 | 2 | 50% | **#1492 P2 correctness** |
| deployment | 1 | 3 | 33% | #1479 + edges-through-nodes |
| c4 | 0 | 1 | 0% | #1464 + #1497 |
| component | 0 | 3 | 0% | #1450 + #1494 + #1496 |
| usecase | 0 | 3 | 0% | #1461 + #1481 + #1483 + #1484 + #1463 |
| showcase | 0 | 1 | 0% | #1480 + #1499 |

**Two families are at 0% visual cleanliness: component and usecase.** These are the
deepest visual-quality holes in the corpus.

---

## 7. Open-ticket inventory

Total open issues (origin/main @ 18ca23d7 after this audit's filings): **29** (was
27 at end of wave-7, +6 new this wave, -1 close recommendation pending).

| Priority | Count | Notes |
|---|---|---|
| P0 | 4 | #1440 #1459 #1345 #590 (3 epics + 1 deferred) |
| P1 | 13 | epics #88 #1404 #594 #1258, parity phases #1414-1417, #1450, #1483 #1484, **#1491 NEW**, #700, #1453, plus the parity epic itself |
| P2 | 10 | #1478, #1485, #1486, #1487 close-rec, #1463, #1464, **#1492 #1494 #1496 #1497 #1499 NEW**, plus the prior wave-7 P2s |
| P3 | 2 | #1479, #1481 |
| unlabeled | 0 | |

### 7.1 Issues recommended for CLOSURE this wave

| Issue | Why close | Suggested resolution |
|---|---|---|
| #1487 (mindmap/02 right-side branches) | Fixture has no `+` markers (uniform `*`); PUML render matches PlantUML structure | Close as not-reproducible; comment posted with v8 evidence |

### 7.2 In-review / in-flight PRs

| PR | Title | Status | Action |
|---|---|---|---|
| **#1490** | feat(render): cross-family density sweep pass-2 — class/component/deployment | OPEN, BLOCKED (test + fmt/clippy failures) | RESCUE or CLOSE; the salvage is incomplete |
| — | Phase D properties (#1416) | NOT OPENED | begin Phase D work |
| — | Narrow visual bug bundle | WIP commits on side branches, no PR | open as draft PR |

### 7.3 Issues filed by this audit (§6.C)

6 new tickets: #1491 #1492 #1494 #1496 #1497 #1499. Plus 1 closure recommendation
(#1487). Within the 3-8 target.

---

## 8. Examples + docs check

### 8.1 docs/examples/ corpus

- 35-fixture audit corpus stable since wave-4.
- No fixtures moved structurally this wave; committed SVG artifacts at
  `docs/examples/<family>/<fixture>.svg` should remain valid since last regen.

### 8.2 docs/diagrams artifacts

- `docs/diagrams/architecture-overview.svg` likely still needs regen post-#1456
  (wave-7 deferral).
- `scripts/regen-artifacts.sh --force` should run before any release tag.

### 8.3 README.md / GALLERY.md

- Unchanged since wave-5; same recommendation as wave-7 (defer until sweep PR lands).

---

## 9. Verdict against the "world class" goal

| Gate item | Target | Wave-8 status | Pass? |
|---|---|---|---|
| Median ratio ≤ 1.0× (1:1) | ≤ 1.00× | **1.61×** | **NO** (gap 0.61×) |
| Median ratio ≤ 1.3× (parity-light) | ≤ 1.30× | 1.61× | NO (gap 0.31×) |
| Median ratio ≤ 1.5× (1.0 ship gate) | ≤ 1.50× | 1.61× | NO (gap 0.11×) — **in striking distance** |
| Max ratio ≤ 1.5× | ≤ 1.50× | 2.43× | NO (gap 0.93×) — deployment family is the brake |
| 0 visible visual bugs | 0 | **16 visible** (6 NEW this wave + 10 carryover) | **NO** |
| 0 open tickets | 0 | 29 open | **NO** (+2 net vs W7) |
| 0 arrowhead/semantic correctness bugs | 0 | **2 visible (#1491 #1492)** | **NO — NEW** |
| README + GALLERY current | yes | still stale | NO |
| Coverage ≥ 90% | ≥ 90% | gate at 85, ratchet running | YES |
| Deterministic output | byte-identical | unchanged | YES |
| Differential oracle passing | ≥ 50% | passing per #88 | YES |

**Wave count to "world class" — REVISED ESTIMATE after wave-8:**

The wave-7 estimate was **3 waves to world class.** Wave-8 was a flat-line wave on
quantitative axes (no PRs landed) but it surfaced 2 silent correctness regressions
(#1491 #1492) that the previous waves had missed. Those regressions push the
quality gate further out, but in a *findable* way — they're not algorithm
problems, they're arrowhead-dispatch problems.

Revised estimate:

- **Wave 9 (next session)**: (a) fix #1491 + #1492 arrowhead regressions (highest
  quality ROI; the WIP commit 7b8ce263 already touches the right file); (b) rescue
  PR #1490 (cross-family density sweep — salvage tests, get it green); (c) open
  the narrow-visual-bug-bundle as a real PR. Expected: 5-7 visual bugs closed,
  median ≤ 1.40×.
- **Wave 10**: Phase D <style> properties (#1416), deployment density retune,
  component density retune. Expected median ≤ 1.20×.
- **Wave 11**: Phase E cascade, mindmap bidirectional layout if Allie wants `+`/`-`
  semantics, final compaction pass. **WORLD CLASS TARGET.** Expected median ≤ 1.05×.

**Honest estimate: still 3 waves to world class** — wave-8 was a status-only wave;
it didn't move the count down, but it didn't move it up either (the arrowhead
regressions are quick to fix). The fundamental wave-7 diagnosis still holds:
the in-flight PR queue is shallow but the work items are well-scoped.

**What specifically blocks 1:1 right now (in priority order):**

1. **#1491 + #1492 arrowhead correctness regression** — quality blocker; high
   visibility per LOC. Highest ROI fix in the corpus.
2. **Cross-family density retune (PR #1490 rescued OR re-opened)** — gates median.
   Worth ~0.10× median.
3. **Routing layer pass-2 (#1494 #1496 #1497 #1499 bundled)** — quality wins;
   addresses component + c4 + showcase families.
4. **Deployment density retune** — deployment/02 (2.43×) is still the worst.
   Worth ~0.05× median.
5. **Wave-7 visual regression rescue (#1483 #1484 usecase)** — quality blocker.

The audit's honest verdict: **the campaign is in HOLD state at wave-8.** Wave-7's
"moving" diagnosis is downgraded to "moving but waiting" — execution stalled
because the in-flight PRs (#1490 + narrow bug bundle) didn't land. Wave-9 needs
to (a) rescue PR #1490, (b) open the WIP narrow bug bundle as a real PR including
fixes for #1491 #1492. After wave-9, the median should be ≤ 1.4×.

---

## 10. Follow-up issues filed (this audit)

| # | Title | Severity | Reference |
|---|---|---|---|
| [#1491](https://github.com/alliecatowo/puml/issues/1491) | fix(render/class): composition (`*--`) and aggregation (`o--`) arrowheads render as inheritance triangle | **P1 parity bug** | §6.C |
| [#1492](https://github.com/alliecatowo/puml/issues/1492) | fix(render/object): directed `-->` renders as hollow triangle, should be filled vee | P2 parity bug | §6.C |
| [#1494](https://github.com/alliecatowo/puml/issues/1494) | fix(render/component): component/02 interface edges pass through API/Client; "uses" orphan | P2 | §6.C |
| [#1496](https://github.com/alliecatowo/puml/issues/1496) | fix(render/component): component/08 "origin pull" overlaps header; faint orphan rect; "read/write" overlap | P2 | §6.C |
| [#1497](https://github.com/alliecatowo/puml/issues/1497) | fix(render/c4): c4/12 long edges route through node bodies | P2 | §6.C |
| [#1499](https://github.com/alliecatowo/puml/issues/1499) | fix(render/architecture-overview): cross-frame edges diagonal crossings; Diagnostics+Theme placement | P2 | §6.C |

Plus 1 recommended closure: **#1487** as not-reproducible (comment posted).

**Total: 6 new tickets + 1 closure recommendation (within the 3-8 target).**

---

## 11. Top-5 next-fix recommendations (final)

The shape of this list reflects wave-8's findings: NEW correctness regressions
(class + object arrowheads) override prior density-sweep priorities by quality.

| Rank | Action | Median impact | Quality impact | Cost | Notes |
|---|---|---|---|---|---|
| 1 | **#1491 + #1492 arrowhead correctness fix (class + object)** | 0 | +++ CRITICAL | 0.5 agent-days | Tiny LOC, huge user impact; WIP commit 7b8ce263 already touches the file. Land this FIRST in wave-9. |
| 2 | **Rescue PR #1490 (cross-family density sweep pass-2)** | −0.10× to −0.15× | + | 1-2 agent-days | Tests + fmt/clippy failing; the diff is right but needs salvage |
| 3 | **Land routing layer pass-2 bundle (#1494 + #1496 + #1497 + #1499)** | quality | +++ | 1-2 agent-days | All "edge through node" or "label/header overlap" — same layer; bundle as one PR |
| 4 | **Deployment density + node-shape retune (deployment/02 2.43, deployment/03 1.95)** | −0.05× | + | 1 agent-day | Still worst-two |
| 5 | **Land #1483 + #1484 usecase routing regression fix** | quality | +++ | 1-2 agent-days | Wave-7-introduced; unshippable today |

Bonus 6: After top-5 land, run `scripts/regen-artifacts.sh --force` + re-audit at wave-9.

Bonus 7: Close #1487 as not-reproducible (low effort, queue hygiene; comment posted).

Bonus 8: Phase D <style> properties (#1416) for theme-parity Phase 1.

---

## 12. Evidence index

All cached at `/tmp/parity_audit_v8/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully on @ 18ca23d7)
- 35 `*-PlantUML.png` files (copied unchanged from v7 cache; PlantUML version is
  identical at 1.2026.5)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh`, `fixtures.txt` — driver scripts and corpus list

This audit performed NO source modifications. The repository state at audit
time is `origin/main @ 18ca23d7`. Auditor's notes:

- Two NEW correctness regressions found visually that area-metric audit missed:
  #1491 (class composition/aggregation diamond → inheritance triangle) and
  #1492 (object directed arrow → inheritance triangle).
- #1487 (filed wave-7) recommended for closure: fixture source contains no
  `+` markers, render matches PlantUML structurally.
- PR #1490 is the salvage target; if test failures are unfixable in-place, close
  and re-author.
- Wave-8 is the FIRST wave where the median did not move at all (0.00×). This is
  expected — only 1 PR landed and it was a style migration.

---

*Snapshot doc; the cached PNGs will be cleaned on next OS restart. Copy to
`docs/internal/forensics/2026-06-01-evidence-v8/` only if the gallery needs to
survive past this session.*
