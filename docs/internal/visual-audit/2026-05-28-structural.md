# Visual audit — 2026-05-28 — Structural diagrams

Bucket: **class + component + use-case + deployment + chen/IE-entity**.

Reviewer: Opus 4.7 multimodal pass over `target/audit_corpus/png/` (corpus ~414
PNGs after this regen). Read tool used on each PNG; no source touched.

Fixtures inspected (most-complex per family):

- Class: `docs/examples/class/03_composition_aggregation.png`, `07_stereotypes.png`,
  `10_full_domain.png`, `12_all_relations.png`, `14_nested_packages.png`,
  `16_interface_hierarchy.png`, `21_microservices.png`, `24_cqrs.png`,
  `32_association_class_deep_packages.png`, `33_mainframe.png`
- Component: `03_packages.png`, `04_deployment_style.png`,
  `07_ports_lollipop_interfaces.png`, `08_cloud_db_queue_stereotypes.png`
- Use case: `03_extends_includes.png`, `04_with_packages.png`,
  `05_actor_generalization_system_boundary.png`, `06_multi_system_boundary.png`
- Deployment: `01_nodes.png`, `04_mixed.png`, `05_three_tier_cloud_onprem.png`,
  `06_kubernetes_pods_containers.png`
- Chen: `03_relationships.png`, `04_weak_eer.png`
- IE entity: `tests/fixtures/families/valid_ie_information_engineering.png`

---

## Finding S-1 — `mainframe` directive renders only the inner frame and drops the body

**File:** `docs/examples/class/33_mainframe.png`

**What I see.** The PNG contains a single small "Domain Frame" folder-tab rectangle
with `Visible` and `Related` classes stacked inside, no surrounding mainframe
chrome, and the whole composition is roughly the size of a single class — far
smaller than the rest of the class corpus. There is no mainframe border around
the diagram and the title bar is missing.

**Expected.** PlantUML's `mainframe Name` directive draws a large rectangular
frame around the entire diagram with the name in a folder-tab in the upper-left,
similar to a sequence-diagram outer frame, with the body diagram laid out inside
at full size.

**Severity:** **P1** — parity gap; affects any user who wraps a class diagram in a
mainframe.

**Suggested fix scope.** Mainframe handling in `src/render/class.rs` (or the
parser pipeline) appears to confuse the `mainframe` directive with a regular
`frame Name { ... }` group and renders only the contents at small scale.
Re-render with PlantUML to confirm baseline behavior.

---

## Finding S-2 — Nested-package layout escapes its parent in deep nesting

**File:** `docs/examples/class/32_association_class_deep_packages.png`

**What I see.** Three nested packages `com > acme > hr`. Then two sibling packages
`reporting` and `payroll` are declared inside `hr`. The rendered frame for
`payroll` extends past the right edge of the `hr` frame and overlaps the outer
`acme`/`com` frame borders. The `has` association label floats far to the right
in white space with no associated edge clearly attached.

Additionally, the inner association class `ReportBinding` (the dashed tether
between `Department → Report` and the association class) has its dashed
connector drawn as a short stub that doesn't reach the midpoint of the
`Department→Report` edge.

**Expected.** All children of `package hr` must lay out within its frame; the `hr`
frame must grow to contain them. Association-class dashed tethers must connect
the association class to the midpoint of the associated edge.

**Severity:** **P0** — nesting invariant broken; this is the canonical "do nested
packages render INSIDE their outer package frame with proper padding?" question
from the audit charter.

**Suggested fix scope.** `src/render/graph_layout.rs` group bbox computation for
deep nesting; verify the recursive `inflate_for_children` pass and association-
class tether-midpoint computation in `src/render/class.rs`.

---

## Finding S-3 — Package-with-relations layout produces edges that traverse through siblings

**File:** `docs/examples/class/14_nested_packages.png`

**What I see.** Three packages `repository`, `service`, `domain`. Six classes:
- `repository::UserRepo`, `repository::ProductRepo`
- `service::UserService`, `service::ProductService`
- `domain::User`, `domain::Product`, `domain::Order`

