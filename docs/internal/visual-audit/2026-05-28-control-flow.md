# Control-flow visual audit — 2026-05-28

**Auditor:** Opus visual-audit agent (control-flow bucket: sequence + activity + state + timing)
**Corpus:** `target/audit_corpus/png/` regenerated 2026-05-28
**Method:** Selected the largest-by-bytes PNGs per family (proxy for "most exercising of the family's grammar"), read each as raster via the `Read` tool, compared to the upstream `.puml` source and the upstream PlantUML reference behavior.

13 fixtures audited. Findings grouped by family. Severity legend:

- **P0** — blocks parity (wrong topology, missing nodes, edges through nodes, swimlanes nesting incorrectly)
- **P1** — clearly visible regression (label far from edge, redundant subtitles, unmerged contiguous timing segments)
- **P2** — cosmetic

---

## Cross-cutting findings (apply to many fixtures)

These keep appearing across the control-flow families, so I'm logging them once up top instead of repeating per-fixture:

- **C1 — Stray `<family> diagram` subtitle below title.** Every activity and timing fixture renders a small grey `"activity diagram"` or `"timing diagram"` subtitle line in the upper-left, just below the title. The source files do not request a subtitle. Severity **P1**.
- **C2 — `concise` / `robust` / `clock` track-kind hint rendered as small grey text under each track label** in timing diagrams. PlantUML does not render the track-kind keyword under the participant label. Severity **P2**.

---

## docs/examples/sequence/15_large_diagram.puml.png

**What I see:** E-Commerce Order Flow with 7 participants (Customer actor, Web App, Order Service, Inventory, Payment, Order DB, Email Queue). Three nested alt groups: `alt in stock` outer, `alt payment ok / else payment failed` inner, then `else out of stock` outer. 18 autonumbered messages. Lifelines extend top to bottom and headers are repeated at top/bottom.

**Expected:** Outer `alt` with two top-level branches (`in stock` / `out of stock`), and within `in stock` an inner `alt` with two branches (`payment ok` / `payment failed`). The frame headers should reflect that nesting structure visually (inner frame inside outer frame).

**Issues:**
- [ ] The frame nesting renders as **three sibling frames stacked vertically** ("alt in stock", "alt payment ok", and the bottom "out of stock" frame) rather than a nested inner-alt-inside-outer-alt visual. The `else payment failed` and `else out of stock` divider labels both render as compact dashed dividers, but the parent-child grouping is lost — a reader cannot tell which `else` belongs to which `alt`.
- [ ] Message #12 ("order confirmed") arrowhead sits **immediately above the `else payment failed` divider**, with no padding between the arrow tail and the divider line.
- [ ] No activation bars on Order Service / Payment despite the deep call chain (cosmetic).

**Suggested fix scope:** `src/render/sequence/` — frame nesting & inner-frame indentation; alt/else divider needs to inherit parent frame depth so nested alts render visually nested.

**Severity:** P1

---

## docs/examples/sequence/48_complex_ref_over_multibox.puml.png

**What I see:** Three colored `box` regions: Client Tier (blue), API Tier (green), Data Tier (yellow). Custom autonumber format `[NN]`. A `group Authentication` wrapping a `ref over Browser, Gateway, AuthSvc` block. `autonumber stop` then `note over Gateway: Token validated` then `autonumber resume`. An `alt cache hit / else cache miss`. Final messages back to User.

**Expected:** ref-over-multibox should span the indicated participant range as a labeled frame. autonumber stop should pause numbering for the note. Cache hit/miss arms should be clearly distinguished.

**Issues:**
- [ ] The `ref over` frame **only spans Browser through Gateway visually**, not Browser/Gateway/AuthSvc. The frame's right edge ends at the Gateway column even though the source explicitly lists AuthSvc as the third participant. The ref label "OAuth2 PKCE handshake" sits inside a frame that doesn't include AuthSvc.
- [ ] The `note over Gateway: Token validated` renders correctly as a small yellow sticky note, but it overlaps with the autonumber resume gap — message [03] starts immediately to the right of the note with no vertical breathing room.
- [ ] Inside `alt cache hit`, the `else cache miss` divider works correctly. Good rendering otherwise.

**Suggested fix scope:** `src/render/sequence/refs.rs` or wherever `ref over <p1, p2, ..., pN>` width is computed — the right-edge participant index is being clamped one too short.

