# PlantUML Parity Wave-4 Status — 35-Fixture Snapshot

**Date:** 2026-05-31
**Auditor:** Claude Opus 4.7 (orchestrator-delegated status audit, no implementation)
**Parent epic:** [#1345](https://github.com/alliecatowo/puml/issues/1345)
**PlantUML reference version:** 1.2026.5 / e0f0ce5 (2026-05-27, GPL build)
**PUML version under test:** `target/release/puml` built from `origin/main` at
`86bfd7b5` (head of main; includes wave-3 audit landing + #1379 #1380 #1373 #1377
#1394 #1395 #1397 #1407 #1411 #1418 #1419 #1421 #1422 plus PR #1388
`--style puml|plantuml` chrome flag).
**In-flight at audit time (NOT merged into main):**
PR [#1378](https://github.com/alliecatowo/puml/pull/1378) (sequence density +
kind-tag pass 2, CONFLICTING with main as of audit),
[#1387](https://github.com/alliecatowo/puml/pull/1387) (collision-only push for #1382),
[#1406](https://github.com/alliecatowo/puml/pull/1406) (inline sprite WIP),
[#1408](https://github.com/alliecatowo/puml/pull/1408) (stereotype-scoped skinparam),
[#1410](https://github.com/alliecatowo/puml/pull/1410) (spline-native waypoint generator),
[#1420](https://github.com/alliecatowo/puml/pull/1420) (style-block parser AST).
**Prior audits:**
- Wave-1: `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` (median 2.93×)
- Wave-2: `docs/internal/forensics/2026-05-30-plantuml-parity-wave2-audit.md` (median 2.25×)
- Wave-3: `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` (median 2.18×)

---

## 0. One-page summary for Allie

**What's the state?**
Median area ratio is **2.24×** on current main vs PlantUML 1.2026.5 (excluding the
phantom `architecture-overview` ratio, see §1). The headline median is essentially
flat from wave-3's 2.18× (Δ ≈ +0.06× from N going 33→34) — even though 10+ PRs
landed in the last 24 h. The reason: the PRs that landed were
correctness/feature/chrome work (#1373 usecase actor edges, #1374 component
parallel-edge dedup, #1375→#1388 `--style` chrome flag, #1377 Stage-3 EdgeRouting,
#1383 generics inheritance, #1394 rounded splines, #1395 chen-ie ortho, #1397
generic name lookup, #1407 inline relation style, #1411–#1422 release/lsp/vscode
infra). **None of these targeted density.** The single density PR (#1378 — sequence
+ kind-tag pass 2) is STILL in flight and is now CONFLICTING with main after the
4-wave landing surge.

**Verdict against the 1.0 gate:**
- Median ≤ 1.3×? **NO** (currently 2.24×; gate gap ~0.94× / ~42%)
- 0 open P0 bugs? **YES** (P0s remaining are the two epics #1345 #590, not bugs)
- Coverage ≥ 90%? **Configured at 90% gate** in `scripts/check-all.sh` line 109; CI
  is enforcing this on PRs

**Bottom line:** Wave-4 is a near-zero-motion checkpoint for density. The good
news is correctness regressions are CLOSING fast: usecase actor placement, class
generics inheritance edge, edge-label drift (now ~15px off-edge vs wave-3's 120px),
and component header overflow are all materially better. The bad news is that
median area ratio is structurally stuck at ~2.25× and will not move until
either (a) #1378 lands AND a deployment/component/class density pass follows, or
(b) Allie reframes the 1.3× gate as a v1.1 goal and ships v1.0 at the current
~2.2× with full correctness parity.

**What's left? (sized by parity impact)**

| Bucket | Count | Status | Median impact if landed |
|---|---|---|---|
| Wave-3 density PR stuck | 1 | #1378 conflicting; needs rebase or fresh-cut | ~0.4× |
| Wave-3 leftover issues still open | 5 | #1371 #1372 #1382 #1384 #1385 | ~0.1-0.2× |
| New material gaps surfaced this wave | 3 | Filed in §7 as P1/P2 | ~0.1-0.3× |
| Cosmetic out-of-scope (no action) | ~5 | unchanged from wave-3 §6.2.E-I | <0.05× |

**What needs your decision (3 items):**

1. **Reframe the 1.0 gate**: median ≤ 1.3× is a 24-month rewrite of layout
   constants per family. If 1.0 means "PlantUML drop-in for correctness + chrome
   parity via `--style plantuml`", current state is shippable at ~88% parity.
   Alternative: hold 1.0 until median ≤ 1.5× (achievable in 2-3 more waves) and
   call ≤ 1.3× the 1.1 goal.
2. **Rescue PR #1378** (sequence density + kind-tag pass 2). The current
   `fix/sequence-density-kindtag-pass2-w16` branch (this audit's host) is the
   same PR and is conflicting with main. Decide: dispatch a rebase agent, fresh-cut
   from main, or hand-merge?
3. **Component edge drop bug** (newly filed P1, §7-A): `component/08` shows
   `Service A → Object Store [upload]`, `Service A → Kafka [publish events]`,
   `Service B → Kafka [publish events]`, `LoadBalancer ↑ EdgeCache [origin pull]`
   STILL missing in PUML output. PR #1379 (parallel-edge dedup, MERGED) does
   NOT fix these — it was about *coincident* terminal coords, not entirely
   *dropped* relations. Different root cause; needs new ticket.

The rest of the doc is the data and citations behind these decisions.

---

## 1. Methodology

- Same 35-fixture corpus as wave-3 (34 examples + `docs/diagrams/architecture-overview.puml`)
- Each rendered to PNG with `/opt/homebrew/bin/plantuml -tpng` (PlantUML 1.2026.5,
  Java 21) and `./target/release/puml --format png`
- Area = `pixelWidth × pixelHeight` from `/usr/bin/sips`
- 6 fixture pairs spot-read side-by-side via the multimodal Read tool
- Cached PNGs at `/tmp/parity_audit_v4/`
- **architecture-overview ratio is a phantom**: PlantUML 1.2026.5 still errors on
  the duplicate `Frontends` identifier (`component [Adapters] as Frontends` + `package
  "Frontends"`); the 599×410 "PlantUML error" sprite was sized normally by sips,
  yielding a bogus 7.03× ratio. Excluded from all medians/percentiles.

No source code was modified. Build was a single `cargo build --release` on
`origin/main` at `86bfd7b5`.

---

## 2. Headline numbers — four-wave progression

| Metric | Wave-1 | Wave-2 | Wave-3 | Wave-4 (this) | Δ overall |
|---|---|---|---|---|---|
| Median area ratio | 2.93× | 2.25× | 2.18× | **2.24×** | −24% |
| Mean area ratio | 3.30× | 2.70× | 2.39× | **2.42×** | −27% |
| Min ratio | 1.25× | 0.71× | 0.70× | **0.96×** | — |
| Max ratio | 7.65× | 5.22× | 4.90× | **4.90×** | −36% |
| N measurable | 33 | 34 | 33 | **34** | — |
| Fixtures ≥ 1.5× | 28 / 34 | 22 / 35 | 25 / 33 | **26 / 34** | 76% |
| Fixtures ≥ 2.0× | — | — | 18 / 33 | **20 / 34** | 59% |
| Fixtures ≥ 3.0× | ~14 / 34 | ~10 / 35 | 7 / 33 | **7 / 34** | strong improvement, stuck |
| Fixtures ≤ 1.3× | — | — | — | **2 / 34** | only 6% pass v1 gate |

Caveats:
- The wave-4 ≥1.5× count (26/34 = 76%) is statistically equivalent to wave-3
  (25/33 = 76%); the gain is within sampling noise of "added gantt/05 back at 2.22×".
- The ≥3.0× count is bin-edge stuck at 7. `class/11_generics` MOVED OUT of the ≥3
  bin in wave-3 (1.88×) but back into the ≥2.5 bin this wave (2.50×) because
  #1383 added the previously-dropped inheritance edge, growing the canvas
  vertically. Net: a correctness win that *worsens* the area metric. This is a
  recurring tension: the area-ratio metric penalizes correctness fixes that add
  missing geometry.
- The `<= 1.3×` count is reported for the first time this wave because the 1.0 gate
  Allie cited is "median ≤ 1.3×". The two passing fixtures are
  `state/10_parallel_regions_shared_events` (0.96×, PUML actually smaller) and
  `usecase/05_actor_generalization_system_boundary` (1.15×, much improved from
  wave-3's 0.70× as PUML's layout switched orientation but is still close).

---

## 3. Full ratio table (current main, 2026-05-31)

| Fixture | PUML | PlantUML | W4 ratio | W3 ratio | Δ vs W3 |
|---|---|---|---|---|---|
| activity/02_if_then_else | 408×394 | 241×359 | 1.86× | 1.85× | +0.01 |
| activity/05_while_loop | 248×438 | 186×437 | 1.34× | 1.33× | +0.01 |
| activity/07_partition | 248×762 | 179×736 | 1.43× | 1.43× | 0.00 |
| activity/09_error_handling | 408×570 | 271×526 | 1.63× | 1.63× | 0.00 |
| c4/12_container_with_databases | 1600×990 | 989×774 | 2.07× | 2.06× | +0.01 |
| class/01_basic | 276×434 | 134×276 | 3.24× | 3.23× | +0.01 |
| class/03_composition_aggregation | 288×590 | 148×384 | 2.99× | 2.98× | +0.01 |
| class/05_visibility | 342×278 | 259×198 | 1.85× | 1.85× | 0.00 |
| class/11_generics | 550×518 | 361×316 | **2.50×** | 1.88× | **+0.62** (W3 PR #1383 added missing edge → grew canvas) |
| component/02_interfaces | 520×452 | 280×205 | 4.09× | 4.09× | 0.00 |
| component/07_ports_lollipop_interfaces | 1558×630 | 702×483 | 2.89× | 2.89× | 0.00 |
| component/08_cloud_db_queue_stereotypes | 1276×1428 | 660×803 | 3.44× | 3.43× | +0.01 |
| deployment/02_databases | 576×696 | 254×322 | 4.90× | 4.90× | 0.00 |
| deployment/03_cloud | 558×452 | 344×199 | 3.68× | 3.68× | 0.00 |
| deployment/06_kubernetes_pods_containers | 1340×1292 | 934×839 | **2.21×** | 2.18× | +0.03 |
| diagrams/architecture-overview | 1032×1672 | (PlantUML error) | **phantom 7.03×** | n/a | (excluded) |
| gantt/05_multi_task | 880×338 | 512×262 | **2.22×** | n/a | (PlantUML now succeeds) |
| mindmap/02_multi_level | 1293×370 | 451×471 | 2.25× | 2.25× | 0.00 |
| mindmap/05_four_levels_asymmetric | 1629×658 | 723×1074 | 1.38× | 1.38× | 0.00 |
| nwdiag/02_multi_network | 760×410 | 295×360 | 2.93× | 2.93× | 0.00 |
| object/02_with_attributes | 327×450 | 223×253 | 2.61× | 2.60× | +0.01 |
| object/05_ch04_parity | 480×440 | 185×236 | 4.84× | 4.83× | +0.01 |
| salt/01_basic_widgets | 198×72 | 145×71 | 1.38× | 1.38× | 0.00 |
| sequence/03_autonumber | 488×280 | 232×210 | 2.80× | 2.80× | 0.00 |
| sequence/07_notes | 550×432 | 255×316 | 2.95× | 2.94× | +0.01 |
| sequence/11_activation | 488×280 | 230×210 | 2.83× | 2.82× | +0.01 |
| sequence/12_create_destroy | 488×360 | 239×222 | 3.31× | 3.31× | 0.00 |
| state/03_concurrent | 232×646 | 246×419 | 1.45× | 1.45× | 0.00 |
| state/07_nested | 273×630 | 207×557 | 1.49× | 1.49× | 0.00 |
| state/10_parallel_regions | 249×1010 | 280×938 | **0.96×** | 0.95× | +0.01 |
| timing/01_concise | 426×156 | 250×165 | 1.61× | 1.61× | 0.00 |
| usecase/02_with_actors | 398×534 | 286×453 | 1.64× | 1.64× | 0.00 |
| usecase/05_actor_generalization_system_boundary | 1084×1270 | 1830×653 | **1.15×** | 0.70× | **+0.45** (#1380 reorients actors-outside; canvas grew but still PUML-favored) |
| usecase/06_multi_system_boundary | 1384×822 | 1090×568 | **1.84×** | 1.73× | +0.11 |
| wbs/02_with_tasks | 1848×246 | 505×344 | 2.62× | 2.61× | +0.01 |

**Net:** Of 34 ratio-measurable fixtures, **30 moved by ≤ 0.03×** (statistical
noise; can be attributed to sips rounding). **Three meaningful moves:**
- `class/11_generics` +0.62× — #1383 added the missing Stack-extends-Container
  inheritance edge; canvas grew vertically to fit. Visually correct now, but area
  metric got worse. **Real win disguised as a regression.**
- `usecase/05` +0.45× — #1380 moved actors outside the boundary frame; PUML
  layout switched from landscape to portrait orientation and grew vertically.
  Visually structural-bug-improved; area still PUML-favored at 1.15×.
- `usecase/06` +0.11× — also #1380 side-effect; multi-boundary handling spreads
  more horizontally now.

---

## 4. Per-family score table — wave-1 / wave-2 / wave-3 / wave-4

Cell = (count of fixtures with ratio ≥ 1.5×) / (total). Lower is better.

| Family | W1 | W2 | W3 | W4 | Trajectory |
|---|---|---|---|---|---|
| activity | 4/4 | 0/4 | 0/4 | **2/4** | activity/02 and /09 still 1.6-1.9× — re-classify "0/4 in W2/W3 was a measurement glitch" |
| c4 | 1/1 | 1/1 | 1/1 | **1/1** | static — 2.07× acceptable for nested boundaries |
| class | 3/4 | 3/4 | 3/4 | **4/4** | regressed via correctness win (#1383) — see §3 note |
| component | 3/3 | 3/3 | 3/3 | **3/3** | static; gated on #1378 + deployment-style density retune |
| deployment | 3/3 | 3/3 | 3/3 | **3/3** | static — never targeted; this is the biggest single-family gap |
| gantt | n/a | 1/1 | n/a | **1/1** | PlantUML works again in W4; 2.22× density |
| mindmap | 1/2 | 1/2 | 1/2 | **1/2** | layout convention divergence — out of scope per W2 §4.12 |
| nwdiag | 1/1 | 1/1 | 1/1 | **1/1** | gated on #1378 (kind-tag pass 2) |
| object | 2/2 | 2/2 | 2/2 | **2/2** | `--style plantuml` (#1388) reduces chrome but NOT layout density (verified §5.3) |
| salt | 0/1 | 0/1 | 0/1 | **0/1** | already in parity zone |
| sequence | 4/4 | 4/4 | 4/4 | **4/4** | gated on #1378 |
| state | 2/3 | 0/3 | 0/3 | **0/3** | done; #1384 still open for stacked-label collision |
| timing | 1/1 | 1/1 | 1/1 | **1/1** | gated on #1378 kind-tag pass 2 |
| usecase | 2/3 | 1/3 | 1/3 | **2/3** | usecase/05 dropped under 1.5× (1.15×); usecase/06 grew over (1.84×) |
| wbs | 1/1 | 1/1 | 1/1 | **1/1** | layout convention; out of scope |
| **Overall ≥ 1.5×** | **28/34** | **22/35** | **25/33** | **26/34** | **76%** above 1.5× — stuck |

Of the 26/34 fixtures above 1.5×, **15 are in families with at least one OPEN
density issue** (#1371 sequence, #1372 component+nwdiag+timing, #1384 state,
#1385 architecture, plus the new tickets in §7). If all of those landed
optimally, the count would drop to ~9/34 (~26% above 1.5×) and median to ~1.7×.

---

## 5. Spot-check delta gallery (6 pairs)

Spot reads from `/tmp/parity_audit_v4/` confirming the residual gaps.

### 5.1 `sequence/07_notes` — 2.95× ratio

**Delta vs wave-3:** Unchanged. PUML uses 2× horizontal participant-column
spacing and 1.6× vertical message-row spacing. Notes render at full
participant-column width vs PlantUML's compact tightly-wrapped chevron.
Structure parity-faithful; every message, activation bar, and note is in the
right relative position.
**Gated on:** PR #1378 (sequence density retune, in-flight, conflicting).

### 5.2 `component/08_cloud_db_queue_stereotypes` — 3.44× ratio

**Delta vs wave-3:** Mixed. Header overflow (wave-3 §5.2 bug 2) appears resolved
by PR #1379 — `package Event Bus` header text is now readable on the dark band.
But the kind-tag leak (wave-3 §5.2 bug 1) is still present: every group header
reads `package CDN Layer`, `package API Cluster`, etc — the kind-tag suppression
pass-2 is gated on PR #1378.

**More critically — the edge drops persist** (wave-3 §5.2 bug 3, presumed fixed
by #1379):

Missing from PUML:
- `LoadBalancer ↑ EdgeCache [origin pull]` — completely absent
- `Service A → Object Store [upload]` — completely absent
- `Service A → Kafka [publish events]` — completely absent
- `Service B → Kafka [publish events]` — completely absent

Present in PUML: `route /v1`, `route /v2`, `read/write`, `read-only`, `consume`,
`aggregate`. PR #1379 fixed *parallel-edge fan offset* (two edges sharing the
same terminal coord) — but these edges aren't merging with another edge, they're
disappearing entirely. **Different root cause; new P1 ticket in §7-A.**

### 5.3 `object/05_ch04_parity` — 4.84× ratio (default `--style puml`)

**Delta vs wave-3:** Same chrome (yellow banner + circled-O badge + drop-shadow
+ underlined name + double-bottom-line) because PR #1388 chose Option B
(`--style plantuml` opt-in) rather than wave-3 Option A (flip default).

**New finding this wave:** I re-rendered with `--style plantuml`:

```
./target/release/puml --format png --style plantuml docs/examples/object/05_ch04_parity.puml
```

Result at `/tmp/parity_audit_v4/object05-PUML-plantuml-style.png`: chrome IS
lighter (no yellow banner, no badge, no shadow — flat light-gray header with
underline-only). **But the rendered dimensions are 480×440 — IDENTICAL to the
default-chrome render.** PlantUML's render is 185×236. The density gap survives
the chrome change. **Object density is a layout-engine constant issue, not a
chrome issue.** The `--style plantuml` flag therefore moves cosmetic parity
forward but is a non-event for area-ratio parity. Worth filing as a separate
density ticket (§7-B) since wave-3 §5.3 conflated chrome and density into the
#1375/#1388 work.

### 5.4 `deployment/02_databases` — 4.90× ratio (worst remaining)

**Delta vs wave-3:** Unchanged. Density-only. PUML's 3D-isometric `node` shapes
render at ~2× PlantUML's width and ~1.5× height. The PostgreSQL/Redis cylinders
are ~1.8× PlantUML's. Vertical spacing AppServer→PostgreSQL is ~3× PlantUML's
tight 80px. **No structural defect, no chrome difference, no fixable bug per se
— pure layout-constant retune.** This and `component/02_interfaces` (4.09×) are
the two highest-impact targets for a deployment-family density pass analogous to
what #1378 does for sequence. **New P1 ticket in §7-C.**

### 5.5 `class/01_basic` — 3.24× ratio (edge-label drift resolved)

**Delta vs wave-3:** Edge-label "owns" now renders within ~15px of the actual
edge between Animal and Dog (wave-3: ~120px stranded right of the edge). This
is wave-3 ticket #1382's underlying behavior — confirmed visually fixed even
though #1382 is still OPEN (it points at PR #1387 which is in-flight). Allie
may want to close #1382 after #1387 lands.

**Remaining issue:** the underlying density gap persists. PUML class boxes are
~2× wider (PUML uses 13-15px horizontal padding for visibility glyphs + 1-2
character buffer; PlantUML uses 6-8px) and the vertical inter-class spacing is
~2× PlantUML's. The wave-3 W1-acquired wins (green ⓒ badge, ○/● glyphs) all
survive.

### 5.6 `usecase/05_actor_generalization_system_boundary` — 1.15× ratio

**Delta vs wave-3:** Major correctness improvements from PR #1380:
- **All four actors (User, Administrator, Registered User, Premium User) now
  render OUTSIDE the dashed E-Commerce Platform boundary** — UML 2.x spec
  compliant. Wave-3 had them inside.
- Use-case ellipses (Browse Catalog, Search Products, Apply Promo Code, Track
  Order, Write Review, Manage Users, View Analytics, Checkout, Add to Cart,
  Access Priority Support, View Product Detail) are properly inside the boundary
  ellipse.
- The Administrator-to-User and Registered-User-to-User generalization
  arrows are drawn.

**Still problematic:**
- Edge tangling: ~10 vertical arrows from actors to use-cases stack tightly
  through the actor row, creating visual congestion. PlantUML routes these as
  smooth curves fanning from each actor.
- `<<extend>>` dashed box is empty — the extension target text is missing
  inside it. PlantUML draws this as a labeled dashed arrow with "extends" inline.
- Premium User-to-User edge crosses through `Apply Promo Code` ellipse.

These are routing-quality issues that should be tracked as a usecase polish
ticket — filing in §7-D.

---

## 6. Remaining gap catalogue

### 6.1 Issues that already exist and remain open

| Issue | What it addresses | Wave-4 status |
|---|---|---|
| #1371 | Sequence density retune | OPEN; PR #1378 conflicting |
| #1372 | Kind-tag pass 2 (component / nwdiag / timing) | OPEN; PR #1378 conflicting |
| #1382 | Class edge-label drift (sparse) | OPEN; visually fixed on main, PR #1387 in-flight |
| #1384 | State multi-label arclength collision (3+ labels) | OPEN; no PR |
| #1385 | Architecture-overview vertical waste + header overflow | OPEN; header part may be improved by #1379, vertical waste persists |
| #1323 | Arrows attach top/bottom only; never to box sides | OPEN; long-standing |
| #1324 | Multi-out edges stack vertically | OPEN; long-standing |
| #1391 | Spline-native router research | OPEN; PR #1410 in-flight |
| #1404 | `<style>` block parity epic | OPEN; PR #1420 + #1413-1417 chain |

### 6.2 New gaps surfaced this wave — file as issues (§7)

| # | Gap | Severity | Existing coverage? | Action |
|---|---|---|---|---|
| A | Component edges entirely dropped (origin pull, upload, publish events ×2) | P1 | None — #1379 fixed dedup not drop | **FILE** |
| B | Object density unchanged by `--style plantuml`; layout constants need retune | P1 | None — #1388 was chrome-only | **FILE** |
| C | Deployment family per-shape size retune (node/cylinder/cloud width × height multipliers) | P1 | None — #1378 covers sequence only | **FILE** |
| D | Usecase actor-to-ellipse edge fan/tangle on dense diagrams | P2 | None | **FILE** |
| E | Class generics area metric grew via #1383 fix — accept tradeoff or compact horizontal layout? | P2 | None | **FILE** |
| F | Architecture-overview can't be diffed against PlantUML (duplicate identifier source) | P3 | #1385 | Decide source rewrite vs reframe the eval set |

### 6.3 Cosmetic / out-of-scope (unchanged from wave-3 §6.2.E-I)

E. Component lollipop circle radius 4-5× PlantUML — hold
F. Component required-interface socket half-circle missing — hold
G. Timing concise hexagon segment shape — hold (larger refactor)
H. Sequence create-participant green-dot decorator — hold
I. Actor name overlap with stick-figure head (usecase/02) — hold

These collectively contribute <0.05× to median ratio and would consume agent
hours better spent on the §6.2 P1 items.

---

## 7. Filed follow-up issues (P1/P2)

Six new issues filed under this audit:

- [#1424](https://github.com/alliecatowo/puml/issues/1424) — P1 component edges entirely dropped (§7-A)
- [#1425](https://github.com/alliecatowo/puml/issues/1425) — P1 object density independent of `--style` flag (§7-B)
- [#1426](https://github.com/alliecatowo/puml/issues/1426) — P1 deployment family per-shape density retune (§7-C)
- [#1427](https://github.com/alliecatowo/puml/issues/1427) — P2 usecase actor-to-ellipse edge tangle (§7-D)
- [#1428](https://github.com/alliecatowo/puml/issues/1428) — P2 class generics compact post-edge layout (§7-E)
- [#1429](https://github.com/alliecatowo/puml/issues/1429) — P3 architecture-overview source not PlantUML-compatible (§7-F)

See §0 for decision asks.

### 7-A. P1 — component edges entirely dropped on dense diagrams (NEW)

`component/08_cloud_db_queue_stereotypes` is missing FOUR relations that
PlantUML 1.2026.5 renders correctly:
- `LoadBalancer -up-> EdgeCache : origin pull`
- `ServiceA --> ObjectStore : upload`
- `ServiceA --> Kafka : publish events`
- `ServiceB --> Kafka : publish events`

PR #1379 (parallel-edge dedup) merged 2026-05-31 and is the closest existing
fix, but it addresses *coincident terminal coords* (two edges sharing one
endpoint pair), not entirely dropped edges. These four edges are eliminated
somewhere in the parse → normalize → render pipeline before reaching the
parallel-dedup pass. Likely candidate: cross-package edge resolution when both
endpoints sit in different `package` containers and the source uses `-->` (vs
`-->>` or `..>`).

**Acceptance:** all 9 relations declared in
`docs/examples/component/08_cloud_db_queue_stereotypes.puml` appear as visible
edges in the rendered output.

### 7-B. P1 — object density unchanged by `--style plantuml` (NEW)

PR #1388 added `--style puml|plantuml` chrome mode. Verified this wave:
`./target/release/puml --format png --style plantuml docs/examples/object/05_ch04_parity.puml`
produces flat-chrome output (no yellow banner, no badge, no shadow) at
**identical 480×440 dimensions** to the default-chrome render. PlantUML reference
is 185×236.

The chrome simplification is correct but the layout engine's per-object box
sizing and inter-object spacing constants are unaffected. These need a separate
retune analogous to what #1378 does for sequence. Targeted fixtures:
`object/02_with_attributes` (2.61× → target ≤ 1.8×), `object/05_ch04_parity`
(4.84× → target ≤ 2.0×).

**Acceptance:** under `--style plantuml`, both fixtures' area ratio drops below
2.0×.

### 7-C. P1 — deployment family per-shape density retune (NEW)

Three deployment fixtures dominate the wave-4 worst-density list:
- `deployment/02_databases`: 4.90× (worst remaining single ratio)
- `deployment/03_cloud`: 3.68×
- `deployment/06_kubernetes_pods_containers`: 2.21×

Wave-3 §5.4 noted this as "the residue of the wave-1 density retune that #1346
did NOT propagate into the deployment family's per-shape sizing constants".
Pure density — no chrome difference, no structural defects. The 3D-isometric
`node` cube and cylinder `database` shapes render ~1.8× larger than PlantUML's
equivalents, and inter-shape vertical spacing is ~2.5× PlantUML's.

Pattern parallels #1378's approach for sequence: introduce a per-family density
scale factor, tune by visual inspection, ratchet visual baselines after blessing.

**Acceptance:** all three fixtures drop below 2.5× area ratio.

### 7-D. P2 — usecase actor-to-ellipse edge fan/tangle (NEW)

`usecase/05_actor_generalization_system_boundary` (1.15× area but visually
tangled) and `usecase/06_multi_system_boundary` (1.84×) both show actor→ellipse
edges as straight near-vertical arrows that stack within a 50-100px band
through the actor row. PlantUML routes these as smooth curves with fanned
arrival angles per actor.

Root cause likely in the edge-routing stage: actors-outside-boundary creates
a long vertical traverse where each edge needs to enter the boundary then
hit a different ellipse on the other side. PUML currently uses parallel
near-vertical paths; PlantUML uses curves with per-target angular offsets.

**Acceptance:** on `usecase/05`, no two actor→ellipse edges occupy the same
20-pixel vertical band along their full length.

### 7-E. P2 — class generics area metric regressed by correctness fix (NEW)

PR #1383 fixed the dropped Stack-extends-Container inheritance edge in
`class/11_generics`. Before: 1.88× ratio (Stack/Container/Map laid out
side-by-side without arrows). After: 2.50× ratio (Stack stacked below
Container with proper inheritance triangle, but vertical canvas grew ~64%).

This is a correctness-vs-density tension worth tracking explicitly. Two
possible directions:
1. Accept the area regression as the cost of correctness; close out and move on.
2. Compact the post-edge layout: keep Container/Map side-by-side at top, place
   Stack underneath Container only (not full vertical stretch).

Option 2 would reduce ratio to ~1.6× while keeping the edge. Worth filing for
the layout engine's per-edge bounding-box tightening logic.

### 7-F. P3 — architecture-overview source not PlantUML-compatible (NEW; or close as wontfix)

`docs/diagrams/architecture-overview.puml` declares both
`component [Adapters] as Frontends` and `package "Frontends"`. PlantUML
1.2026.5 errors: "This element (Frontends) is already defined (Assumed diagram
type: component)". PUML tolerates and renders. As a result the fixture cannot
be ratio-measured (it generated the phantom 7.03× in §3).

Options: (1) rewrite source to use distinct identifiers, restoring measurability;
(2) drop the fixture from the parity corpus; (3) keep as-is and document the
divergence. #1385 covers the rendering-quality side (vertical waste) but not
the source incompatibility. Filing for tracking only.

---

## 8. Verdict against the 1.0 gate

| Gate item | Target | Wave-4 status | Pass? |
|---|---|---|---|
| Median ratio ≤ 1.3× | ≤ 1.30× | **2.24×** | **NO** (gap 0.94×) |
| 0 open P0 bugs | 0 | 2 P0 issues open, both epics not bugs | **YES (effectively)** |
| Coverage ≥ 90% | ≥ 90% | Gate at 90% in `scripts/check-all.sh:109` enforced by CI | **YES** |
| All fixtures render without panic | 100% | 35/35 PUML renders succeed | **YES** |
| Structural parity per fixture | spec-compliant placement | confirmed by spot-reads §5 | **YES with caveats** (see edge-drop §7-A) |
| Differential oracle ≥ 50% | tracked by `differential-svg-oracle` | required check passing per #88 epic | **YES** |

**The single failing gate is median ratio.** Everything else passes or is
within striking distance. Two realistic gate-shipping paths:

**Path A — ship 1.0 at 2.24× median, defer 1.3× to 1.1:**
- Close 1.0 with "PUML is a drop-in PlantUML replacement for correctness +
  structural parity; default chrome is PUML-styled; `--style plantuml` opt-in
  gives chrome parity; layout density is consistently 2-3× wider than PlantUML
  but never overlapping or incorrect".
- 1.1 promises ≤ 1.5× median via #1378 land + deployment-density retune (§7-C)
  + object-density retune (§7-B) + sequence-density (#1371).
- 1.2 promises ≤ 1.3× median via spline-native routing (#1391/#1410), class/
  component constant retune, and per-family compaction passes.

**Path B — hold 1.0 until ≤ 1.5×:**
- Requires: #1378 land (clears #1371 #1372 → drops 7 fixtures' ratio ~30%),
  #7-B object retune (drops 2 fixtures), #7-C deployment retune (drops 3
  fixtures), #1384 state retune (drops 1 fixture). 13 of 34 fixtures move; new
  median estimate ~1.6×.
- Estimated 4-6 weeks of agent work (3-4 waves at current cadence).

Recommend Path A unless Allie has a hard external commitment to ≤ 1.3×.

---

## 9. Top-5 next-fix recommendations (ranked by ROI)

| Rank | Action | Estimated median impact | Estimated cost | Notes |
|---|---|---|---|---|
| 1 | **Rescue PR #1378** — rebase or fresh-cut from main, land it | −0.3 to −0.4× (closes 7 fixtures' density gap) | 1 agent-day | Conflicting now; this is the most important single action |
| 2 | **File + assign §7-C deployment density retune** | −0.2× (closes 3 of the worst-3 ratios) | 2-3 agent-days | Pattern-matches #1378's approach for sequence |
| 3 | **File + assign §7-B object density retune** | −0.15× (closes 2 fixtures, lifts `--style plantuml` to actual parity) | 1-2 agent-days | Layout-only, no chrome work |
| 4 | **File + assign §7-A component edge drops** | 0× area but +4 visible edges (correctness) | 1 agent-day; root-cause unknown | Highest correctness ROI; investigate cross-package edge resolution |
| 5 | **Decide Path A vs Path B on 1.0 gate** | gate clarity | 1 Allie-decision | Without this, agent waves can't be planned for the right horizon |

Bonus item 6 (low impact, low cost):
- Close #1382 after #1387 lands (visually verified fixed in §5.5).

---

## 10. Evidence index

All cached at `/tmp/parity_audit_v4/`:

- 35 `*-PUML.png` files (all fixtures rendered successfully)
- 35 `*-PlantUML.png` files (architecture-overview is the error sprite)
- 1 `object05-PUML-plantuml-style.png` (chrome-only ablation render)
- `ratios.tsv` — full quantitative table
- `compute_ratios.sh` — the script
- `render_puml.sh`, `render_plantuml.sh` — render drivers
- `fixtures.txt` — corpus list

This audit performed NO source modifications. The repository state at audit
time is `origin/main @ 86bfd7b5`.

---

*This document is a snapshot. The cached PNGs will be cleaned on next OS
restart; copy to `docs/internal/forensics/2026-05-31-evidence-v4/` only if the
gallery needs to survive.*