Multiple relation edges (UserService→User, ProductService→Product,
UserRepo→User, ProductRepo→Product) leave their package, cross the bottom edge
of the `repository` frame, traverse the upper border of the `domain` package,
and land on the target. The edges run vertically right through the middle of
unrelated class bodies (e.g. ProductService's edge runs through both repository
classes' boxes — visible as a vertical black line piercing the `UserRepo` and
`ProductRepo` rectangles when traced).

The `domain::User` box also shows a small arrow-half (V-glyph) painted on its
right edge with no source edge attached — likely a clipped arrowhead from a
relation routed off the package.

**Expected.** Edges between classes in different packages should be routed via
orthogonal channels that do not pierce other class boxes. At minimum, they
should detour around the rectangles. The dangling arrowhead glyph should not
appear.

**Severity:** **P0** — readability blocker for any multi-package class diagram.

**Suggested fix scope.** Orthogonal edge routing for class family with package
groups; very likely a `src/render/graph_layout.rs` channel routing issue that
ignores non-source/non-target node bboxes as obstacles when both endpoints live
in non-adjacent groups.

---

## Finding S-4 — `class` family's inheritance/association arrow on `BaseEntity` is drawn twice and clipped

**File:** `docs/examples/class/10_full_domain.png`

**What I see.** Five classes with a `BaseEntity` super-class. The inheritance
arrow `User --|> BaseEntity` is drawn correctly with the hollow triangle. But a
second short orthogonal line (vertical bar + horizontal stub forming an
incomplete "P" shape) is painted just below the hollow triangle and to the
right, ending mid-air. Looks like a duplicate routed segment or a re-attempted
edge that wasn't garbage-collected.

Separately, the `Order` → `Address` edge labeled `has` and `Order` → `OrderItem`
edge labeled `contains` cross each other at right angles instead of being
deconflicted vertically, and both labels sit on top of the cross point making
"has" and "contains" hard to distinguish.

**Severity:** **P1** — minor extra strokes, but the duplicate arrow stub is
visually confusing and the label collision degrades a canonical example.

**Suggested fix scope.** `src/render/class.rs` edge emission — check for
duplicated path entries when a child has both a generalization and a sibling
association. Edge-label placement heuristic should offset perpendicular to the
edge rather than centering on the bbox midpoint.

---

## Finding S-5 — CQRS diagram doesn't lay out the four orphan classes (`Command`, `Query`) along the same axis as their handlers

**File:** `docs/examples/class/24_cqrs.png`

**What I see.** Four classes at the top — `CommandBus`, `QueryBus`, `Command`,
`Query` — laid out in a single row. Below them, only two classes —
`CommandHandler` and `QueryHandler`. The two upper standalone classes `Command`
and `Query` have no connection to anything (correct for the source) but they
share the same row as the buses, making it look like they're peer entities at
the orchestration tier.

Worse: the connector lines between `CommandBus → CommandHandler` and
`QueryBus → QueryHandler` are drawn with an orthogonal Z-bend that goes
left-down-left-down through empty space, even though both handlers are directly
below their bus. The edges should be straight verticals. Both arrows are also
missing arrowheads.

**Severity:** **P1** — layout looks fine but edges are clearly wrong (no arrows,
unnecessary Z-bends).

**Suggested fix scope.** Class diagram association routing — when the source and
target share an X-coordinate (within tolerance), emit a straight vertical edge
instead of a multi-segment orthogonal route. Confirm `--|>` and `..>` arrowheads
are emitted on the target endpoint.

---

## Finding S-6 — Component `lollipop` interfaces overlap "provides/requires" labels with each other

**File:** `docs/examples/component/07_ports_lollipop_interfaces.png`

**What I see.** The `Order Service` package contains four components with ports
(small port-stubs on the left/right edges, rendered correctly). The lollipop
interfaces `IOrderService`, `IOrderRepository`, `INotifier` hang below their
host components.

Problems:
1. The `provides` and `realizes` labels for `IOrderService` overlap each other
   horizontally on the left side of the lollipop circle. They sit on top of
   the same edge with no separation.