**Severity:** P0 (ref-over span is a parity bug)

---

## docs/examples/sequence/17_all_groups.puml.png

**What I see:** "All Group Types" — sequence with 4 participants (Alice, Bob, Charlie, DB). Stacked groups in order: alt success/failure, opt optional step, loop retry 3 times, par parallel (with else), critical critical section, break on error, group custom label. Each group is its own rectangle.

**Expected:** All groups should sit on a continuous set of lifelines that run from top to bottom without breaks.

**Issues:**
- [ ] **Lifelines visually break between `par parallel` and `critical critical section`** — there is a clear vertical gap with no dashed lifeline crossing it. The four lifeline ticks are visible above and below the gap, but the four dashed lines do not continue through. Looks like a frame did not extend its bottom guide.
- [ ] In the `break on error` group, the self-arrow `Alice -> Alice: abort` renders with a tight rectangular self-loop with the arrowhead pointing left into Alice's lifeline. The label "abort" sits inside the loop. Acceptable.
- [ ] The `critical critical section` arrow `Bob -> DB: update` is correctly rendered.

**Suggested fix scope:** `src/render/sequence/frame.rs` — lifeline continuity across consecutive frame transitions. Probable cause: after one frame closes, the next frame's top edge is computed at a y-coordinate higher than the previous frame's bottom edge, leaving an unfilled vertical band with no lifeline ticks drawn into it.

**Severity:** P1

---

## docs/examples/sequence/46_nested_alt_with_par.puml.png

**What I see:** User -> Browser -> Auth Service -> Session DB. Activation `activate A` triggered after POST /login. Triple-arm alt: `credentials valid` / `else invalid password` / `else account locked`. Within `credentials valid` branch: `activate DB` then `deactivate DB`. Final `deactivate A`.

**Expected:** Activation bar on Auth Service should run from the `activate A` line down through the entire alt block (since `deactivate A` is after `end`). Activation bar on Session DB inside the first arm should run only during `create session` -> `sessionId`.

**Issues:**
- [ ] **The activation bar on Auth Service is only a tiny 2-line stub** at the top of the alt block where `activate A` was issued. It does NOT extend down through the alt block to the matching `deactivate A`. Per PlantUML semantics, `activate A` opens an activation that should persist until `deactivate A`, drawn as a tall thin rectangle on the lifeline through all three alt arms.
- [ ] **No activation bar visible on Session DB at all**, despite `activate DB` / `deactivate DB` being correctly paired in the source.
- [ ] The three-arm alt renders correctly with `else invalid password` and `else account locked` dividers.

**Suggested fix scope:** `src/render/sequence/activation.rs` — activation lifetime should not be terminated by a frame boundary. Currently it appears to be auto-deactivated at the start of the first nested frame.

**Severity:** P0 (activation bars are core sequence diagram semantics)

---

## docs/examples/activity/18_repeat_while_nested_partition.puml.png

**What I see:** Three partitions stacked vertically (Extract / Transform / Load). Extract contains a repeat-while with diamond `more rows?`. Transform contains a while loop with diamond `records in queue?`. Load contains a fork with three branches (Write primary table / Update search index / Publish CDC events) joining to Commit transaction.

**Expected:** Each partition's frame should fully enclose its activities. Fork/join bars should be sized to span only the parallel branches inside the partition, not stretch beyond the partition's left/right walls.

**Issues:**
- [ ] **The fork bars in the `Load` partition extend FAR outside the partition's left and right walls** — the bars span nearly the full diagram width (~960px), while the partition's dashed frame is much narrower. Write primary table is rendered OUTSIDE the Load partition on the left; Publish CDC events is OUTSIDE on the right.
- [ ] **The Load partition's dashed frame is clipped** — its right edge ends before the fork bar's right tip. The partition appears to wrap only Open destination / Commit transaction / Close destination, with the entire fork visually punching through and out the sides.
- [ ] On the Extract repeat-while: the loop label `yes / no` is rendered as a single combined text fragment positioned to the right of the loopback edge, not as separate `yes` (on the back-edge) and `no` (on the exit-edge). Compare to the Transform `while`, which DOES use separate `yes` / `no` labels (still oddly placed but separate). The repeat-while loop label format is inconsistent with the while-do loop label format.
- [ ] **Subtitle** `activity diagram` clutter (cross-cutting C1).

