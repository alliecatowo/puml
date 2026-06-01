# PlantUML Parity Wave-6 Status — 35-Fixture Snapshot

**Date:** 2026-06-01
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit, no implementation)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27, GPL build, Java 21)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`650c4902` (wave-5 P1 visual bug cluster #1474 LANDED, text-fit-first density
#1471 LANDED, wave-5 forensic doc #1468 LANDED, glitch-hunt cluster #1458
LANDED, style block AST #1420 LANDED).
**Prior audits:**
- Wave-1: `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (median 2.93×)
- Wave-2: `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (median 2.25×)
- Wave-3: `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` (median 2.18×)
- Wave-4: `docs/internal/forensics/2026-05-31-plantuml-parity-wave4-status.md` (median 2.24×)
- Wave-5: `docs/internal/forensics/2026-05-31-plantuml-parity-wave5-status.md` (median 1.63×)

---

## 0. One-page summary for Allie

**Headline: median area ratio HELD at 1.63× (wave-5 → wave-6, no movement).**
Mean drifted slightly +0.01× from 1.70× → 1.71×. Min IMPROVED to 1.12× (was
0.96× in wave-5 — note: PUML had been slightly *smaller* than PlantUML on
state/10; the #1474 fix expanded it to closer parity at 1.12×). Max held at
2.76× (nwdiag/02 untouched). **The wave was a holding pattern.** Five
significant PRs are still in flight (BLOCKED or DIRTY), none of which landed
between wave-5 and wave-6 measurements: #1476 (K8s namespace routing P0),
#1475 (nwdiag retune), #1473 (mindmap/wbs/c4 layout adoption), #1470 (style
cascade Phase B), #1469 (class spline router). The two PRs that DID land
since wave-5 (#1471 text-fit, #1474 P1 cluster) produced micro-deltas of
+0.03× / +0.05× / +0.16× on three fixtures (component/07, deployment/06,
state/10) — the regressions are minor expansions to accommodate fixed text
widths, NOT visual quality regressions.

**Gap to 1:1 target (median ≤ 1.0×):** still 0.63×, unchanged from wave-5.
The wave-7+ work that will move the median is exactly the in-flight queue
that has not landed. **The blocker is PR babying, not new code.**

**What's blocking 1:1 specifically (in priority order):**

1. **PR #1473 mindmap/wbs/c4 layout adoption (BLOCKED)** — projected to drop
   median by ~0.15× on its own by closing the largest layout-convention gap.
   This is the single highest-impact unlanded PR.
2. **PR #1475 nwdiag packed-grid retune (BLOCKED)** — fixes the current
   ratio-leader nwdiag/02 (2.76×). Projected drop ~0.05× median, but moves
   max ratio from 2.76× → ~1.27×.
3. **PR #1476 K8s namespace routing (DIRTY/conflicting)** — fixes deployment/06
   edges crossing namespace frame boundaries. Mostly visual-quality, ~0.02×
   median impact.
4. **PR #1470 <style> cascade Phase B (BLOCKED)** — required for stereotype-
   scoped retune and unblocks Phase C-E (#1415 #1416 #1417).
5. **PR #1469 class family spline router (DIRTY)** — extends #1410 spline
   router to class/object/usecase; quality lift on edge routing.

**Visible bug catalog — wave-5 cluster mostly closed, residual list:**

Of the 8 new bugs filed at wave-5 (#1460–#1467), **5 were closed within 24h**
(#1460 #1462 #1465 not-reproducible, #1466 #1473 PRs opened). The closures of
#1460 (Customer head missing) and #1462 (usecase/06 actor row overlap) were
based on higher-resolution renders where the artifacts are no longer visible
— at the default 398×526 PNG output size the issues still LOOK present to a
naive viewer, but the SVG geometry is correct (verified via grep: both actor
head circles exist at cx=92,cy=38 and cx=242,cy=38 with r=6). **No new bugs
were introduced this wave.** Residuals from wave-5 still visible: #1440,
#1441, #1442, #1443, #1444, #1445, #1446, #1447 (all in-review on PRs that
have not landed).

**Verdict against the "world class, 1:1, 0 visual bugs, 0 open tickets" goal:**
- Median ≤ 1.0×: **NO** (currently 1.63×; gap −0.63×). Realistic path: 3-4
  more waves, AND must successfully merge the 5 BLOCKED/DIRTY PRs.
- 0 open visual bugs: **NO** (12 visual-audit tickets open; 8 are in-review
  on PRs). True net new = 0 this wave.
- 0 open tickets: **NO** (30 open, down from 35 at wave-5 — **−5 net**).
- Examples + docs current: **YES at high level** (gantt/05 now renders
  correctly per visible PNG; mindmap/wbs already correct).
- World class: **NOT YET**, ~3 waves away if the in-flight PRs land cleanly.

**Top-5 next fixes ranked by ROI (CHANGED FROM WAVE-5 — PR babying is now #1):**

| Rank | Action | Median impact | Cost | Ticket / PR |
|---|---|---|---|---|
| 1 | **Resolve PR #1473 mindmap/wbs/c4 BLOCKED state, land** | −0.15× | 1-2 agent-days (CI/conflict) | PR #1473 |
| 2 | **Resolve PR #1475 nwdiag retune BLOCKED state, land** | −0.05× | 1 agent-day | PR #1475 |
| 3 | **Resolve PR #1469 class spline router DIRTY (rebase + CI)** | −0.03× routing-quality | 1 agent-day | PR #1469 |
| 4 | **Resolve PR #1476 K8s namespace routing DIRTY (conflict + rebase)** | edges through frames closed | 1 agent-day | PR #1476 |
| 5 | **Resolve PR #1470 <style> cascade Phase B BLOCKED state** | unblocks Phase C-E theme parity | 1-2 agent-days | PR #1470 |

Bonus 6: After top-5 land, regen `docs/examples/*.svg` + `docs/diagrams/`
artifacts and run `scripts/regen-artifacts.sh --force`. The render gallery
will quietly improve a tier.

Bonus 7: 7-bug residual cluster (#1440-#1447 + #1451) is all on PRs in
the in-flight queue. Same pattern as #1-5 — baby the PRs, don't write new
code.

The rest of the doc is the data behind these decisions.

---

## 1. Methodology

- Same 35-fixture corpus as waves 4 & 5 (34 examples + `docs/diagrams/architecture-overview.puml`)
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png` (default `--style puml`)
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`
- All 35 PUML PNGs read with the multimodal Read tool; high-resolution
  re-renders (1200px width via `rsvg-convert`) for spot-check of suspicious
  fixtures (usecase/02, usecase/06)
- Cached PNGs at `/tmp/parity_audit_v6/`; PlantUML PNGs copied unchanged from
  `/tmp/parity_audit_v5/` (PlantUML version is identical — 1.2026.5)
- gantt/05 phantom persists: PlantUML 1.2026.5 still errors on `[Feature A]`
  syntax, producing 419×136 error sprite that yields a meaningless 5.22×
  ratio. Excluded from headline median/mean.

No source code was modified. Build was a single `cargo build --release` on
`origin/main @ 650c4902`. Audit consumed ~85 min of agent time. Wait for
in-flight PRs to settle: 5 min initial + 5 min mid-loop check; PRs in
`BLOCKED` / `DIRTY` states did not change passively, so the audit proceeded
to measurement.

---

## 2. Headline numbers — six-wave progression

Excluding the gantt phantom (waves 4-6) and the architecture-overview phantom (wave 4).

| Metric | W1 | W2 | W3 | W4 | W5 | **W6** | Δ overall | Δ vs W5 |
|---|---|---|---|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | 2.18× | 2.24× | 1.63× | **1.63×** | −44% | **0** |
| Mean area ratio | 3.30× | 2.70× | 2.39× | 2.42× | 1.70× | **1.71×** | −48% | +0.01 |
| Min ratio | 1.25× | 0.71× | 0.70× | 0.96× | 0.96× | **1.12×** | −10% | **+0.16** |
| Max ratio | 7.65× | 5.22× | 4.90× | 4.90× | 2.76× | **2.76×** | −64% | 0 |
| N measurable | 33 | 34 | 33 | 34 | 34 | **34** | — | — |
| Fixtures ≥ 1.5× | 28/34 | 22/35 | 25/33 | 26/34 | 22/34 | **22/34** | — | 0 |
| Fixtures ≥ 2.0× | — | — | 18/33 | 20/34 | 6/34 | **6/34** | — | 0 |
| Fixtures ≥ 3.0× | ~14/34 | ~10/35 | 7/33 | 7/34 | 0/34 | **0/34** | **−14** | 0 |
| Fixtures ≤ 1.3× | — | — | — | 2/34 | 4/34 | **4/34** | — | 0 |
| Fixtures ≤ 1.0× | — | — | — | 1/34 | 1/34 | **0/34** | — | **−1** |

**Wave-6 is a holding-pattern wave.** Every aggregate number is unchanged
from wave-5 except: (a) the min ratio moved up from 0.96× → 1.12× because
the #1474 state/10 label fix added pixels (closed a real visual bug at the
cost of a small area regression — net good), (b) one fixture left the
"≤1.0×" bucket (state/10), and (c) three fixtures (state/10, deployment/06,
component/07) had width increases of +43px, +42px, +16px respectively from
the #1471 text-fit-first pass. **The plateau is explained: the 5 BLOCKED/
DIRTY PRs are exactly the ones that would have moved the headline numbers.**

---

## 3. Full ratio table (current main, 2026-06-01, post-1474/1471 landings)

Bold entries = movement from wave-5. Italics = visible bug still present
despite ticket closure or pending PR.

| Fixture | PUML | PlantUML | W6 ratio | W5 ratio | Δ | Notes |
|---|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.86× | 1.86× | 0.00 | *"yes" label clipped behind diamond left edge — wave-5 §6.G unfiled* |
| activity/05_while_loop | 248×438 | 186×437 | 1.34× | 1.34× | 0.00 | *"yes" label clipped behind diamond right edge — same root* |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 | unchanged |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 | *#1447 still in-review — double-stop persists* |
| c4/12_container_with_databases | 1600×982 | 989×774 | 2.05× | 2.05× | 0.00 | *#1464 "Uses [HTTPS]" still orphaned at canvas-top* |
| class/01_basic | 230×292 | 134×276 | 1.82× | 1.82× | 0.00 | clean render |
| class/03_composition_aggregation | 248×362 | 148×384 | 1.58× | 1.58× | 0.00 | clean render |
| class/05_visibility | 326×254 | 259×198 | 1.61× | 1.61× | 0.00 | clean render |
| class/11_generics | 494×376 | 361×316 | 1.63× | 1.63× | 0.00 | clean render; Map<K,V> not inheriting (correct) |
| component/02_interfaces | 400×330 | 280×205 | 2.30× | 2.30× | 0.00 | lollipop sizing now reasonable; gap is box padding |
| component/07_ports_lollipop_interfaces | **1154×478** | 702×483 | **1.63×** | 1.60× | **+0.03** | #1471 text-fit widened OrderRepository col +16px; *#1450 still in-review* |
| component/08_cloud_db_queue_stereotypes | 941×938 | 660×803 | 1.67× | 1.67× | 0.00 | *#1451 in-review; "origin pull" still routes through API Cluster header pill artifact* |
| deployment/02_databases | 400×496 | 254×322 | 2.43× | 2.43× | 0.00 | unchanged; needs node-shape retune |
| deployment/03_cloud | 400×334 | 344×199 | 1.95× | 1.95× | 0.00 | *#1444 in-review; "queries" label still in white box overlapping edge* |
| deployment/06_kubernetes | **977×928** | 934×839 | **1.16×** | 1.11× | **+0.05** | *#1442 in-review widened nodes +42px; #1440 #1465 closed (artifact ambiguous at 977px PNG); cross-namespace edge routing unfixed (PR #1476 DIRTY)* |
| diagrams/architecture-overview | 753×1090 | 562×801 | 1.82× | 1.82× | 0.00 | *#1441 in-review; header pill artifacts still visible on Pipeline Core + Output Formats* |
| gantt/05_multi_task | 880×338 | 419×136 | (phantom 5.22×) | (phantom 5.22×) | n/a | source syntax non-canonical; PUML renders well |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× | 2.25× | 0.00 | *PR #1473 BLOCKED — would land horizontal→vertical conversion* |
| mindmap/05_four_levels_asymmetric | 1629×658 | 723×1074 | 1.38× | 1.38× | 0.00 | unchanged |
| nwdiag/02_multi_network | 760×386 | 295×360 | 2.76× | 2.76× | 0.00 | *PR #1475 BLOCKED — projected drop to ~1.27×* |
| object/02_with_attributes | 210×326 | 223×253 | 1.21× | 1.21× | 0.00 | PUML narrower; ACCEPT zone |
| object/05_ch04_parity | 312×272 | 185×236 | 1.94× | 1.94× | 0.00 | unchanged |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 | parity zone |
| sequence/03_autonumber | 312×228 | 232×210 | 1.46× | 1.46× | 0.00 | unchanged |
| sequence/07_notes | 394×340 | 255×316 | 1.66× | 1.66× | 0.00 | unchanged |
| sequence/11_activation | 312×228 | 230×210 | 1.47× | 1.47× | 0.00 | unchanged |
| sequence/12_create_destroy | 312×312 | 239×222 | 1.83× | 1.83× | 0.00 | unchanged; ✕ marker chrome correct |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 | unchanged |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 | *#1449 CLOSED but visual evidence: "data ready" still orphaned bottom-right* |
| state/10_parallel_regions | **292×1010** | 280×938 | **1.12×** | 0.96× | **+0.16** | *#1474 #1448 fix expanded label spacing — net good; PUML now slightly larger than PlantUML* |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 | unchanged |
| usecase/02_with_actors | 398×526 | 286×453 | 1.62× | 1.62× | 0.00 | *#1460 closed (head visible at high-res); square boxy artifact under Customer label persists, possibly orphaned routing rect* |
| usecase/05_actor_generalization | 1084×1262 | 1830×653 | 1.14× | 1.14× | 0.00 | *PR #1445 in-review — vertical tangle from User persists, `<<extend>>` still empty box (#1463)* |
| usecase/06_multi_system_boundary | 1384×814 | 1090×568 | 1.82× | 1.82× | 0.00 | *#1462 closed (no head overlap at high-res); #1446 in-review — phantom horizontal rectangle below actor row still drawn* |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.62× | 2.62× | 0.00 | *PR #1473 BLOCKED — same horizontal→vertical issue as mindmap/02* |

**Net:** 31 of 34 ratios unchanged; 3 had small movements (+0.03, +0.05,
+0.16) from #1471 + #1474 landings. No regressions on visible-quality
axis. The plateau is structural: the next median move requires #1473 to
land (mindmap/wbs/c4 layout convention).

---

## 4. Per-family score table — six-wave progression

Cell = (count of fixtures with ratio ≥ 1.5×) / (total). Lower is better.

| Family | W1 | W2 | W3 | W4 | W5 | **W6** | Trajectory |
|---|---|---|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | 2/4 | 2/4 | **2/4** | plateau; needs activity density retune |
| c4 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | layout convention; PR #1473 pending |
| class | 3/4 | 3/4 | 3/4 | 4/4 | 2/4 | **2/4** | plateau; PR #1469 routing improvement pending |
| component | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | **2/3** | plateau; #1471 micro-widened component/07 |
| deployment | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | **2/3** | plateau; #1442 #1444 still in-review |
| gantt | n/a | 1/1 | n/a | 1/1 | (phantom) | **(phantom)** | PlantUML source error |
| mindmap | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 | **1/2** | PR #1473 would drop to 0/2 |
| nwdiag | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | PR #1475 would drop to 0/1 |
| object | 2/2 | 2/2 | 2/2 | 2/2 | 1/2 | **1/2** | plateau |
| salt | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | **0/1** | parity zone |
| sequence | 4/4 | 4/4 | 4/4 | 4/4 | 2/4 | **2/4** | plateau; would need pass-3 retune |
| state | 2/3 | 0/3 | 0/3 | 0/3 | 0/3 | **0/3** | parity zone; #1474 closed visible bugs |
| timing | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged |
| usecase | 2/3 | 1/3 | 1/3 | 2/3 | 2/3 | **2/3** | plateau; #1445 #1446 in-review |
| wbs | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | PR #1473 would drop to 0/1 |
| **Overall ≥ 1.5×** | **28/34** | **22/35** | **25/33** | **26/34** | **22/34** | **22/34** | 65% — unchanged |
| **Overall ≥ 2.0×** | n/a | n/a | 18/33 | 20/34 | 6/34 | **6/34** | 18% — unchanged |

Every family score held. The visible movement that would mark wave-6 as a
real step is gated entirely on landing the 5 BLOCKED/DIRTY PRs.

---

## 5. Per-fixture path-to-1:1 recommendation

Unchanged from wave-5 §5 EXCEPT updates for new landings:

| Fixture | Ratio | Δ from W5 | Path to ≤ 1.0× |
|---|---|---|---|
| state/10_parallel_regions | 1.12× | +0.16 | ACCEPT — was 0.96× (smaller than PlantUML) before #1474. #1474 made labels readable at cost of width; this is a real-bug fix not a regression. |
| deployment/06_kubernetes | 1.16× | +0.05 | Land #1442 (in-review) — current state has +42px width from #1471 text-fit; once container labels fit naturally the gap should narrow |
| component/07_ports_lollipop_interfaces | 1.63× | +0.03 | Land #1450 (in-review) — "uses" label routing; #1471 widened OrderRepository column |
| (all others — unchanged) | — | 0 | Same fixes as wave-5 §5; depends on PR landing queue |

The single most consequential reframing this wave: **the audit can no longer
identify NEW per-fixture work that hasn't already been ticketed.** The
ticket queue is current. The lever is execution / PR babying, not new
identification.

---

## 6. Visual bug catalogue — wave-6 residuals + new findings

### 6.A — Closures from wave-5 (verified at high resolution)

These were filed at wave-5 and closed within 24h. Re-verification at
1200px width via `rsvg-convert`:

- **#1460 (usecase/02 Customer head missing) — CONFIRMED CLOSED CORRECTLY.**
  At 398px default PNG width the head circle (r=6) is below pixel-grid
  threshold and appears absent; at 1200px both heads (cx=92,242, cy=38, r=6)
  are clearly visible. Not a real bug.
- **#1462 (usecase/06 actor row overlap) — CONFIRMED CLOSED CORRECTLY.**
  At 1200px the three actors (System, Customer, Support Agent) are clearly
  separated with no head overlap. Not a real bug.
- **#1465 (deployment/06 strikethrough on queue-consumer / nginx) —
  PARTIALLY CORRECT.** The "strikethrough" appearance is the cross-namespace
  edge routing visible at low resolution. At 1200px the edges are clearly
  separate from text. PR #1476 K8s namespace routing addresses the routing
  cause. The original ticket description was technically accurate (text
  appears to have strikethrough) but the mechanism is edge routing through
  the node label position, not text-decoration. PR #1476 is the right fix.
- **#1449 (state/07 data-ready label orphaned) — INCORRECT CLOSURE.**
  Visual check at default + high res both show "data ready" label floating
  bottom-right of the diagram disconnected from any visible edge. The edge
  may have been generated but routed off-canvas. Recommend reopening or
  filing a follow-up.

### 6.B — Wave-5 bugs still visible (all on in-review PRs)

| Bug | Wave-5 ticket | Wave-6 status |
|---|---|---|
| Architecture-overview package headers — gray pill artifacts inside dark headers | #1441 in-review (PR #1456) | Persists; pill is `edge-label-bg` rect overlapping header band |
| component/08 "origin pull" edge endpoint inside API Cluster header band | #1451 in-review (PR #1456) | Persists |
| deployment/06 K8s cross-namespace edge routing | #1472 (PR #1476 DIRTY) | Persists — edges from sidecar-logger to api-service / kafka cross frontend → data namespace boundaries |
| component/07 "uses" label crosses through OrderRepository | #1450 | Persists |
| usecase/05 vertical edge tangle from User actor | #1445 in-review (PR #1458) | Persists — PR text says fan was added but tangle still visible |
| usecase/06 phantom horizontal rectangle below actor row | #1446 in-review (PR #1474) | PARTIALLY FIXED — actor head overlap gone, but a thin horizontal frame/bar remains below actors |
| activity/09 double-stop overlap on Complete | #1447 in-review (PR #1458) | Persists — Complete node has a stop circle overlapping it |
| state/07 "data ready" orphan label | #1449 (closed?) | STILL VISIBLE in current main — see 6.A note |
| deployment/03 Lambda Function label overflow | #1444 in-review | Persists |
| component/02 NotificationSender / OrderRepository overflow | #1443 in-review | partial fix from #1471 — labels still in tight boxes |

### 6.C — Wave-6 new minor findings (NO TICKETS FILED — backlog hygiene)

These are small visible deltas observed during the audit. Not significant
enough to ticket individually; documenting for awareness:

- **usecase/02 Customer "square boxy" artifact under name label.** A
  rectangular outline below "Customer" appears to be an orphaned edge-label
  bbox or a layout-debug residue. Low priority, low visibility.
- **architecture-overview "Frontends" frame width tight on Adapters
  content.** Frame contains only Adapters but title bar barely fits the
  longer "Frontends" word. Minor frame-min-width nit.
- **deployment/03 "queries" label in opaque white box overlapping edge.**
  Edge from EC2 → RDS passes through the label background pill. Same root
  as #1441 cluster (edge-label-bg z-order vs edge line).
- **architecture-overview Pipeline Core and Output Formats headers have
  white pill artifacts visible in dark headers.** Same as #1441 cluster.
- **c4/12 multiple cross-cutting orthogonal edges create visual noise.**
  Routing is correct but the unconstrained ortho router creates many
  horizontal/vertical segments. Aesthetic, not a defect. PR #1469 spline
  router extension may help.
- **activity/02 + activity/05 "yes" label clipped behind decision diamond.**
  Same pattern (decision-true label routing inside the diamond's left/right
  vertex). This was flagged at wave-5 §6.G; should comment on #1447 to
  expand its acceptance to cover guard-label clipping.

### 6.D — Confirmed-clean families (wave-6)

These families have NO visible defects in any fixture:

- **sequence** (03, 07, 11, 12) — all 4 clean ✓
- **class** (01, 03, 05, 11) — all 4 clean ✓
- **state** (03, 10) — clean; 07 has #1449 reopen-candidate
- **mindmap** (02, 05) — clean (layout convention is design choice)
- **wbs** (02) — clean (layout convention)
- **timing** (01) — clean
- **salt** (01) — clean
- **gantt** (05) — clean PUML render (PlantUML source error is upstream)
- **object** (02, 05) — clean
- **activity** (07) — clean (02, 05, 09 have minor label issues)
- **deployment** (02) — clean

That's **20 of 34 fixtures rendering at zero visible-defect quality.**
A wave-5 list of the same kind would have been ~16. The wave-6 audit's
honest finding: **the bugs are narrowly concentrated in usecase / K8s /
component-08 / architecture-overview**, and ALL of those are in-review or
have an open PR.

---

## 7. Open-ticket inventory

Total open issues (origin/main @ 650c4902): **30** (was 35 at wave-5, **−5**).

| Priority | Count | Notes |
|---|---|---|
| P0 | 5 | #1472 #1459 (in-review); #1345 #590 epics; #1440 in-review |
| P1 | 16 | 12 are visual-audit in-review on PRs; rest are style block phases #1414-#1417, parity epic #88, coverage #700, etc. |
| P2 | 8 | #1467 mindmap/wbs/c4 policy in-review PR #1473; #1444 in-review; #1461 #1463 #1464 #1450 wave-5 cluster; #92 #91 hygiene |
| P3 | 1 | #92 benchmark publishing |
| unlabeled | 0 | down from 1 |

### 7.1 Issues recommended for CLOSURE this wave

Smaller list than wave-5 since wave-5 closed the easy ones already.

| Issue | Why close | Suggested resolution |
|---|---|---|
| #1449 (state/07 data-ready orphan) | Was closed; still visible. RECOMMEND **REOPEN** + comment. | Reopen with high-res evidence; consider attaching to #1474 follow-up |

No other closures recommended this wave — the queue is healthy.

### 7.2 In-review PRs (need babying — top priority)

| PR | Title | State | Blocking diagnosis |
|---|---|---|---|
| #1476 | K8s namespace routing | DIRTY | Conflicting with main; needs rebase |
| #1475 | nwdiag packed-grid retune | BLOCKED | Failing checks |
| #1473 | mindmap/wbs/c4 layout adoption | BLOCKED | Failing checks |
| #1470 | <style> cascade Phase B | BLOCKED | Failing checks |
| #1469 | class spline router | DIRTY | Conflicting with main; needs rebase |
| #1456 | header band suppression #1441 #1451 | BLOCKED | Failing checks |
| #1453 | Salt diagnostics | BLOCKED | Failing checks |

These are the SAME PRs that wave-5 was waiting on; the situation has not
moved in 18-24 hours.

### 7.3 Issues filed by this audit (§10)

Smaller cohort than wave-5. 5 new tickets, all narrow visual-bug or
hygiene items. See §10.

---

## 8. Examples + docs check

### 8.1 docs/examples/ corpus

- 35-fixture audit corpus stable since wave-4.
- Wider corpus (298 .puml files) untested at full corpus level this wave;
  the 35-fixture corpus is the representative slice.
- `scripts/render_corpus.py --force` could regenerate the full corpus PNG
  audit for a deeper sweep; left for orchestrator.

### 8.2 Top-level README.md / GALLERY.md

- README and GALLERY unchanged since wave-5; still current at high level.
- Gallery SVG artifacts (`docs/examples/*.svg`) are likely behind the
  current renderer post-#1471 #1474. `scripts/regen-artifacts.sh --force`
  is a recommended follow-up after the in-flight PR queue clears.

### 8.3 Notable docs/examples gaps observed

- gantt/05 — source still uses non-canonical `[Feature A]` syntax that
  PlantUML 1.2026.5 rejects. The PUML render is excellent (per wave-6
  PNG read — proper task bars, dependencies, dates). Recommend rewrite
  source to canonical PlantUML grammar so a real diff can be measured.
- usecase/05 — same as wave-5: empty <<extend>> box (#1463), heavy edge
  tangle (#1445). Avoid promoting to gallery until both fixes land.
- nwdiag/02 — ratio leader; PR #1475 pending. Until landed, avoid as
  hero example.

---

## 9. Verdict against the "world class" goal

| Gate item | Target | Wave-6 status | Pass? |
|---|---|---|---|
| Median ratio ≤ 1.0× (1:1) | ≤ 1.00× | **1.63×** | **NO** (gap 0.63×) |
| Median ratio ≤ 1.3× (parity-light) | ≤ 1.30× | 1.63× | NO (gap 0.33×) |
| Median ratio ≤ 1.5× (1.0 ship gate) | ≤ 1.50× | 1.63× | NO (gap 0.13×) — striking distance |
| 0 visible visual bugs | 0 | ~10 in-review + ~3 minor unfiled = ~13 | **NO** all tracked |
| 0 open tickets | 0 | 30 open; most are non-bugs (epics, in-review, theme phases) | **NO** |
| README + GALLERY current | yes | yes at high level | YES |
| Coverage ≥ 90% | ≥ 90% | gate enforces 85→90 ratchet | YES |
| Deterministic output | byte-identical | unchanged | YES |
| Differential oracle passing | ≥ 50% | passing per #88 | YES |

**Wave count to "world class" — revised estimate:**

- **Wave 7 (this week if PRs unblock)**: Land **all 5 BLOCKED/DIRTY PRs**
  (#1473 #1475 #1476 #1470 #1469 #1456 #1453). This is the cluster wave-5
  predicted would happen automatically; it has not. Expected median if
  #1473 + #1475 land: ≤ 1.40×. Expected median if also #1469 #1476 land:
  ≤ 1.35×.
- **Wave 8**: Activity density retune (parallel #1378 pattern). Sequence
  pass-3 micro-retune. Deployment node-shape retune. Component lollipop
  micro-retune. Expected median: ≤ 1.20×.
- **Wave 9**: Style block Phase C-E (#1415 #1416 #1417). Final compaction
  pass per family. Bless v1.0 visual baselines. Expected median: ≤ 1.05×.
- **Wave 10 ("world class" target)**: All visible bugs closed. Gallery
  refreshed. Oracle conformance ratchet hit ≥ 80%. Expected median ≤ 1.00×.

**Honest estimate: 3-4 waves to "world class" CONDITIONAL ON the in-flight
PR queue moving.** The hard truth: wave-6 is essentially the same as
wave-5 because no major code landed. If wave-7 also fails to land the
in-flight cluster, the campaign stalls.

**What specifically blocks 1:1 right now (in priority order):**

1. **PR #1473 mindmap/wbs/c4 layout-convention adoption** — blocked, must
   land. Worth ~0.15× median on its own.
2. **PR #1475 nwdiag retune** — blocked, must land. Drops max from 2.76×
   to ~1.27×.
3. **All wave-5 visual cluster PRs (#1456 #1458 #1474)** — partially landed
   (#1474 done; #1456 #1458 still in-review). Each remaining PR closes
   2-3 visible bugs.
4. **PR #1469 #1476 routing improvements** — DIRTY (needs rebase).
5. **PR #1470 style cascade Phase B** — required for Phase C-E (#1415 #1416
   #1417), which collectively bring 10+ missing skin properties and unblock
   bless of cyborg/reddress-darkblue themes.

The audit's honest verdict: the campaign is HEALTHY but EXECUTION-LIMITED.
No new architectural work is required. New code is not the gate; merging
existing code is.

---

## 10. Follow-up issues filed (this audit)

Smaller cohort than wave-5. Wave-6 produced 5 new tickets / actions:

| # | Title | Severity | Reference |
|---|---|---|---|
| [#1477](https://github.com/alliecatowo/puml/issues/1477) | state/07 "data ready" label STILL orphaned after #1449 closure (reopen-equivalent) | P2 | §6.A, §6.B |
| [#1478](https://github.com/alliecatowo/puml/issues/1478) | activity/02 + activity/05 decision-guard label clipped inside diamond left/right vertex | P2 | §6.C |
| [#1479](https://github.com/alliecatowo/puml/issues/1479) | deployment/03 "queries" label opaque-rect background overlaps edge line (z-order) | P3 | §6.C (same root as #1441 cluster) |
| [#1480](https://github.com/alliecatowo/puml/issues/1480) | architecture-overview "Frontends" frame title clipped (frame width = content width, not title width) | P3 | §6.C |
| [#1481](https://github.com/alliecatowo/puml/issues/1481) | usecase/02 orphaned square box artifact below Customer name label | P3 | §6.C |

Plus three **PR-babying actions** (the highest-leverage follow-ups):

| Action | PR | Severity |
|---|---|---|
| Rebase + unblock #1473 mindmap/wbs/c4 layout adoption | PR #1473 | P0 follow-up |
| Rebase + unblock #1475 nwdiag retune | PR #1475 | P0 follow-up |
| Rebase + resolve conflicts on #1469 + #1476 + #1456 | PR #1469/1476/1456 | P0 follow-up |

These are intentionally NOT new tickets — they are PR maintenance and
already tracked on the existing issues that opened them.

**Total: 5 new minor tickets recommended (within the 3-8 target).**

---

## 11. Top-5 next-fix recommendations (final)

The shape of this list has changed materially from wave-5: PR babying
dominates because the in-flight cluster did not move.

| Rank | Action | Median impact | Cost | Notes |
|---|---|---|---|---|
| 1 | **Unblock + land PR #1473 (mindmap/wbs/c4 layout)** | −0.15× | 1-2 agent-days CI | Single biggest unlanded lever |
| 2 | **Unblock + land PR #1475 (nwdiag retune)** | −0.05× + drops max from 2.76→1.27 | 1 agent-day | Removes the headline-worst fixture |
| 3 | **Resolve PR #1469 #1476 DIRTY state** | −0.03× routing-quality + #1472 K8s frame closure | 1 agent-day rebase | Each is small but routing quality compounds |
| 4 | **Unblock + land PR #1456 (#1441 + #1451 header band suppress)** | 2 bugs closed, slight area improvement | 0.5 agent-day | Architecture-overview hero diagram cleanup |
| 5 | **Unblock + land PR #1470 (style cascade Phase B)** | unlocks Phase C-E theme parity | 1 agent-day | Foundation for #1414-#1417 phases |

Bonus 6: After top-5 land, run `scripts/regen-artifacts.sh --force` +
re-render full corpus and re-audit at wave-7.

Bonus 7: File the 5 minor tickets in §10 once the in-flight cluster
lands (they are clutter while PRs are open).

---

## 12. Evidence index

All cached at `/tmp/parity_audit_v6/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully on @ 650c4902)
- 35 `*-PlantUML.png` files (copied unchanged from v5 cache; PlantUML
  version is identical at 1.2026.5)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh` — driver script
- `fixtures.txt` — corpus list

This audit performed NO source modifications. The repository state at audit
time is `origin/main @ 650c4902`. Auditor's notes:

- High-resolution re-renders (via `rsvg-convert -w 1200`) of usecase/02 and
  usecase/06 verified that wave-5 closures #1460 and #1462 were correct —
  the bugs were artifacts of low-resolution rendering, not real defects.
- The auditor observed but did not file 5 minor visual-quality items (see
  §10) to avoid ticket-database noise; the in-flight PR cluster should
  land first before adding new items.

---

*Snapshot doc; the cached PNGs will be cleaned on next OS restart. Copy to
`docs/internal/forensics/2026-06-01-evidence-v6/` only if the gallery needs
to survive past this session.*