2. The `Message Bus` package title reads `package Message Bus" <<queue>>` —
   there is a stray `"` character before the `<<queue>>` stereotype, indicating
   a quoting-escape bug in the package-stereotype renderer (also visible on
   `package Database" <<database>>`).
3. The `OrderRepository` "uses" edge and the "SQL" edge to `PostgreSQL` overlap
   labels (both around y ≈ 370).

**Expected.** Stereotype suffixes should be `package Message Bus <<queue>>` with
the `"` consumed by the parser. Lollipop labels should be vertically separated
when multiple labels attach near the same circle.

**Severity:** **P0** for the stray `"` quote leakage (that's a parser/render
escaping bug, not just visual noise). **P1** for the label overlap.

**Suggested fix scope.** Package-name quote stripping in
`src/parser/<family>.rs` for the component family; check the path that handles
`package "Name" <<stereotype>>` — the closing quote is being kept in the
display name.

---

## Finding S-7 — Component `04_deployment_style` connector merges two edges then loses the arrowhead

**File:** `docs/examples/component/04_deployment_style.png`

**What I see.** `App Server 1` and `App Server 2` both connect to `Primary DB`
labeled `writes`. The two edges merge into a single shared vertical stub above
`Primary DB`. The arrowhead lands on `Primary DB` correctly, but a small
"connector dot" or joint marker is missing where the two edges merge, and both
`writes` labels are right next to each other, partially overlapping in the
shared trunk segment.

This is acceptable as "bus-style merging" but ambiguous: a viewer cannot tell
whether one edge or two edges hit the database.

**Severity:** **P2** — aesthetic and ambiguity issue; not a correctness bug per
se.

**Suggested fix scope.** When two edges share the same target and are routed to
share a vertical trunk, render them as separate near-parallel lines with a
small offset, or add a connector dot at the merge point.

---

## Finding S-8 — Deployment "node 3D cube" loses its back-right face when nested inside a parent node

**File:** `docs/examples/deployment/05_three_tier_cloud_onprem.png`

**What I see.** Top-level cloud nodes `Internet`, `Cloud Region`, `Corporate
Data Center` render as **dashed-border rectangles** with a folder-tab header
bar — basically a `package` style, NOT the 3D-cube style they should be in
deployment family. The inner 3D nodes (CDN, WAF, Load Balancer, Web Servers,
Redis Primary/Replica, SQS, VPN Gateway, Active Directory, File Server,
PostgreSQL Primary/Standby) DO render as 3D cubes (front face + back-right
top-tab + back-right side stripe) correctly.

Additionally, group titles like `node Cloud Region (us-east-1)" <<cloud>>` show
the same stray `"` character we saw in component diagrams (Finding S-6),
confirming this is a generic quote-stripping bug across container parsers.

Inside the `Cloud Region`, the `Message Queue` and `Cache Cluster` sub-groups
have their entire frame drawn but the inner `node` 3D cubes overflow the
boundary — `node Message Queue" <<queue>>` shows `SQS` cube inside it, but
parts of the cube hang off the bottom of the dashed frame.

**Expected.** A `node`-typed container with children should retain its 3D-cube
visual identity OR explicitly fall back to a dashed-rect group style — but it
must be a deliberate consistent choice, documented in render rules. Currently
it depends on whether the node has children. Children must lay out inside.

**Severity:** **P1** for the cube-vs-rect ambiguity, **P0** for the same quote
leakage as S-6 confirmed on a second family.

**Suggested fix scope.** Document and harmonize the "container node" style.
File one ticket for "deployment node container should keep cube outline when
hosting children" and add a stripped-quote oracle test for `<<stereotype>>` on
package/node names.

---

## Finding S-9 — Kubernetes deployment fixture is barely readable: nodes ride on top of group frame borders

**File:** `docs/examples/deployment/06_kubernetes_pods_containers.png`