**Suggested fix scope:** `src/render/activity/partition.rs` + `fork.rs` — partition width must be max(content width, fork-bar width). Fork bar layout currently uses a fixed wide width independent of partition bounds.

**Severity:** P0 (partition + fork interaction breaks visually)

---

## docs/examples/activity/10_authentication.puml.png

**What I see:** Three-deep nested if/else with branches: Username valid -> Password match -> MFA enabled -> OTP valid. The deepest then/else paths join back to a single stop. The diagram occupies maybe 50% of the width and 30% of the height; the rest is empty padding. Multiple edges visible crossing each other to converge on the bottom stop.

**Expected:** A clean if/else tree where each diamond's branches converge cleanly at merge nodes, then the whole tree converges to one stop. No edges through nodes.

**Issues:**
- [ ] **Multiple edges pass directly through node rectangles**. The "Max attempts?" diamond has an edge running through its body; the "Grant Access" (center) node has an edge crossing through it horizontally. The "Lock Account" node has an outgoing line cutting through "Deny + Log".
- [ ] **"yes" label sits on top of the "Grant Access" node** — the `MFA enabled? -> no -> Grant Access` edge labels its "no" outside the box, but the `Password match? -> yes -> ... -> Grant Access` flow labels "yes" landing inside the Grant Access rectangle.
- [ ] **Massive empty whitespace below the flow** — the rendered content occupies roughly the top half; the stop bullseye is centered at the very bottom with no nearby content. Suggests the layout reserved space for a much taller tree that never materialized.
- [ ] **The merge points before stop are invisible** — there should be diamond merge nodes (or implicit merges) where branches converge; instead they're rendered as edge crossings without explicit merge symbols.
- [ ] **Subtitle** `activity diagram` (C1).

**Suggested fix scope:** `src/render/graph_layout.rs` — orthogonal edge routing must avoid node rectangles; multi-branch convergence to a single sink needs explicit merge-node insertion or much smarter edge bundling. This fixture should be a benchmark case for the layout engine.

**Severity:** P0

---

## docs/examples/activity/16_nested_swimlanes_parallel_forks.puml.png

**What I see:** Source defines 4 swimlanes (Customer | Warehouse | Finance | Logistics) used in an order/payment/shipping flow with two `fork` blocks and a `detach`. Rendered diagram: Customer lane is wide and tall on the left; Warehouse header appears inside the Customer lane vertical space; Finance header appears as a separate horizontal bar; Logistics is positioned on the right side. The first fork's bar extends across Warehouse + Finance lanes. The second fork bar (after Print label) extends from far-left to Logistics column.

**Expected:** Swimlanes should be rendered as **vertical columns side-by-side** with each activity placed inside its owning lane's column. Fork bars should span only the columns containing forked branches. `detach` should terminate a branch with no outgoing edge (no stub).

**Issues:**
- [ ] **Swimlanes do not render as side-by-side columns.** Instead they appear nested/stacked: Customer occupies the full left column, Warehouse appears as a header bar in the middle inside Customer's region, Finance appears as another header bar further down, Logistics is a separate column on the right. This is fundamentally wrong for swimlanes.
- [ ] **The first fork** renders `Pick items / Pack items` (Warehouse branch) and `Charge payment / payment ok? / Issue receipt / Notify customer` (Finance branch) as two columns inside the Finance lane region, with the bar spanning both. The Warehouse branch should be in the Warehouse column.
- [ ] **`payment ok? -> yes -> Issue receipt` and `payment ok? -> no -> Notify customer`** render with both Issue receipt AND Notify customer **on the same outgoing edge** (Notify customer below Issue receipt, both descending vertically from the diamond), as if they're sequential instead of branches. The diamond only labels one branch ("no"), missing the "yes" label.
- [ ] **`detach` after Notify customer renders as a small dangling line stub** below the node, not as a clean termination (no outgoing arrow at all). The stub looks like a visual orphan.
- [ ] **Second fork's join bar extends from x=0 (left edge) to the Logistics column**, even though both forked branches live inside Logistics + Warehouse. The bar is way too wide and pokes outside the active region.
- [ ] **Customer lane has a huge vertical empty band** below Submit order, occupying ~40% of the diagram height with nothing in it.
- [ ] **Subtitle** `activity diagram` (C1).

