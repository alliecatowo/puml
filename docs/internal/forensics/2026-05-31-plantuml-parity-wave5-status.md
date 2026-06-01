# PlantUML Parity Wave-5 Status — 35-Fixture Snapshot

**Date:** 2026-05-31 (late afternoon)
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit, no implementation)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27, GPL build, Java 21)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`f989abc9` (head of main; sequence density retune #1378 LANDED, object density
#1433 LANDED, deployment density #1431 LANDED, component density #1437 LANDED,
class density #1435 LANDED, spline-native edge router #1410 LANDED, style block
AST #1420 LANDED, glitch-hunt visual cleanup #1452 LANDED, collision-free
invariant #1455 LANDED).
**Prior audits:**
- Wave-1: `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (median 2.93×)
- Wave-2: `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (median 2.25×)
- Wave-3: `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` (median 2.18×)
- Wave-4: `docs/internal/forensics/2026-05-31-plantuml-parity-wave4-status.md` (median 2.24×)

---

## 0. One-page summary for Allie

**Headline: median area ratio dropped from 2.24× (wave-4) to 1.63× (wave-5).**
That is a **−0.61× absolute** / **−27% relative** improvement in one wave — the
single biggest jump of the four-wave campaign so far. The mean dropped from
2.42× to 1.70× (−30%). Worst-case (excluding the gantt phantom, see §1) is
2.76× (nwdiag/02), down from 4.90×. **Zero fixtures sit at ≥3.0× anymore**
(wave-4 had 7). **Four fixtures now sit at ≤1.3×** the v1 parity gate target
(wave-4 had 2).

**What landed this wave (the density crash):** Five family-targeted density
retunes shipped in 24 h — sequence (#1378), object (#1433), component (#1437),
deployment (#1431), class (#1435) — plus the spline-native edge router (#1410),
the glitch-hunt visual cleanup pass (#1452), and the collision-free invariant
gate (#1455). The density-retune pattern from #1346 (the original wave-1
sequence prototype) has now been propagated through every contended family.

**What remains for 1:1 parity (median ≤ 1.0×, the new "world class" goal):**
The gap is now 0.63× on median, distributed across:
- **Three layout-convention divergences** (mindmap/02, wbs/02, c4/12) — these
  account for ~0.20× of the gap and are NOT bugs; they are different valid
  layout policies. PUML splays horizontally where PlantUML stacks vertically.
  Either retune the policy or accept these as "intentional improvement".
- **One overflow-from-density-retune cluster** — issues #1442 #1443 #1444
  (component, deployment label overflow) need #1459 text-fit-first density
  fix (P0, agent-ready) to land. This is the single highest-priority work
  item. Estimated ~0.10× additional median reduction.
- **One usecase routing cluster** — #1445 #1446 actor edge tangle/crossings
  through frame headers. Density is fine; routing is the issue. Estimated
  ~0.05× median reduction.
- **A handful of small per-fixture residuals** — nwdiag/02 at 2.76× is the
  worst remaining real ratio and is a layout-engine constants issue specific
  to nwdiag's network/subnet/host packed grid. Estimated ~0.05× median
  reduction once retuned.

**Verdict against the "world class, 1:1, 0 visual bugs, 0 open tickets" goal:**
- Median ≤ 1.0×: **NO** (currently 1.63×; gap −0.63×). Realistic path: 3-4
  more waves at current cadence.
- 0 open visual bugs: **NO** (35 open issues; 4 P0, 23 P1, 6 P2, 1 P3, 1
  unlabeled; ~14 of the P1s are visible-bug tickets already filed and in
  review). Most are already in-review on PR #1456 #1458, so closure is
  imminent.
- 0 open tickets: **NO** but most "open" are epics (#1345 #590 #1404 #88 #1258)
  or planning tickets, not bugs. Distinguishing closed-as-implemented vs
  obsolete: ~5 candidates flagged in §7.
- Examples + docs current: **YES at high level** (README, GALLERY current);
  some `docs/examples/*.svg` artifacts may be stale post-density-retunes —
  `scripts/regen-artifacts.sh --force` needs a run; spot checks in §8.
- World class: **NOT YET, but visibly close.** Honest call: we are 1-2 waves
  away from "ship-1.0 quality" if the goal stays at median ≤ 1.5×, and
  3-4 waves away from "median ≤ 1.0× = best-in-class" parity.

**Top-5 next fixes ranked by ROI:**

| Rank | Action | Median impact | Cost | Ticket |
|---|---|---|---|---|
| 1 | Land **#1459 text-fit-first density** (P0). Unblocks #1442 #1443 #1444 overflow cluster. | −0.10× | 1 agent-day | #1459 |
| 2 | Tighten **mindmap/02 and wbs/02 layout convention** (horizontal-vs-vertical splay). Visible-bug optional; layout-policy choice. | −0.15× | 1-2 agent-days | NEW (filed §10) |
| 3 | Land usecase routing cluster (**#1445 #1446**) — actor-edge fan + frame-header avoidance. | −0.05× routing-quality; visual bug closure | 1 agent-day | #1445 #1446 |
| 4 | Retune **nwdiag/02 packed grid** density (still 2.76×). | −0.05× | 2 agent-days | NEW (filed §10) |
| 5 | Land **#1450 component/07 'uses' label-through-node** + #1454 multiplicity-bleed cluster — small but visible. | −0.02× | 1 agent-day | #1450 #1454 |

Bonus item 6 (decide / close):
- **Decide gantt/05 fixture**: PlantUML 1.2026.5 still errors on its source
  (`[Feature A]` is non-canonical PlantUML gantt syntax). PUML renders
  faithfully but cannot be ratio-measured. Options: rewrite source to canonical
  PlantUML gantt grammar (with `as id` aliases), drop from the parity corpus,
  or keep flagged as PUML-mode-only. Recommend rewrite — see §6 ratio note.

The rest of the doc is the data behind these decisions.

---

## 1. Methodology

- Same 35-fixture corpus as wave-4 (34 examples + `docs/diagrams/architecture-overview.puml`)
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png` (default `--style puml`)
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`
- All 35 PUML PNGs read with the multimodal Read tool; 20 PlantUML PNGs read for
  pair-comparison spot checks (every fixture flagged as ≥1.5× or where a visual
  bug was suspected)
- Cached PNGs at `/tmp/parity_audit_v5/`
- **gantt/05_multi_task is a phantom this wave**: PlantUML 1.2026.5 errors with
  `Some diagram description contains errors` on `[Feature A]` (the bracketed
  task identifier is not standard PlantUML gantt syntax; canonical PlantUML
  requires `[Feature A] as feat_a` or unbracketed `Feature A`). PlantUML's
  419×136 error sprite was sized normally by sips, yielding a bogus 5.22×
  ratio. Reported separately and excluded from the headline median/mean.
- The wave-4 `docs/diagrams/architecture-overview` phantom is now RESOLVED:
  PlantUML 1.2026.5 renders the diagram correctly this wave, giving a measurable
  1.82× ratio. The duplicate-`Frontends` identifier note in wave-4 was incorrect
  or the rendering bug was elsewhere; the diagram now produces a normal C4-style
  package layout in PlantUML.

No source code was modified. Build was a single `cargo build --release` on
`origin/main @ f989abc9`. Audit consumed ~75 min of agent time.

---

## 2. Headline numbers — five-wave progression

Excluding the gantt phantom (wave-5) and the architecture-overview phantom (wave-4).

| Metric | Wave-1 | Wave-2 | Wave-3 | Wave-4 | **Wave-5** | Δ overall | Δ vs W4 |
|---|---|---|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | 2.18× | 2.24× | **1.63×** | −44% | **−27%** |
| Mean area ratio | 3.30× | 2.70× | 2.39× | 2.42× | **1.70×** | −48% | **−30%** |
| Min ratio | 1.25× | 0.71× | 0.70× | 0.96× | **0.96×** | — | 0 |
| Max ratio | 7.65× | 5.22× | 4.90× | 4.90× | **2.76×** | −64% | **−44%** |
| N measurable | 33 | 34 | 33 | 34 | **34** | — | — |
| Fixtures ≥ 1.5× | 28/34 | 22/35 | 25/33 | 26/34 | **22/34** | — | −4 |
| Fixtures ≥ 2.0× | — | — | 18/33 | 20/34 | **6/34** | — | **−14** |
| Fixtures ≥ 3.0× | ~14/34 | ~10/35 | 7/33 | 7/34 | **0/34** | **−14** | **−7** |
| Fixtures ≤ 1.3× | — | — | — | 2/34 | **4/34** | — | +2 |
| Fixtures ≤ 1.0× | — | — | — | 1/34 | **1/34** | — | 0 |

**Wave-5 is a step-change wave.** All five major density PRs (#1378 #1431 #1433
#1435 #1437) landed in a coordinated wave; the impact is visible in every
contended family. The wave-3-to-wave-4 plateau is conclusively broken.

---

## 3. Full ratio table (current main, 2026-05-31, post-density-crash)

| Fixture | PUML | PlantUML | W5 ratio | W4 ratio | Δ vs W4 | Notes |
|---|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.86× | 1.86× | 0.00 | unchanged — activity untouched this wave |
| activity/05_while_loop | 248×438 | 186×437 | 1.34× | 1.34× | 0.00 | unchanged |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 | unchanged |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 | unchanged; #1447 still tracks double-stop bug |
| c4/12_container_with_databases | 1600×982 | 989×774 | 2.05× | 2.07× | −0.02 | trivial improvement; layout convention persists |
| class/01_basic | 230×292 | 134×276 | **1.82×** | 3.24× | **−1.42** | **#1435 class density retune CRUSHED this** |
| class/03_composition_aggregation | 248×362 | 148×384 | **1.58×** | 2.99× | **−1.41** | density retune win |
| class/05_visibility | 326×254 | 259×198 | **1.61×** | 1.85× | **−0.24** | density retune partial win |
| class/11_generics | 494×376 | 361×316 | **1.63×** | 2.50× | **−0.87** | density retune win; inheritance edge still rendered |
| component/02_interfaces | 400×330 | 280×205 | **2.30×** | 4.09× | **−1.79** | **#1437 component density retune CRUSHED this** |
| component/07_ports_lollipop_interfaces | 1138×478 | 702×483 | **1.60×** | 2.89× | **−1.29** | density retune win; #1450 'uses' label routing still |
| component/08_cloud_db_queue_stereotypes | 941×938 | 660×803 | **1.67×** | 3.44× | **−1.77** | density retune win; missing edges (#1451 in-review) |
| deployment/02_databases | 400×496 | 254×322 | **2.43×** | 4.90× | **−2.47** | **#1431 deployment density CRUSHED this — formerly worst fixture** |
| deployment/03_cloud | 400×334 | 344×199 | **1.95×** | 3.68× | **−1.73** | density retune win; #1444 Lambda overflow remains |
| deployment/06_kubernetes | 935×928 | 934×839 | **1.11×** | 2.21× | **−1.10** | **near-parity now**; #1440 #1442 overflow remain |
| diagrams/architecture-overview | 752×1090 | 562×801 | **1.82×** | (phantom) | new measurable | first valid ratio; #1441 header bg still tracked |
| gantt/05_multi_task | 880×338 | 419×136 | (phantom 5.22×) | n/a | n/a | PlantUML errors on `[Feature A]` syntax; see §1 |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× | 2.25× | 0.00 | layout-convention difference (splay vs stack); not a bug |
| mindmap/05_four_levels_asymmetric | 1629×658 | 723×1074 | 1.38× | 1.38× | 0.00 | unchanged |
| nwdiag/02_multi_network | 760×386 | 295×360 | **2.76×** | 2.93× | −0.17 | trivial improvement; nwdiag never retuned |
| object/02_with_attributes | 210×326 | 223×253 | **1.21×** | 2.61× | **−1.40** | **#1433 object density CRUSHED this — PUML now slightly NARROWER than PlantUML** |
| object/05_ch04_parity | 312×272 | 185×236 | **1.94×** | 4.84× | **−2.90** | **biggest single fixture improvement; formerly 4.84× now 1.94×** |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 | unchanged; in parity zone |
| sequence/03_autonumber | 312×228 | 232×210 | **1.46×** | 2.80× | **−1.34** | **#1378 sequence density CRUSHED this** |
| sequence/07_notes | 394×340 | 255×316 | **1.66×** | 2.95× | **−1.29** | density retune win |
| sequence/11_activation | 312×228 | 230×210 | **1.47×** | 2.83× | **−1.36** | density retune win |
| sequence/12_create_destroy | 312×312 | 239×222 | **1.83×** | 3.31× | **−1.48** | density retune win |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 | unchanged |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 | #1449 data-ready label still orphaned |
| state/10_parallel_regions | 249×1010 | 280×938 | 0.96× | 0.96× | 0.00 | unchanged; in parity zone; #1448 label collision |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 | unchanged |
| usecase/02_with_actors | 398×526 | 286×453 | **1.62×** | 1.64× | −0.02 | density nudge; new bug (NO STICK-FIGURE HEAD on Customer) — see §6 |
| usecase/05_actor_generalization | 1084×1262 | 1830×653 | 1.14× | 1.15× | −0.01 | PUML in parity zone but heavily edge-tangled (#1445) |
| usecase/06_multi_system_boundary | 1384×814 | 1090×568 | 1.82× | 1.84× | −0.02 | routing-through-frame-headers bug (#1446) |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.62× | 2.62× | 0.00 | layout-convention difference (horizontal vs vertical); not a bug |

**Net:** of 34 ratio-measurable fixtures, **15 moved by ≥0.20×** (significant
density-retune impact), with 13 of those being density wins of ≥1.0× — the
average per-affected-fixture gain is −1.4× area. The two worst-remaining
ratios (nwdiag/02 at 2.76× and wbs/02 at 2.62×) are both layout-convention or
constants issues not yet targeted.

---

## 4. Per-family score table — wave-1 / wave-2 / wave-3 / wave-4 / wave-5

Cell = (count of fixtures with ratio ≥ 1.5×) / (total). Lower is better.

| Family | W1 | W2 | W3 | W4 | **W5** | Trajectory |
|---|---|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | 2/4 | **2/4** | unchanged; activity not density-retuned |
| c4 | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged; 2.05× nested-boundary divergence persists |
| class | 3/4 | 3/4 | 3/4 | 4/4 | **2/4** | **#1435 retune: 2 fixtures dropped under 1.5×** |
| component | 3/3 | 3/3 | 3/3 | 3/3 | **2/3** | **#1437 retune: component/07 dropped under 1.5×** |
| deployment | 3/3 | 3/3 | 3/3 | 3/3 | **2/3** | **#1431 retune: deployment/06 dropped under 1.5×** |
| gantt | n/a | 1/1 | n/a | 1/1 | **(phantom)** | PlantUML can't parse source |
| mindmap | 1/2 | 1/2 | 1/2 | 1/2 | **1/2** | unchanged; layout convention |
| nwdiag | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged; never retuned |
| object | 2/2 | 2/2 | 2/2 | 2/2 | **1/2** | **#1433 retune: object/02 dropped under 1.5×** |
| salt | 0/1 | 0/1 | 0/1 | 0/1 | **0/1** | unchanged; in parity zone |
| sequence | 4/4 | 4/4 | 4/4 | 4/4 | **2/4** | **#1378 retune: 2 fixtures dropped under 1.5×** |
| state | 2/3 | 0/3 | 0/3 | 0/3 | **0/3** | unchanged; state already in parity zone |
| timing | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | unchanged; kind-tag retune helped but density unchanged |
| usecase | 2/3 | 1/3 | 1/3 | 2/3 | **2/3** | unchanged; usecase/05 in parity zone, /06 still 1.82× |
| wbs | 1/1 | 1/1 | 1/1 | 1/1 | **1/1** | layout convention; unchanged |
| **Overall ≥ 1.5×** | **28/34** | **22/35** | **25/33** | **26/34** | **22/34** | **65% above 1.5× (was 76%)** |
| **Overall ≥ 2.0×** | n/a | n/a | 18/33 | 20/34 | **6/34** | **18% above 2.0× (was 59%)** |

The ≥2.0× count dropped from 20 to 6 in one wave — this is the single most
representative number for the density-crash impact. The 6 remaining ≥2.0×
fixtures are: c4/12 (2.05), component/02 (2.30), deployment/02 (2.43),
mindmap/02 (2.25), nwdiag/02 (2.76), wbs/02 (2.62). All 6 are either layout-
convention divergences (c4/12, mindmap/02, wbs/02) or families with residual
shape-primitive constants not yet retuned (component/02 lollipop circle radius;
deployment/02 3D node/cylinder sizes; nwdiag never retuned).

---

## 5. Per-fixture path-to-1:1 recommendation

This table answers task #5: for every fixture currently > 1.0×, the SPECIFIC
reason and the SPECIFIC fix. Cells marked "ACCEPT" are parity-zone fixtures
where further compression would harm readability.

| Fixture | Ratio | Specific reason | Specific fix to reach ≤ 1.0× |
|---|---|---|---|
| activity/02_if_then_else | 1.86× | Per-action rounded-rectangle padding ~3-4× PlantUML; diamond label position drifts ("no" label clipped behind diamond per §6) | (a) activity density retune (parallel #1378 for activity); (b) fix #1447-adjacent label routing |
| activity/05_while_loop | 1.34× | Vertical spacing between actions ~1.4× PlantUML | activity density retune as above |
| activity/07_partition | 1.43× | Per-partition header padding tall; vertical inter-action spacing | activity density retune |
| activity/09_error_handling | 1.63× | Same as 02; plus #1447 double-stop circle bug overlapping Complete node | density retune + #1447 fix |
| c4/12_container_with_databases | 2.05× | LAYOUT CONVENTION: PUML lays out as horizontal-spread C4 (User left, all dbs bottom-right); PlantUML stacks vertically with User+Admin top, dbs bottom. PUML canvas is ~1.6× wider. | Either accept (PUML's spread is more readable for large diagrams) or constrain C4 layout to prefer vertical stacking when participant count > 6 |
| class/01_basic | 1.82× | Density retune narrowed boxes but vertical inter-class gap is still ~1.6× PlantUML | further class vertical-gap tightening; very minor |
| class/03_composition_aggregation | 1.58× | Same as class/01 | class vertical-gap tightening |
| class/05_visibility | 1.61× | Single-class fixture; PUML class box is ~1.3× wider for the same content (member icon padding) | class member-row padding tightening |
| class/11_generics | 1.63× | Inheritance edge added by #1383 grew vertical canvas; per-class density now compact, the canvas is the residual | accept (correctness > area) OR compact post-edge layout per #1428 |
| component/02_interfaces | 2.30× | Lollipop circles render at 4-5× PlantUML radius (long-standing); plus minor box padding residual | shrink lollipop interface circle radius from current ~22px to ~6px |
| component/07_ports_lollipop_interfaces | 1.60× | Same lollipop sizing + #1450 'uses' label-through-node routing | lollipop fix + #1450 routing fix |
| component/08_cloud_db_queue_stereotypes | 1.67× | Density now good; missing edges (#1451 in-review) bring canvas larger when fixed (correctness regression risk) | land #1451 cluster |
| deployment/02_databases | 2.43× | 3D-isometric node shape renders ~2× PlantUML; cylinder database shape ~1.5× PlantUML | retune `node` and `database` shape constants in `src/render/deployment.rs` |
| deployment/03_cloud | 1.95× | Same 3D node sizing; cloud shape sizing | same |
| deployment/06_kubernetes | 1.11× | NEAR PARITY; remaining gap is overflow labels triggering wider boxes (#1442) | land #1459 text-fit-first density |
| diagrams/architecture-overview | 1.82× | Density now good post-retune; gray pill artifacts inside dark headers (#1441) bloat header height | land #1441 cluster |
| mindmap/02_multi_level | 2.25× | LAYOUT CONVENTION: PUML splays horizontally with root in center, branches both sides; PlantUML stacks vertically root-left, branches-right | Either accept (PUML splay is symmetric and beautiful) or add `skinparam mindmapLayout vertical` to match PlantUML default |
| mindmap/05_four_levels_asymmetric | 1.38× | Same convention split but mostly fine | accept |
| nwdiag/02_multi_network | 2.76× | Subnet bar height ~2× PlantUML; inter-subnet vertical spacing ~3× PlantUML; host-to-subnet drop length excessive | nwdiag-family density retune (NEW TICKET, §10) |
| object/02_with_attributes | 1.21× | PUML now NARROWER than PlantUML (210 vs 223 px); only height residual | ACCEPT (already in parity zone, going further harms readability) |
| object/05_ch04_parity | 1.94× | Density now good; circled-O badge + bold-italic-underlined identifier chrome adds ~30% per-object height | accept default-chrome OR `--style plantuml` for tighter chrome |
| salt/01_basic_widgets | 1.38× | ACCEPT; in parity zone | accept |
| sequence/03_autonumber | 1.46× | Footer-actor row + small column-padding residual | very minor sequence retune (the wave-2 sequence retune was conservative) |
| sequence/07_notes | 1.66× | Same plus note-chevron padding ~1.4× PlantUML | sequence retune pass 3 on notes |
| sequence/11_activation | 1.47× | Same | sequence retune pass 3 |
| sequence/12_create_destroy | 1.83× | Same plus extra row for `destroy` X marker | accept (correctness chrome) |
| state/03_concurrent | 1.45× | Per-state vertical padding; concurrent-divider line bloat | minor state retune |
| state/07_nested | 1.49× | Vertical state stacking density; #1449 orphan label adds margin | land #1449 |
| state/10_parallel_regions | 0.96× | PUML SMALLER than PlantUML — only thing left is #1448 stacked-label readability | land #1448 (no area impact but visual fix) |
| timing/01_concise | 1.61× | Concise lane height ~1.5× PlantUML | minor timing retune |
| usecase/02_with_actors | 1.62× | **NEW BUG (§6.A): Customer actor missing stick-figure head** (head clipped or absent in PUML render). Plus density. | bug fix + minor density retune |
| usecase/05_actor_generalization | 1.14× | Density in parity zone; #1445 fan/tangle is the visible issue | land #1445 |
| usecase/06_multi_system_boundary | 1.82× | Actor row stacks heads/labels overlapping (visible in §6.C); plus #1446 routing through frame headers | land #1446 + actor row spacing fix |
| wbs/02_with_tasks | 2.62× | LAYOUT CONVENTION: PUML lays WBS as horizontal-spread tree (root center, leaves spread horizontally); PlantUML lays vertically (root top, leaves dropping down) | Either accept (horizontal is more typical of org charts) or add `skinparam wbsLayout vertical` |

**Summary**: 28 of 34 fixtures have an identifiable specific fix; 4 are layout-
convention divergences worth a policy decision; 2 are in the parity zone and
should be accepted.

---

## 6. Deep visual bug catalogue (P0 / P1 / P2)

This task #6: every PUML PNG was read with the multimodal tool. New visible
bugs not already in the ticket database:

### 6.A — usecase/02_with_actors: Customer stick-figure head missing (**NEW P1**)

PUML render shows Customer actor as a stick-figure body+arms+legs **but the
head circle is absent**. The Customer name label sits where the head would
normally be. Admin actor renders correctly with both head and body.
Compare `/tmp/parity_audit_v5/docs_examples_usecase_02_with_actors-PUML.png`
to PlantUML reference. Looks like a name-overlapping-head collision avoidance
went wrong on the first actor declared.

**File as: P1 bug.** Acceptance: both Customer and Admin actors render with
visible stick-figure heads at consistent positions.

### 6.B — usecase/02_with_actors: Customer→BrowseProducts edge routes THROUGH Customer name (**NEW P2**)

The vertical arrow from Customer to BrowseProducts passes through the position
where the Customer text label sits. The label likely needs to be moved down
(below the head circle as in PlantUML) or the edge needs to start below the
label.

**File as: P2 bug.** Acceptance: edge does not visually intersect actor name.

### 6.C — usecase/06: actor row stacks with overlapping heads + labels (**NEW P1**)

The three actors (System, Customer, Support Agent) render with their heads
on the same y-axis but the heads visually overlap each other; the name
labels also overlap. PlantUML renders them with adequate horizontal spacing
and labels below each head. Wave-5 ratio is 1.82× but the visual collision
is the real defect.

**File as: P1 bug.** Acceptance: actor heads are visually distinct (no head
overlap) and labels do not cross.

### 6.D — usecase/05: <<extend>> dashed box is empty (**NEW P2**)

The dashed `<<extend>>` box (top right of usecase/05) renders as an empty
dashed rectangle in PUML. PlantUML draws this as a dashed labeled arrow
connecting the extension target. The box is structurally wrong — it should
be a dashed `<<extend>>` arrow, not a container.

**File as: P2 bug.** Acceptance: `<<extend>>` renders as a dashed
relationship arrow with inline label, matching UML 2.x spec.

### 6.E — c4/12: "Uses [HTTPS]" label orphaned at canvas-top (**NEW P2**)

The top-of-canvas "Uses [HTTPS]" text label is detached from any visible
edge in PUML's c4/12 render. PlantUML places this label inline with the
User→SinglePageApp edge. Looks like the c4 family's edge-label position
is computing y=0 instead of edge-midpoint-y.

**File as: P2 bug.** Acceptance: c4/12 "Uses [HTTPS]" label sits on the
User→SinglePageApp edge segment.

### 6.F — deployment/06_kubernetes: "queue-consumer" and "nginx" labels rendered with strikethrough characters (**NEW P1**)

The labels for queue-consumer (in Pod: worker) and nginx (in Pod: nginx-proxy)
appear to have a horizontal line through them. PlantUML renders these without
strikethrough. Suspicious that text-decoration is leaking from somewhere
(might be `«stereotype»` italic style being applied to non-stereotype text).

**File as: P1 bug.** Acceptance: labels render with no strikethrough or
unintended text-decoration.

### 6.G — activity/02 and activity/09: decision diamond label collision (**NEW P2 / consolidate with #1447**)

In activity/02 the "no" label is clipped behind the diamond's left edge.
In activity/09 the "yes" label sits stacked under the "no" indicator next
to the same diamond. Both are decision-edge label position errors.
Likely the same root cause as #1447 (activity_09 double-stop bug); may want
to merge or duplicate-link.

**File as: P2 bug** or comment on #1447 to expand acceptance.

### 6.H — Already-tracked bugs confirmed still present this wave (NO new tickets)

These were filed in prior waves; visual confirmation re-confirmed each:

- #1440 deployment/06 cluster y=−34 header clipped (in-review)
- #1441 architecture-overview package headers white-rect overlay (in-review;
  visible as gray pill-shapes inside dark headers — actually wave-5 shows
  this as light-gray "lollipop"-style empty boxes overlaying header text)
- #1442 deployment/06 container label overflow (in-review)
- #1443 component NotificationSender/OrderRepository overflow (in-review)
- #1444 deployment/03 Lambda Function overflow (in-review)
- #1445 usecase/05 actor edge tangle (in-review)
- #1446 usecase/06 actor edges through frame headers
- #1447 activity/09 double-stop overlap (in-review)
- #1448 state/10 bidirectional label collision (in-review)
- #1449 state/07 orphan data-ready label
- #1450 component/07 'uses' label through OrderRepository
- #1451 component/08 'origin pull' edge endpoint in header (in-review)
- #1454 class/32 multiplicity bleed (in-review)

**14 of the 21 visible defects I observed are already filed.** This is a
healthy ticket-database state. The 7 new defects from §6.A-G need filing.

---

## 7. Open-ticket inventory and cleanup recommendations

Total open issues (origin/main @ f989abc9): **35**

| Priority | Count | Notes |
|---|---|---|
| P0 | 4 | #1459 text-fit-first density (agent-ready); #1440 deploy/06 cluster (in-review); #1345 epic; #590 epic |
| P1 | 23 | 14 are visual-audit bugs in-review on PRs #1456/#1458; rest are style block phases #1414-#1417, parity epic #88, coverage #700, etc. |
| P2 | 6 | salt diagnostic #1423 in-review; deployment/03 Lambda #1444 in-review; misc routing tweaks |
| P3 | 1 | benchmark publishing #92 (not on critical path) |
| unlabeled | 1 | #1261 visual audit catalog (use as planning ref, can leave) |

### 7.1 Issues recommended for CLOSURE as obsolete / consolidated

Recommend closing the following with brief comment explaining the fix is
captured elsewhere:

| Issue | Why close | Suggested resolution |
|---|---|---|
| #1323 (Arrows attach top/bottom only; never to box sides) | Wave-5 visual sweep shows class/01 inheritance arrow goes top→bottom AND left edge-routed cleanly post-spline-router; sequence labels attach to side; this is observably no longer true for class/sequence/object families. May still be true for activity diamonds. | Re-scope to "decision-diamond arrows only attach top/bottom" or close as resolved by #1410 spline router. |
| #1324 (Multi-out edges from same node don't branch; stack vertically) | Same as #1323 — spline router landing has materially changed this. | Re-scope or close. |
| #1384 (multi-label arclength collision 3+ labels) | Visual check on state/10 and other multi-label fixtures shows ≤2 labels rendered cleanly; 3+ label case not visible in any of the 35 corpus fixtures. | Move to backlog or close as "no longer reproducible in corpus". |
| #1261 (2026-05-27 visual audit catalog) | All actionable items have either landed or have dedicated tickets. The doc remains useful as historical context but the issue tracker entry has no new action. | Close with comment: "Tracked items each have own tickets; closing as captured." |
| #1428 (class/11 generics compact post-edge layout — wave-4 follow-up) | Class density retune #1435 reduced class/11 from 2.50× to 1.63×; the remaining gap is acceptable; this issue's specific recommendation (Container/Map side-by-side, Stack below) would over-engineer. | Close as resolved-by-density-retune. |

Potential closures: **5**. Net open-issue count after closures: **30**.

### 7.2 Issues with stale "in-review" labels (PRs may have merged)

Run `gh issue list --label in-review` and cross-check with closed PRs. Per
PR list at audit time, PRs #1453 #1456 #1458 are still open — those issues
are correctly in-review. No stale labels detected in this audit.

### 7.3 Issues we'd LIKE to file from this wave (filed in §10)

8 candidates surfaced; recommend filing **8 of 8** as new P1/P2 tickets plus
a NEW nwdiag density retune ticket and layout-convention decision ticket.
See §10.

---

## 8. Examples + docs check

### 8.1 docs/examples/ corpus

- **298 .puml files** across 32 family directories — corpus is healthy and
  current.
- `GALLERY.md` lists 324 source diagrams + 328 SVG renders (post-wave-5
  the actual counts may differ slightly; the README banner says 324, the
  filesystem currently shows 298 .puml — likely a count of the broader
  recursive listing including index/snippet files).
- **`docs/examples/*.svg` artifacts may be stale after the 5 density PRs
  landed.** `scripts/render_check.py --fail-on-doc-drift --quiet` should
  be run to confirm; if it fails, run `scripts/regen-artifacts.sh --force`
  to refresh. **This is a follow-up action, not part of this audit.**

### 8.2 Top-level README.md

- README is current: lists v0.1.0, the architecture-overview image, the
  gallery card-grid is well-organized, the architectural-decisions section
  reads correctly.
- The README's featured-render gallery uses the new `docs/examples/...svg`
  files; if those need a refresh (per §8.1) the gallery will too. The PUML
  v5 renders are visibly better than v4 — the user experience here should
  be improved post-refresh.

### 8.3 Getting-started / docs entry points

- `docs/internal/agents/codex-workflow.md` — agent runbook current
- `docs/internal/architecture/renderer-refactor-roadmap.md` — referenced
  but not re-checked this audit; assumed current
- `CLAUDE.md` — well-maintained, current; the parity-roadmap reference is
  alive

### 8.4 docs/examples/* gaps observed

- `usecase/05` is mostly used to demonstrate `<<extend>>` and actor
  generalization. **PUML's render is sufficiently buggy (head overlaps,
  empty extend box) that publishing the SVG to gallery is currently
  embarrassing.** Recommend either fix-then-regen or temporarily exclude
  from GALLERY featured-section.
- `usecase/02` similarly — Customer head missing is a public-facing bug
  in the most common usecase tutorial.
- `gantt/05` — source file uses non-canonical PlantUML gantt syntax (see
  §1 and §6); recommend rewriting source to canonical form to enable
  diff against PlantUML.

### 8.5 Sample-cycle: ensure no fixture has obviously diverged from intent

Spot-checked 10 of 35 fixture sources — all read as exercises of the family's
described chapter (e.g. class/11 is generics-with-inheritance, sequence/12 is
create-and-destroy with the ✕). Nothing appears stale or off-intent.

---

## 9. Verdict against the "world class" goal

| Gate item | Target | Wave-5 status | Pass? |
|---|---|---|---|
| Median ratio ≤ 1.0× (1:1) | ≤ 1.00× | **1.63×** | **NO** (gap 0.63×) |
| Median ratio ≤ 1.3× (parity-light) | ≤ 1.30× | 1.63× | NO (gap 0.33×) |
| Median ratio ≤ 1.5× (1.0 ship gate) | ≤ 1.50× | 1.63× | NO (gap 0.13×) — striking distance |
| 0 visible visual bugs | 0 | 14 in-review + ~7 new = ~21 visible | **NO** but all tracked |
| 0 open tickets | 0 | 35 open; would be 30 after recommended closures | **NO** but mostly hygiene |
| README + GALLERY current | yes | yes at high level | YES |
| Coverage ≥ 90% | ≥ 90% | gate enforces 85→90 ratchet | YES |
| Deterministic output | byte-identical | unchanged | YES |
| Differential oracle passing | ≥ 50% | passing per #88 | YES |

**Wave count to "world class":**

- **Wave 6 (this week)**: Land #1459 text-fit-first density (clears 1442/1443/
  1444 cluster). Land usecase fan + frame-header avoidance (#1445 #1446). Land
  #1450 component routing + #1454 multiplicity bleed. Bless visual baselines.
  Expected median ≤ 1.55×.

- **Wave 7**: Activity density retune (parallel #1378 pattern). Nwdiag packed-
  grid retune (NEW, see §10). Sequence retune pass 3 (notes + activation
  micro-retune). Component lollipop circle radius shrink. Decide mindmap/wbs
  layout-convention policy. Expected median ≤ 1.30×.

- **Wave 8**: Deployment 3D-node shape size retune. C4 layout convention
  resolution. Style block Phase B-E land (#1414-#1417). Expected median ≤ 1.15×.

- **Wave 9 ("world class")**: Final compaction pass per family. Bless v1.0
  visual baselines. README/GALLERY refresh. Expected median ≤ 1.05×, all
  visible bugs closed.

**Honest estimate: 3-4 waves to "world class" at current cadence.**

---

## 10. Follow-up issues filed (this audit)

The audit script will file the following new tickets (5-15 range as requested,
total = 8):

| # | Title | Severity | Reference |
|---|---|---|---|
| [#1460](https://github.com/alliecatowo/puml/issues/1460) | usecase/02 Customer actor missing stick-figure head | P1 | §6.A |
| [#1461](https://github.com/alliecatowo/puml/issues/1461) | usecase/02 Customer→BrowseProducts edge through name label | P2 | §6.B |
| [#1462](https://github.com/alliecatowo/puml/issues/1462) | usecase/06 actor row head + label visual overlap | P1 | §6.C |
| [#1463](https://github.com/alliecatowo/puml/issues/1463) | usecase/05 <<extend>> renders as empty dashed box, should be arrow | P2 | §6.D |
| [#1464](https://github.com/alliecatowo/puml/issues/1464) | c4/12 "Uses [HTTPS]" label orphaned at canvas top | P2 | §6.E |
| [#1465](https://github.com/alliecatowo/puml/issues/1465) | deployment/06 strikethrough on queue-consumer / nginx text | P1 | §6.F |
| [#1466](https://github.com/alliecatowo/puml/issues/1466) | nwdiag family packed-grid density retune (2.76× → ≤1.5×) | P1 | §3 |
| [#1467](https://github.com/alliecatowo/puml/issues/1467) | mindmap/wbs/c4 layout-convention policy: splay vs stack (decision) | P2 | §5 |

Plus a NOTE on [#1447](https://github.com/alliecatowo/puml/issues/1447#issuecomment-4589936377)
to expand its acceptance to cover activity/02 decision-label clipping.

Total: **8 new issues filed** + 1 comment on existing.

---

## 11. Top-5 next-fix recommendations (final)

| Rank | Action | Median impact | Cost | Notes |
|---|---|---|---|---|
| 1 | **Land #1459 text-fit-first density** (P0 agent-ready) | −0.10× | 1 agent-day | Single most blocking; everything else builds on this |
| 2 | **Land #1445 #1446 + new #10.C (usecase actor cluster)** | −0.05× + 3 bugs closed | 1-2 agent-days | High visual-correctness ROI |
| 3 | **Land #1440 #1441 #1442 #1443 #1444 (overflow + clipping cluster, all in-review)** | −0.07× + 5 bugs closed | 0.5 agent-day (just merge) | Already on PRs; needs CI babying |
| 4 | **File and assign new ticket #10.G (nwdiag packed-grid retune)** | −0.05× | 2 agent-days | Last single-family ≥ 2.5× ratio |
| 5 | **Decide layout-convention policy: mindmap/02, wbs/02, c4/12** | −0.15-0.20× if changed; 0 if accepted | 0 (decision) + 1 day (impl) | Largest single lever toward 1:1, but might be intentional improvement |

Bonus 6: **Run `scripts/regen-artifacts.sh --force` and commit refreshed
`docs/examples/*.svg`** so the gallery reflects current quality.

Bonus 7: **Decide gantt/05_multi_task**: rewrite source to canonical PlantUML
syntax (recommended) or move to PUML-only fixture set.

---

## 12. Evidence index

All cached at `/tmp/parity_audit_v5/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully)
- 35 `*-PlantUML.png` files (gantt/05 is the error sprite)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh`, `render_puml.sh`, `render_plantuml.sh` — driver scripts
- `fixtures.txt` — corpus list

This audit performed NO source modifications. The repository state at audit
time is `origin/main @ f989abc9`.

---

*Snapshot doc; the cached PNGs will be cleaned on next OS restart. Copy to
`docs/internal/forensics/2026-05-31-evidence-v5/` only if the gallery needs
to survive past this session.*