**What I see.** A `Kubernetes Cluster` outer node contains four namespace
sub-groups, each of which contains Pod sub-groups, each of which contains
container nodes. The inner Pod frames are drawn with very tight padding so the
container 3D-cubes sit on or just beneath the Pod frame's top folder tab. The
namespace labels overlap at the top: `node Namespace: backend`, `node
Namespace: data`, `node Pod: postgres` all stack on the same Y coordinate with
their text overlapping (~y=325 in the PNG).

Edges (`HTTP`, `mTLS`, `pod`, `redis`, `log forward`) traverse multiple group
borders horizontally, and their labels float in empty space far from their
edge midpoint (e.g. "log forward" sits above the `sidecar-logger` cube
disconnected from the visible vertical edge).

The `Kubernetes Cluster` outer frame's right edge is drawn cleanly, but the
bottom edge cuts off the `postgres-0` and `redis-server` cubes' bottom-right
3D-stripes.

**Severity:** **P0** — three-level nested containers are unreadable. This is
the most complex deployment fixture and it's the worst-rendered.

**Suggested fix scope.** `src/render/graph_layout.rs` recursive nesting padding
needs a per-level inflate-children + recompute-bbox loop; also a label-
placement pass for long-range edges that respects the actual routed segments.

---

## Finding S-10 — Use-case `extend`/`include` edges are not routed orthogonally and overlap each other

**File:** `docs/examples/usecase/05_actor_generalization_system_boundary.png`

**What I see.** Inside the `E-Commerce Platform` system boundary:
- The actors `Administrator`, `Registered User`, `Premium User`, and `User`
  appear scattered. `User` is *above* `Registered User` (correct for actor
  generalization), but the arrowhead for the generalization is drawn far above
  the `User` actor stick-figure (the hollow triangle sits at y ≈ 175 while
  `User` is at y ≈ 145, so the triangle is *above* the parent).
- The `<<extend>>` dashed line from `Apply Promo Code` to `Browse Catalog` is
  drawn as a multi-segment zigzag with the `<<extend>>` label printed *between*
  the segments rather than along the line.
- A duplicate vertical line is drawn from around `Apply Promo Code` going
  downward parallel to nothing — appears to be a stray routing remnant.
- `Registered User` has *two stacked staircase-shaped* edges leaving it going
  down-right; one terminates at `Checkout`, the other dangles and crosses
  `Premium User`'s actor symbol.

**Expected.** Actor generalization arrow points from child to parent with the
hollow triangle at the parent end. `<<extend>>` labels should sit on top of the
dashed line, not between segments. No stray edges.

**Severity:** **P0** — generalization arrowhead is on the wrong side of the
parent actor; that's a meaning-altering correctness bug.