**Suggested fix scope:** `src/render/activity/swimlanes.rs` — full architectural rework needed. Swimlanes must be assigned distinct horizontal columns; activity placement must respect its owning `|Lane|` directive at the time of declaration; fork branches need swimlane-aware column routing.

**Severity:** P0 (swimlanes essentially broken; this is a major parity gap)

---

## docs/examples/activity/13_user_registration.puml.png

**What I see:** Nested if/else (Valid? -> Unique? -> Confirmed?) with branches feeding into Activate Account / Resend Email / Show Error / Show Validation Errors. Multiple edges crossing as branches converge to a single stop.

**Expected:** Each diamond's yes/no branches join cleanly at merge nodes; final stop has one or two incoming edges.

**Issues:**
- [ ] **`Unique? -> yes` label is rendered INSIDE the "Show Error" node text** — the label "yes" sits on top of the Show Error rectangle. (Show Error is on the no-branch of Unique?, and a flow line for the yes-branch passes through Show Error visually.)
- [ ] **Activate Account is positioned at the far LEFT while Resend Email is BELOW it, both off the central spine.** The visual flow doesn't show a clean two-arm split; instead Activate Account is at a lower-left position and Resend Email dangles below the merge.
- [ ] **The "yes" / "no" labels on Confirmed? are placed on the arrow stems but with poor alignment** — "yes" partially overlapping the underline of "Activate Account".
- [ ] **Show Validation Errors is on the far right** with a horizontal edge from Valid? `no` (or yes — hard to tell since label routing is ambiguous) — the label position implies the WRONG arm.
- [ ] **Excessive empty vertical space below Resend Email** before the stop bullseye (~60% of diagram height is empty).
- [ ] **Subtitle** `activity diagram` (C1).

**Suggested fix scope:** Same as #10: `src/render/graph_layout.rs` orthogonal routing + merge-node materialization. Lighter weight than #10 but same root cause.

**Severity:** P1

---

## docs/examples/state/09_three_level_composite.puml.png

**What I see:** Composite state Device containing Off and On; On contains Initializing / Running / Sleeping; Initializing contains LoadingFirmware and SelfTest; SelfTest contains RamCheck and StorageCheck. Vertical layout, single column. Transitions between siblings render as bidirectional-looking arrow pairs.

**Expected:** Edge labels (`firmware loaded`, `ram ok`, `tests passed`, `init complete`, `task received`, `task complete`, `idle timeout`, `wake event`) should be placed adjacent to the edges they describe, near the midpoint of the arrow.

**Issues:**
- [ ] **All inter-state transition labels are placed in the LEFT and RIGHT margins**, far from their actual edges. "firmware loaded", "ram ok", "tests passed", "init complete", "idle timeout" are all in the LEFT margin; "task complete" is in the RIGHT margin; "wake event" is at the bottom-right far outside the Device frame. The arrows are in the center column but the labels are 200+ pixels away.
- [ ] **The two `Device : entry / connect power` and `exit / flush state` action labels are completely missing** from the rendered output. PlantUML displays these as inline text inside the composite state's frame (below the title bar). Not rendered at all here.
- [ ] **Idle <-> Busy bidirectional pair**: source defines two separate transitions (Idle -> Busy : task received and Busy -> Idle : task complete). The rendered output shows two parallel arrows side by side, which is correct topology, but `task received` label is far left and `task complete` is far right with the arrows in the center — both labels are completely detached.
- [ ] **The Device frame's bottom edge cuts off before the Sleeping state's bottom**, and the wake event label sits outside the Device frame entirely.

**Suggested fix scope:** `src/render/state/edges.rs` + label-placement pass — edge labels must be anchored to edge midpoints with collision avoidance against state rectangles, not pushed to margins. Also `src/render/state/composite.rs` for missing entry/exit action display.

**Severity:** P0 (label detachment makes the diagram unreadable; entry/exit actions missing is a parity gap)

---

## docs/examples/state/12_ch09_parity.puml.png

**What I see:** Source exercises many advanced state features: stereotyped pseudostates `<<entryPoint>>`, `<<exitPoint>>`, `<<inputPin>>`, `<<outputPin>>`, `<<expansionInput>>`, `<<expansionOutput>>`; deep history `[H*]`; styled state with color spec; JSON state blocks `json $payload`/`json $cfg`; attached note; link note.