**Suggested fix scope.** Use-case family edge routing — actor-generalization
edges need the arrowhead clamped to the parent's actor symbol bbox edge, not
extrapolated above it. Investigate `src/render/usecase.rs` (or the equivalent
module) edge-endpoint computation for actor symbols (they're not boxes; the
"top" of an actor is the head circle's top).

---

## Finding S-11 — Multi-system-boundary use-case routes actor-to-usecase edges through unrelated boundaries

**File:** `docs/examples/usecase/06_multi_system_boundary.png`

**What I see.** Three actors at the top — `System`, `Customer`, `Support Agent`
— and three system boundaries side-by-side: `Customer Portal` (left),
`Automation Engine` (right), and a lower `Support Console`.

Edges from `Customer` go to `Login`, `View Orders`, and `Chat with Agent` —
inside the boundaries — but they're drawn as long vertical lines that cut
straight through the top edges of `Customer Portal` and `Automation Engine`
boundary frames. Each of the actors' four edges runs through both upper
boundary frame borders, not via a routed channel.

Worse: the `<<extend>>` label from `Close Ticket` to `Escalate Issue` is
printed *above* `Support Console`'s frame label, completely outside the
boundary frame, while the dashed line itself is correctly inside.

**Severity:** **P0** — system-boundary container is essentially decoration only;
edges ignore it. Make boundaries proper routing-channel obstacles.

**Suggested fix scope.** Use-case orthogonal routing must treat system-boundary
group rectangles as routing obstacles (or at least snap edges to enter/exit at
the frame boundary). `<<extend>>`/`<<include>>` label positioning must follow
the routed polyline midpoint.

---

## Finding S-12 — Chen ERD `weak entity` is rendered with the correct double-rectangle but the EER specialization triangle is empty

**File:** `docs/examples/chen/04_weak_eer.png`

**What I see.** Strong entity `PARENT`, weak entity `CHILD` (drawn with double
border, correct), and EER specialization sub-entities `TEEN`, `TODDLER`. The
`d` (disjoint) specialization marker is drawn as a small light-blue diamond
labeled `d` with `EER` underneath. **However:**

1. The disjoint diamond should be a circle (or a circle with `d` inside) per
   conventional Chen/EER notation, not a small rotated square. The current
   rendering uses the relationship-diamond shape for specialization, which
   conflates the two concepts.
2. The line from the `d` diamond down to `TEEN` and `TODDLER` is rendered as a
   single solid line per child without the standard double-line "specialization
   subset" indicator. The line from `CHILD` *to* the `d` marker passes through
   the `Age` attribute oval (or very close to it) on the left.
3. The identifying relationship `PARENT_OF` between weak `CHILD` and strong
   `PARENT` should be drawn as a **double-bordered diamond** because it's
   identifying. Here it's a single-border diamond, identical to a non-
   identifying relationship.

**Severity:** **P1** — Chen/EER notation correctness; affects pedagogical
value of the renderer.

**Suggested fix scope.** Chen renderer (`src/render/chen.rs` typed scene
emitter): differentiate identifying relationships (double diamond) and
specialization markers (circle, not diamond). Add `is_identifying` flag in the
typed scene per Wave-21 refactor patterns.

---

## Finding S-13 — IE entity diagram lacks crow's-foot endpoints on relations

**File:** `tests/fixtures/families/valid_ie_information_engineering.png`

**What I see.** Three IE entities `CUSTOMER`, `ORDER`, `LINE_ITEM` rendered
nicely as PK/attribute boxes with yellow stereotype headers. Two relations
labeled `places` (CUSTOMER → ORDER) and `contains` (ORDER → LINE_ITEM, dashed).

Endpoint glyphs are problematic:
1. The `referred_by` self-relation on `CUSTOMER` shows a `┤` (left-bracket + cross
   bar) glyph at the CUSTOMER end — that's an IE "one and only one" marker. But
   the other end of that self-loop hangs down off the bottom of the entity with
   no visible endpoint glyph at all (just a line truncated).
2. The `places` relation between CUSTOMER and ORDER has **no crow's-foot or
   bar glyph at either end** — it's just a plain line. Should show `||` (one
   mandatory) on the CUSTOMER side and `}|` (one-or-many) on the ORDER side
   per typical IE notation, depending on cardinality.
3. The `contains` relation between ORDER and LINE_ITEM (drawn dashed, suggesting
   non-identifying) similarly has no endpoint glyphs.

**Expected.** IE crow's-foot notation requires explicit glyphs (`||`, `o|`, `}|`,
`o{`) at the relation tip touching the entity box edge. Tests in the
audit charter explicitly call out: "do the cardinality glyphs at relation
endpoints actually render at the line tip, not floating?" — answer here is no,
they don't render at all on most edges.

**Severity:** **P0** — IE notation is unusable without crow's-foot glyphs.

**Suggested fix scope.** Render path for IE relations needs an endpoint-glyph
emitter that reads the relation's parsed cardinality and draws the standard
crow's-foot at each end. Verify against the spec PDF section on IE notation.

---

## Finding S-14 — Stereotype rendering inconsistency between class diagrams and IE/component diagrams

**Cross-cutting observation across:** `class/07_stereotypes.png`,
`component/08_cloud_db_queue_stereotypes.png`,
`valid_ie_information_engineering.png`

**What I see.** Stereotype glyph styles:

- Class family (`class/07_stereotypes.png`): `«controller»`, `«service»`,
  `«repository»`, `«entity»` rendered in italic small caps centered above the
  class name, using French guillemets. The `entity` variant correctly gets a
  light-yellow header. Crisp and readable.
- Component family (`component/08_cloud_db_queue_stereotypes.png`):
  `«component»` consistently rendered, but no special header color for
  `<<cloud>>`, `<<database>>`, `<<queue>>` stereotype-typed packages — they
  all look the same dark-header style.
- IE family (`valid_ie_information_engineering.png`): `«entity»` uses
  guillemets, matching class diagrams — good.
- Deployment family (`deployment/05_three_tier_cloud_onprem.png`): `<<cloud>>`,
  `<<queue>>`, `<<database>>` use double-angle ASCII brackets `<<...>>`, NOT
  the typographic guillemets `«...»`. Mixed conventions across families.

**Expected.** Single project-wide stereotype glyph style. Likely
guillemets-italic across the board per PlantUML reference.

**Severity:** **P2** — visual consistency / brand polish; not blocking.

**Suggested fix scope.** Stereotype rendering helper should be shared across
families. Audit `src/render/*.rs` for places that emit `<<...>>` directly
instead of going through a `format_stereotype()` helper.

---

## Finding S-15 — Component `02_interfaces.png` style component shapes appear correctly but package-stereotype text shows escape leakage (same as S-6)

(Referenced for context — already covered by S-6 fix.)

---

## Severity summary

| Finding | Family | Severity | File reference |
|---|---|---|---|
| S-1 mainframe directive collapses | class | P1 | `class/33_mainframe.png` |
| S-2 nested package overflow + association-class tether | class | **P0** | `class/32_association_class_deep_packages.png` |
| S-3 inter-package edges pierce sibling class boxes | class | **P0** | `class/14_nested_packages.png` |
| S-4 duplicate inheritance stub + label collision | class | P1 | `class/10_full_domain.png` |
| S-5 CQRS edges Z-bend + missing arrowheads | class | P1 | `class/24_cqrs.png` |
| S-6 component package-stereotype `"` leakage + lollipop label overlap | component | **P0** (quote leak) / P1 (label) | `component/07_ports_lollipop_interfaces.png` |
| S-7 merged "writes" trunk lacks connector dot | component | P2 | `component/04_deployment_style.png` |
| S-8 deployment node-container styling inconsistent + quote leakage | deployment | **P0** (quote leak) / P1 (style) | `deployment/05_three_tier_cloud_onprem.png` |
| S-9 Kubernetes deep-nest unreadable | deployment | **P0** | `deployment/06_kubernetes_pods_containers.png` |
| S-10 actor-generalization arrowhead floats above parent | use-case | **P0** | `usecase/05_...png` |
| S-11 system-boundary frames don't block edges | use-case | **P0** | `usecase/06_multi_system_boundary.png` |
| S-12 Chen EER specialization shape + identifying-diamond wrong | chen | P1 | `chen/04_weak_eer.png` |
| S-13 IE crow's-foot endpoints missing | IE | **P0** | `valid_ie_information_engineering.png` |
| S-14 stereotype glyph style inconsistent across families | cross | P2 | multiple |

P0 count: 7 unique issues (S-2, S-3, S-6 quote leak, S-8 quote leak, S-9,
S-10, S-11, S-13). S-6 and S-8 share a root cause (quote-stripping bug in
container parsers), so 6 distinct root causes.

---

## Recommended ticket grouping for swarm wave

1. **Container parser quote-stripping** (S-6 + S-8): one ticket, parser scope,
   affects component + deployment.
2. **Nested-package layout invariants** (S-2 + S-3 + S-9): one architecture
   ticket spawning multiple implementation tickets — recurring graph_layout
   issue across class, deployment, and IE.
3. **Use-case routing through system boundaries** (S-10 + S-11): one ticket,
   usecase renderer scope.
4. **IE crow's-foot endpoint emission** (S-13): standalone, ie_entity renderer.
5. **Chen EER specialization + identifying-diamond** (S-12): standalone,
   chen renderer.
6. **Class arrowhead/route polish** (S-4 + S-5 + S-1): low-priority cleanup,
   one ticket.

---

*Audit performed: 2026-05-28.* Source untouched. PNGs only. No SVG inspection.