Rendered: Active state (pink fill, dashed blue outline) connects directly via thick magenta dashed line to Empty, which connects down through a vertical stack of **small empty unlabeled rectangles** (2 small squares, then 2 small concatenated squares, then 2 more — these are presumably the pin / expansion pseudostates) and lands at `$cfg (json)` block, then a small unlabeled circle, then into the Parent composite state. Parent contains Child (light blue) plus a `$payload (json)` block and `<<entry>>`/`<<exit>>` markers (small circles with X). Below Parent: unlabeled X circle, Styled state (green fill, bold red border), link note tag, H* deep history marker.

**Expected:**
- entryPoint: small unfilled circle on the parent state's border with name displayed
- exitPoint: small circle with X on parent border with name displayed
- inputPin / outputPin: small squares on parent border with names
- expansionInput / expansionOutput: square with 3 internal vertical lines (the "expansion bar" glyph), names displayed
- All pseudostates should have their declared names ("entry1", "exit1", "in1", "out1", "expIn", "expOut") rendered as labels.

**Issues:**
- [ ] **All six pseudostates render as unlabeled tiny squares with NO names visible**. A reader cannot tell which is entry1 vs in1 vs expIn. This is a complete loss of information.
- [ ] **Pseudostates are positioned in a vertical stack between Active and Parent**, threaded onto the main flow as if they were waypoints. They should be rendered ON the Parent state's border (since entry/exit points are by definition associated with a composite state), not as standalone waypoints in the main flow chain.
- [ ] **`expansionInput` / `expansionOutput` glyphs are wrong** — they render as paired squares (looks like ▢▢ side by side) instead of the standard expansion-region glyph (vertical lines inside a small square). The diagram does have a different visual for pin vs expansion but neither is the correct UML glyph.
- [ ] **The thick magenta dashed line `Active -[#DD00AA,dashed]-> entry1` correctly applies the color and dash style**, but its label "colored" is placed midway down (between two pseudostates) rather than at the Active->entry1 transition. Acceptable label position-wise but the edge target is unclear since entry1 has no visible label.
- [ ] **`$cfg (json)` JSON block is inserted INTO the main flow** as if it's a state node. The source declares it after the main transitions, so it should render as an off-to-the-side annotation, not as a node on the spine.
- [ ] **The `attached note` (note left of Active)** renders correctly to the left of Active with a leader line — good.
- [ ] **`link note` (note on link)** renders correctly near the Styled -> Done transition — good.
- [ ] **Active's pink fill + dashed blue border** renders correctly.
- [ ] **Styled state's green fill + bold red border** renders correctly; but the source also specifies `text:blue` and the text "Styled" appears black, not blue.

**Suggested fix scope:** `src/render/state/pseudostates.rs` (or wherever stereotyped pseudostates are dispatched) — entry/exit/pin/expansion stereotypes need:
1. Correct UML glyph per stereotype
2. Labels rendered next to the glyph
3. Placement on the owning composite state's border, not as inline flow waypoints

Also: `text:` color spec in `##` style block isn't being applied.

**Severity:** P0 (multiple stereotype glyphs are wrong/missing labels; this is the canonical Chapter 9 parity fixture and it's broken)

---

## docs/examples/state/11_entry_exit_actions_history.puml.png

**What I see:** Connection Manager composite with Disconnected / Connecting / Connected (which contains Authenticated / Degraded and a history pseudostate). entry/exit/do actions inline in each state body. H pseudostate rendered at the very TOP of the diagram outside any state. An initial bullet at top connects down to CM. Disconnected and Connecting have bidirectional-looking arrows. Connected contains Authenticated <-> Degraded.

**Expected:**
- `[H]` history pseudostate should render INSIDE the Connected composite state (the source has `state Connected { [H] --> Authenticated : resume }`).
- All transition labels (connect(), handshakeOk, timeout, disconnect(), networkError, recoveryOk, resume) should be near their edges.
- The initial transition `[*] --> CM` should be one single arrow from the black bullet to CM.

**Issues:**
- [ ] **The `H` history pseudostate is rendered at the top of the diagram, OUTSIDE the Connected composite state**, with an edge connecting down to the initial bullet, then the bullet edges into CM. This implies a topology of `H -> [*] -> CM` which is nowhere in the source. The `H` should live INSIDE Connected and label its edge with "resume".
- [ ] **Edge labels are pushed to the left and right diagram margins**: "resume", "handshakeOk", "networkError" in the LEFT margin; "disconnect()", "recoveryOk" in the RIGHT margin. "connect()" and "timeout" are inline. Same root issue as fixture #09.
- [ ] **Disconnected <-> Connecting** render as two parallel arrows pointing in opposite directions; visually looks like a bidirectional edge but is actually `connect()` and `timeout` (correct semantics). The two labels for these are split: "connect()" is between Disconnected and Connecting (good) but "timeout" is in the right margin (bad).
- [ ] **`handshakeOk` (Connecting -> Connected) has no visible matching edge** in the rendered diagram — the label is in the left margin but no arrow connects Connecting down into Connected at the labeled position. The Connecting state's outgoing edge to Connected may be implicit through layout.
- [ ] **`Connection Manager` title at top** of CM frame renders correctly with the quoted alias styling.
- [ ] **State entry/do/exit action lines** (e.g., `entry / clearCredentials()`) render correctly in italic inline below each state title — good.

**Suggested fix scope:** `src/render/state/composite.rs` — child pseudostates declared inside a composite must render inside the composite's frame; currently `[H]` floats out. Plus same label-placement fix as #09.

**Severity:** P0 (H pseudostate topology is wrong, not just cosmetic)

---

## docs/examples/timing/06_robust_states_value_annotations.puml.png

**What I see:** Four tracks (HTTP Server / Worker Pool / Rate Limiter / Queue Depth). Robust tracks render as colored parallelograms with state names. Concise track (Queue Depth) renders as rectangles with numeric values. Time axis @0 through @700 along top.

**Expected:** Adjacent identical states should merge into a single wider segment (PlantUML behavior — successive `@100 SRV is Handling`, `@200 SRV is Handling` should render as ONE "Handling" box from x(@100) to x(@300), not two side-by-side boxes both labeled Handling).

**Issues:**
- [ ] **HTTP Server has two adjacent "Handling" boxes** between @100/@200 and @200/@300 instead of one merged "Handling" spanning @100->@300. Same issue would apply anywhere a state persists unchanged across consecutive time anchors.
- [ ] **Rate Limiter shows correct merging** — "Open" is correctly one wide segment from @0 to @200 since the source doesn't change it in between, and another wide "Open" from @500 to end. So the bug is specifically with redundant declarations creating phantom segment boundaries.
- [ ] **`robust` and `concise` track-kind hints** appear as small grey text under each participant label — cross-cutting C2 clutter.
- [ ] **`timing diagram` subtitle** — cross-cutting C1.
- [ ] Color palette and trapezoid transition styling between states render well overall.

**Suggested fix scope:** `src/render/timing/segments.rs` — when emitting segments, coalesce adjacent segments where state value is unchanged (preserve only state-change transitions in the rendered geometry).

**Severity:** P1

---

## docs/examples/timing/05_concurrent_timelines_message_arrows.puml.png

**What I see:** Five tracks (CPU / Cache L1 / Memory Bus / I/O Controller / System Clock). Several message arrows between tracks: CPU -> MEM ReadReq at @3, MEM -> CPU ReadResp at @5, CPU -> MEM DmaKick at @6, MEM -> IO DmaAck at @7. The arrows are blue diagonal lines.

**Expected:** Message arrows between non-adjacent tracks should route around intervening tracks (or with a clear visual cue that they're crossing — gap, bridge, or different layer). They should not appear to "pierce" a state box belonging to an unrelated track.

**Issues:**
- [ ] **CPU -> MEM ReadReq arrow at @3 passes directly through the Cache L1 row** — the diagonal line slices across the L1 "Hit" -> "Miss" boundary visually. To a reader this looks like the arrow involves L1, which it doesn't.
- [ ] Same for **ReadResp, DmaKick, DmaAck** — all four cross-row arrows visually pierce intervening track rows.
- [ ] **DmaAck label is squeezed into a thin sliver between Memory Bus and I/O Controller rows** because the arrow endpoint is close to a row boundary. The label is small and risks colliding with row borders.
- [ ] **No arrowhead missing or mis-oriented** — arrowheads render correctly at the destination side.
- [ ] **System Clock waveform** renders correctly as a square wave with low/high transitions at each @n.
- [ ] **`concise` track-kind hint** under each label — C2 clutter.
- [ ] **`timing diagram` subtitle** — C1.

**Suggested fix scope:** `src/render/timing/messages.rs` — cross-row message arrows need to either (a) lift to a dedicated "message layer" rendered above all tracks, or (b) introduce small bridge/jump glyphs where they cross intermediate rows so they don't look like they involve those rows.

**Severity:** P1

---

## docs/examples/timing/08_clock_anchor_messages.puml.png

**What I see:** Time axis ticks visible at @0, @50, @75, @100, @125, @150, @200. Two message arrows from Producer to Consumer at staggered times. Bus clock square waveform with period 50 (visible as alternating high/low pulses). Producer track shows Idle / Header / Payload / Done. Consumer track shows Idle / ReceiveHeader / ReceivePayload / Done.

**Expected:** Time axis ticks should appear at every declared anchor: @0, @clk*1 (= @50), @:frame_start+25 (= @75), @clk*2 (= @100), @:frame_start+75 (= @125), @clk*3 (= @150), @:frame_start+100 (= @150 — collides), @clk*4 (= @200). Also @25 and @175 are not declared explicitly so they should NOT appear; that part is correct.

Looking again, the source only declares 7 unique anchors: 0, 50, 75, 100, 125, 150, 200. Those are the seven ticks shown. So tick set is correct.

**Issues:**
- [ ] **Time axis tick spacing is non-uniform**: @0 to @50 occupies ~250px of width, @50 to @75 occupies ~120px, @75 to @100 occupies ~120px, @100 to @125 ~120px, @125 to @150 ~120px, @150 to @200 ~240px. PlantUML's default behavior is **proportional** spacing — actual time delta maps to pixel delta. The render appears to use roughly equal-cell spacing per declared anchor (with @0->@50 and @150->@200 slightly wider, possibly because they're 50-unit gaps vs 25-unit gaps elsewhere). It's mostly correct proportional-by-delta layout. Acceptable.
- [ ] **No message arrow labels visible.** Source has `TX -> RX@:frame_start+50` and `TX -> RX@:frame_start+100` — neither has a label, so this is technically correct, but a reader has no way to know which message is which.
- [ ] **Arrow targeting `RX@:frame_start+50`** (a future-time anchor on Consumer) should land at x=@100. The first arrow does land in the ReceiveHeader segment which starts at @100 — looks correct.
- [ ] **`clk` label** appears as a small grey label above the Bus clock waveform — clutter.
- [ ] **`clock` and `period 50` subtitle lines** under Bus clock label — C2 clutter.
- [ ] **`timing diagram` subtitle** — C1.

**Suggested fix scope:** Mostly cosmetic — clutter labels under track names should not render unless explicitly requested.

**Severity:** P2

---

## Summary

13 fixtures audited across sequence (4), activity (4), state (3), timing (3).

**P0 count (parity blockers):** 6
1. Sequence 48 — `ref over` participant range clipped one short
2. Sequence 46 — activation bars don't persist across nested frames
3. Activity 18 — fork bar width breaks partition frame
4. Activity 10 — edges pass through nodes; layout convergence broken
5. Activity 16 — swimlanes don't render as side-by-side columns
6. State 09 — transition labels exiled to margins; entry/exit actions missing
7. State 12 — pseudostate stereotypes render as unlabeled boxes
8. State 11 — `[H]` history pseudostate rendered outside its parent composite

**P1 count (visible regressions):** 5
- Sequence 15 — nested alt loses visual nesting
- Sequence 17 — lifelines break between consecutive frames
- Activity 13 — labels overlap nodes; mis-aligned branches
- Timing 06 — adjacent same-state segments not merged
- Timing 05 — cross-row messages pierce intervening tracks

**P2 count (cosmetic):** 1 (timing 08; mostly the cross-cutting clutter labels)

**Cross-cutting clutter labels** (subtitle, track-kind hints) affect ALL activity/timing fixtures — single fix would clear those across the family.

**Most impactful single fix:** `src/render/graph_layout.rs` orthogonal routing and merge-node materialization — would address activity 10, activity 13, state 09 label placement, and state 11 label placement in one pass.

**Most architecturally damaged area:** activity swimlanes (`src/render/activity/swimlanes.rs` per fixture 16) — currently produces a layout that is unreadable for any non-trivial swimlane diagram. This is a major parity gap and worth filing as an epic.
