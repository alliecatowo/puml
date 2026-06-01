## Forensic: Examples Sprawl, AWS/GCP/Azure Stdlib Status, Family Completeness (2026-06-01)

**Author:** Forensic audit agent (Claude Opus 4.7)
**Date:** 2026-06-01
**Trigger:** Allie's question: "we have a bunch of examples causing sprawl, could we reconcile some of them so it'd easier to see a lot of issues in 1, and then work on aws/gcp/whatever styling (I'm thinking it's still missing), and make sure good examples showing off everything exist? more families we need to make?"

## Executive summary

1. **Examples sprawl is real but smaller than feared.** `docs/examples/` ships **341 `.puml` files** across **38** family-or-topic directories. Most large families (sequence=49, class=34, themes=31, skinparams=20, activity=18) are well-organized; the corpus is intentionally maximal because `scripts/render_check.py` and `tests/all_examples_render.rs` use it as an executable-documentation evidence corpus. **Recommended consolidation is targeted (~20-30 files), not wholesale.** The big bottleneck for deletion is `tests/coverage_render_other.rs`, `tests/coverage_render_family.rs`, `tests/render_invariants.rs`, `tests/parity_state_activity_timing_depth.rs`, `tests/oracle_promoted_fixtures.json`, and the visual-regression manifest, which `include_str!` real example paths. Tickets filed; risk-managed consolidation done in this PR is limited to **two obvious dupes** with no test references.

2. **AWS stdlib is the only one of the three cloud packs that renders correctly today.** Azure and GCP stdlib macros (`AzureVM`, `ComputeEngine`, etc.) have a **swapped-arg bug** in their `AzureIcon` / `GCPIcon` common procedures: the visible label becomes the macro name (e.g. `AzureVM`, `ComputeEngine`) and the user-supplied label gets mangled into a stereotype string like `<<azure->>`. AWS works because each AWS service shim (EC2.puml, RDS.puml) bypasses the common procedure and emits `object $label as $alias <<aws-...>>` directly. **Filed as a P1 stdlib bug.** Beyond the bug, the cloud shims are all label-only `object` stubs with no icon art — upstream PlantUML ships real sprites. Filed as a parity enhancement.

3. **Family completeness vs PlantUML LRG: all 19 family chapters covered, all "structural" / common-command chapters covered.** Zero PlantUML-LRG **diagram** families are missing. The 27 chapters of the LRG decompose to 19 diagram families (chapters 1-20), 6 cross-cutting topics (chapters 21-26), and 1 stdlib chapter (27). PUML implements all 19, plus 11 PUML-extension families (chronology, chart, chen, board, files, wire, ditaa, regex, ebnf, sdl, stdlib-keyword). **What's actually missing is depth, not new families** — and the right tracking surface for that is the chapter audits under `docs/internal/spec/audit/`, not new family renderers. Three top-priority chapter-depth gaps stand out for new tickets: stdlib packs (`kubernetes`, `archimate`, `tupadr3` sprites), Salt advanced widgets (`ch14` 3 ❌ rows), and Creole block-level (`ch22` 16+ ❌ rows).

4. **One consolidation safe to ship now:** `docs/examples/nwdiag/02_multi_network.puml` and `docs/examples/nwdiag/02_multiple_nets.puml` collide on numeric prefix (`02_`) and demonstrate near-identical content. The richer `02_multi_network.puml` strictly covers the smaller `02_multiple_nets.puml`. Verified zero references outside `docs/examples/` and `docs/benchmarks/` (the benchmark JSON regenerates from corpus via `scripts/render_check.py`, so it self-heals). **This deletion is the only change applied to `docs/examples/` in this PR.** Two other candidates I originally proposed (deleting `class/06_abstract_interface.puml` and renaming the `nwdiag/03_/04_` prefix collisions) were dropped: `class/06` is referenced by `tests/coverage_render_family.rs::class_abstract_interface_typed_scene_counts` which asserts a 4-node set (Shape/Drawable/Circle/Rectangle) that class/13 does not provide; the nwdiag renames would cascade into `docs/benchmarks/render_check_latest.json` artifact-freshness checks. Both deferred to dedicated future PRs with proper test/benchmark refresh.

---

## Section 1: Example sprawl analysis and consolidation plan

### 1.1 Inventory by family directory

```
sequence       49
class          34   (including 11_spot_stereotypes.puml at same prefix as 11_generics.puml)
themes         31
skinparams     20
activity       18
state          14
component      12
c4             12
timing         10
gantt          10
deployment      9
activity_new    9
usecase         8
wbs             7
salt            7
nwdiag          7   (! two prefix collisions at 02_ and 03_)
mindmap         7
sprites         6
preprocessor    6
object          6
chart           6
creole          5
chronology      5
wire            4
json            4
chen            4
activity_old    4
yaml            3
regex           3
ebnf            3
archimate       3
sdl             2
math            2
ditaa           2
board           1
files           1
stdlib          1
(top-level)     6
TOTAL         341
```

**Top-level files (6):** `basic_hello.puml`, `groups_notes.puml`, `lifecycle_autonumber.puml`, `supported_primitives_lifecycle_structure.puml`, `supported_primitives_participants_messages.puml`, `supported_primitives_styling_groups_notes.puml`. The three `supported_primitives_*` files are referenced by `docs/examples/supported_primitives.md` and by `docs/benchmarks/render_check_latest.json` and `docs/benchmarks/parity_latest.json`. `basic_hello.puml` is referenced by `tests/oracle_promoted_fixtures.json`. None of the six are safe to delete in this PR.

### 1.2 Concrete duplicates and near-duplicates

| Pair / Group | Status | Action |
|---|---|---|
| `nwdiag/02_multi_network.puml` vs `nwdiag/02_multiple_nets.puml` | Different content, same `02_` prefix — collides; both teach same thing | **Delete `02_multiple_nets.puml`**; richer `02_multi_network.puml` covers it |
| `nwdiag/03_address_ranges.puml` vs `nwdiag/03_with_groups.puml` | Both at `03_` prefix, distinct teaching value | **Rename `03_with_groups.puml` → `04_groups.puml`** (conflict with existing `04_groups.puml` — see below) |
| `nwdiag/04_groups.puml` vs `nwdiag/04_icons_multiline.puml` | Two files at `04_` prefix | **Renumber: `04_groups.puml` → `05_groups.puml`** so `03_with_groups.puml` can take `04_`, and `04_icons_multiline.puml` → `06_icons_multiline.puml` |
| `class/06_abstract_interface.puml` vs `class/13_abstract_interface.puml` | Same name, class/13 is richer | **Delete `06_abstract_interface.puml`**, keep `13_abstract_interface.puml`. The slot at `06_` is now free; do not renumber other files (high test-reference cost). |
| `sequence/04_autonumber_format.puml` vs `sequence/20_autonumber_format.puml` | Both named "autonumber_format" but show different features (start value vs stop/resume) | **Rename `04_autonumber_format.puml` → `04_autonumber_start.puml`**, keep `20_autonumber_format.puml` as-is. (Deferred — moderate test-reference risk; ticket filed.) |
| `activity_new/` vs `activity/` | `activity_new/` overlaps with `activity/` (both use modern `start; :a; stop` syntax). `activity_old/` correctly tracks legacy `(*) --> "X"` syntax. | **Defer** — `activity_new/06_partition.puml` and `activity_new/08_*` are referenced by tests. Ticket filed to merge `activity_new/` into `activity/` in a future dedicated PR (3-4 file moves + test-path edits). |
| `themes/01_plain.puml`..`themes/10_spacelab_box.puml` vs `themes/theme_*.puml` (21 files) | Two parallel naming schemes coexist | **Defer** — large blast radius; ticket filed. |

### 1.3 What this PR does (safe consolidation, zero test refs)

**Deletions (1 file pair):**
- `docs/examples/nwdiag/02_multiple_nets.puml` + `.svg`

**Verification:** `grep -rE "nwdiag/02_multiple_nets"` returns zero hits in `tests/`, `scripts/`, `src/`, `crates/`, `agent-pack/`, and the only matches in `docs/` are inside `docs/benchmarks/render_check_latest.json` and `docs/benchmarks/parity_latest.json`, which are auto-derived snapshots that regenerate from the corpus on the next `python3 scripts/render_check.py` run. The `docs/examples/GALLERY.md` per-row entries are family directory names, not per-file links.

**Originally proposed but DROPPED after deeper verification:**
- `class/06_abstract_interface.puml` deletion — **blocked**: `tests/coverage_render_family.rs::class_abstract_interface_typed_scene_counts` asserts a 4-node typed scene (Shape, Drawable, Circle, Rectangle). `class/13_abstract_interface.puml` is structurally different (adds Resizable, drops Rectangle) and would break this assertion. Filed as a ticket to either merge the test's expectations or rename class/13 to a non-overlapping name.
- nwdiag `03_with_groups.puml` and `04_groups.puml` / `04_icons_multiline.puml` renames — **deferred**: cascade into `docs/benchmarks/*.json` artifact-freshness paths. The CI check `check-artifact-freshness` would flag the rename, so this needs to be a dedicated PR that runs the corpus regenerator and commits the benchmark JSON refresh as part of the same change.

### 1.4 Deferred consolidations (with tickets filed)

| Action | Why deferred | Test cost |
|---|---|---|
| Merge `activity_new/*` into `activity/*` | `tests/parity_state_activity_timing_depth.rs` calls `include_str!("../docs/examples/activity_new/06_partition.puml")`; visual manifest references `activity_new/08_notes_split_partitions.puml` | 1 test edit + 1 manifest edit |
| Drop top-level `basic_hello.puml` etc. in favor of canonical `sequence/01_basic.puml` | `tests/oracle_promoted_fixtures.json` references `basic_hello.puml`; gallery + supported-primitives.md doc reference 5 of the 6 | 3-4 doc edits + 1 oracle entry update |
| Themes dir rename of legacy `theme_*.puml` to numeric scheme (or vice versa) | 21 files referenced from gallery and themes README | 2-3 doc edits + path scan |
| Sequence dir: 49 files contain three theme-showcase variants (26, 27, 28, 29, 30 + 37-45) — could be consolidated into one `26_theme_showcase.puml` that cycles through themes via separate `@startuml` blocks | Each theme example is exercised individually by `tests/all_examples_render.rs` (a directory walk; not per-name), but `scripts/render_check.py` outputs per-file rows in `render_check_latest.json` | Low test cost; medium docs-regen cost |

### 1.5 Recommended canonical per-family example set (target for future cleanup)

Aspirational target: **3-5 fixtures per family**, covering basic / complex / styled / edge-case. Sequence diagrams should be split into a `sequence/` family demo subset + a `themes/` showcase subset rather than blending 26 themed variants into the sequence directory. Class diagrams should publish one "patterns gallery" (multi-pattern), not one per pattern (currently 17, 18, 19, 25, 26, 27, 28, 29, 30 are all "X pattern" fixtures — keep 2-3, fold the rest into a `class/patterns.puml`).

| Family | Current | Target | Drop candidates |
|---|---:|---:|---|
| sequence | 49 | 12 | 26_theme_aws..45_theme_carbon_gray (20 theme variants) → fold into `sequence/26_theme_gallery.puml` containing 5 representative themes |
| class | 34 | 10 | 17-30 (pattern fixtures) → fold into `class/17_design_patterns.puml`. Plus duplicate `06_abstract_interface.puml`. |
| themes | 31 | 10 | The 10 numeric + 21 named have overlap; pick 10 representative |
| skinparams | 20 | 8 | Single-key fixtures (`07_maxmessagesize`, `09_default_font`, etc.) → fold smaller ones into combined fixtures by category |
| activity | 18 | 8 | Workflow fixtures (10_authentication, 13_user_registration, 14_purchase_flow) are interchangeable demonstrations of swimlanes + sequence; keep 2 |

These should land in a follow-up PR with a coordinated test-rename pass — too much surface for a single agent session under the 150min ceiling.

---

## Section 2: AWS / GCP / Azure stdlib styling status

### 2.1 Bundled stdlib packs

`puml -stdlib` reports the following root-level packs:

- `C4/` — 7 includes, macros implemented
- `awslib/` (alias) → `awslib14/` — 25 includes across Compute / Database / Networking / Security / Storage
- `azure/` — 11 services + AzureCommon
- `gcp/` — 11 services + GCPCommon
- `material/` — 53 icon shims + Material.puml
- `office/` — 5 icon shims
- `tupadr3/` — 4 icon shims

**Missing upstream packs** (per `puml -stdlib` header comment and `docs/internal/spec/audit/ch27-stdlib.md`):
- `DomainStory`, `ada`, `archimate`, `aws` (the non-Labs upstream variant), `bootstrap` (macro form), `classy`, `classy-c4`, `edgy`, `eip`, `elastic`, `k8s` / `kubernetes`, `material7`, `cloudinsight`, `cloudogu`, `logos`, `osa`

(Note: Bootstrap Icons and Google Material Icons are bundled as **built-in sprite registries** with full SVG art via `<$bi-...>` / `<$ma_...>` syntax. The local `stdlib/material/*.puml` files are deterministic macro/include shims, not the full upstream stdlib.)

### 2.2 Verification: AWS renders correctly

Test fixture used:
```
@startuml
!include <awslib/AWSCommon>
!include <awslib/Compute/EC2>
!include <awslib/Database/RDS>
!include <awslib/Storage/S3>
EC2(web, "Web server")
RDS(db, "Postgres")
S3(bucket, "Assets")
web --> db : queries
web --> bucket : reads
@enduml
```

**Result:** Renders correctly. Each node shows AWS-orange (#FF9900) header bar, "AWS" badge in upper-left, service name in white in the header ("EC2", "RDS", "S3"), and the user-supplied label ("Web server", "Postgres", "Assets") in dark text underlined in the body. This matches the structure of an `aws-ec2` stereotyped object.

(See `/tmp/aws_test.png` produced during this audit. Visual check passed.)

### 2.3 Verification: Azure and GCP are broken (P1 stdlib bug)

Test fixture (Azure):
```
@startuml
!include <azure/AzureCommon>
!include <azure/AzureVM>
AzureVM(vm, "Web VM", "Linux")
@enduml
```

**Result:** The visible underlined label is **`AzureVM`** (the macro name), the "service name" in the header reads `"Web VM"` (with literal quotes), and the stereotype string is malformed (`<<azure->>` instead of `<<azure-vm>>`).

**Root cause** (in `stdlib/azure/AzureCommon.puml`):
```
!procedure AzureIcon($alias, $service_or_label, $label_or_technology="", $descr="")
object $service_or_label as $alias <<azure-$label_or_technology>>
!endprocedure
```

The variable name `$service_or_label` betrays the bug: when called as `AzureIcon($alias, AzureVM, $label, $descr)` from `AzureVM.puml`, the second positional arg is the service name (`AzureVM`) but is being used as the `object`'s display label, while `$label` (the user's `"Web VM"`) is used as the stereotype technology suffix.

**Identical bug pattern in `stdlib/gcp/GCPCommon.puml`** with `GCPIcon`. Same symptom: `ComputeEngine` shown as the underlined label, `"Web VM"` shown as the service name.

**Compare to working AWS shim (`stdlib/awslib14/Compute/EC2.puml`)**, which does **not** go through a shared `AWSIcon` indirection — it directly emits:
```
!procedure EC2($alias, $label="", $description="")
object $label as $alias <<aws-ec2>>
!endprocedure
```

**Fix shape:** Either (a) inline every Azure/GCP shim like the AWS pattern (16+ shim edits but no abstraction shift), or (b) repair `AzureIcon` / `GCPIcon` to swap arg order so `$label` is the displayed label and the per-service caller passes the service-name as the stereotype suffix. Filed as P1 ticket.

### 2.4 Cloud icon styling beyond macros

The AWS render shows real visual treatment: a colored header bar with an "AWS" badge, the service name in the header, and the user label in the body. This is provided by the **renderer's** cloud-icon node path (`src/render/family/cloud_icons.rs`) keyed off the `<<aws-...>>` stereotype, **not** by the stdlib shim. Once the Azure/GCP arg-order bug is fixed, the `<<azure-vm>>` / `<<gcp-compute-engine>>` stereotypes should similarly trigger Azure-blue and GCP-blue-and-green branded headers via `cloud_icons.rs`. (Confirmed: Azure renders teach `"AzureBlobStorage"` etc. inside an Azure-blue badge — see `/tmp/azure_test.png` — so the **renderer side already exists**, the stdlib side is just calling it with swapped args.)

**Sprite art (real icons):** AWS, Azure, GCP services are still rendered as labeled boxes + a textual service-name in the header. Upstream PlantUML ships real sprite SVG paths for each service. PUML's stdlib shims are explicitly documented as "deterministic compatibility stubs, not bulk-vendored upstream art" (see `stdlib/README.md`). This is a known parity gap; tracked at #88 / chapter-27 audit. Not urgent — most users get fine results with the colored badges PUML already produces. Filed as a `parity` enhancement ticket, not P1.

---

## Section 3: Family completeness vs PlantUML LRG

### 3.1 PUML's family enum

`src/ast/mod.rs::DiagramKind` enumerates:

```
Sequence, Class, Object, UseCase, Salt, MindMap, Wbs, Gantt,
Chronology, Component, Deployment, State, Activity, Timing,
Json, Yaml, Nwdiag, Archimate, Regex, Ebnf, Math, Sdl, Ditaa,
Chart, Stdlib, Chen, Board, Files, Wire, Unknown
```

= **29 diagram families** (counting `Unknown` as a no-op).

### 3.2 PlantUML LRG family coverage

The 27 LRG chapters partition as:

**Diagram families (ch 1-20):**

| # | Chapter | PUML | Notes |
|---|---|---|---|
| 1 | Sequence | ✅ | 28✅ / 14🟡 / 6❌ at chapter level |
| 2 | Use Case | ✅ | 15 / 2 / 1 |
| 3 | Class | ✅ | 22 / 13 / 11 |
| 4 | Object | ✅ | 5 / 2 / 1 |
| 5 | Activity (legacy) | ✅ | 0 / 2 / 10 — partial parser, basic render |
| 6 | Activity (new) | ✅ | 6 / 12 / 12 |
| 7 | Component | ✅ | 12 / 5 / 2 |
| 8 | Deployment | ✅ | 14kw / 10sec / 15kw |
| 9 | State | ✅ | 14 / 5 / 6 |
| 10 | Timing | ✅ | 4 / 12 / 13 — MVP renderer |
| 11 | JSON | ✅ | 7 / 4 / 3 |
| 12 | YAML | ✅ | 3 / 1 / 4 |
| 13 | nwdiag | ✅ | 15 / 2 / 2 |
| 14 | Salt | ✅ | 12 / 9 / 3 |
| 15 | ArchiMate | ✅ | 6 / 2 / 1 |
| 16 | Gantt | ✅ | 29 / 5 / 5 |
| 17 | MindMap | ✅ | 11 / 2 / 0 |
| 18 | WBS | ✅ | 6 / 4 / 2 |
| 19 | Math | ✅ | 3 / 1 / 1 |
| 20 | Information Engineering | ✅ | Subsumed by Class arrows (IE crow's-foot in ch3); 1 / 2 / 2 |

**Cross-cutting topics (ch 21-26):**

| # | Chapter | PUML | Notes |
|---|---|---|---|
| 21 | Common Commands | ✅ | 14 / 7 / 1 |
| 22 | Creole | 🟡 | 14 / 4 / 16+ — block-level markup is the biggest depth gap |
| 23 | Sprites | ✅ | 9 / 0 / 1 |
| 24 | Skinparam | 🟡 | 5 / 4 / 5 — work in progress under style-cascade epic |
| 25 | Preprocessing | ✅ | 18 / 9 / 1 |
| 26 | Unicode | ✅ | 3 / 2 / 0 |

**Stdlib (ch 27):**

| 27 | Standard Library | 🟡 | 8/17 packs bundled, 9/17 not bundled. See section 2. |

### 3.3 Conclusion: zero missing diagram families

**Every LRG diagram family chapter maps to a PUML `DiagramKind` variant.** PUML also implements 11 **PUML-extension families** that are not in PlantUML's LRG:
- `Chronology` (PUML-specific timeline format)
- `Chart` (bar / line / pie / multi-series)
- `Chen` (Chen-style ER, distinct from the IE notation in ch20)
- `Board` (sprint board / kanban)
- `Files` (file-tree visualisation)
- `Wire` (wireframe widgets, complementing Salt)
- `Ditaa` (ASCII-art rendering — partial parity with PlantUML's ditaa pass-through)
- `Regex` (regex-DFA visualisation)
- `Ebnf` (grammar railroad diagrams)
- `Sdl` (SDL process diagrams)
- `Stdlib` (`stdlib` listing keyword)

(Note: PlantUML technically supports Ditaa / SDL / EBNF / Regex through different routes, but PUML's first-class native renderers for these count as "extension families" relative to the strict LRG chapter list.)

### 3.4 Where real depth gaps live (priority-ranked)

Not "new families," but **expanding existing families** to close ❌ rows in the chapter audits:

| Priority | Gap | Where | Estimated demand |
|---|---|---|---|
| **P1** | Creole block-level (lists, tables, links, separators, headers) — ch22 has 16+ ❌ rows | `src/render/text.rs`, `src/render/text_metrics.rs`, parser creole pass | **High** — every diagram with notes hits creole; tables are a common feature request |
| **P1** | Salt advanced widgets (3 ❌ rows) — sliders, scrollbars, password fields | `src/render/salt.rs`, parser Salt grammar | **Medium-high** — Salt users typically need richer widgets |
| **P1** | Stdlib packs: `kubernetes`, `archimate`, `tupadr3 sprites`, `bootstrap macros` | `stdlib/*` directory tree | **High** for k8s; **medium** for others |
| **P2** | Timing renderer depth (13 ❌ rows) — analog levels, full clock waveforms, message-bus syntax | `src/render/timing/` | **Medium** — timing is a niche but vocal use case |
| **P2** | Activity legacy (`(*)--> "X"`) — 10 ❌ rows | `src/parser/`, `src/render/activity/` | **Low** — legacy syntax, declining usage |
| **P2** | Class depth (11 ❌ rows) — full stereotype display, smetana layout hooks, member-level visibility extras | `src/render/family/class_*` | **Medium-high** — class is the most-used family |
| **P3** | YAML depth (4 ❌ rows) — `?` complex keys, anchors / aliases | parser YAML pass, `src/render/family/tree.rs` | **Low** |
| **P3** | nwdiag depth (2 ❌ rows) — peer-connector mode, IPv6 ranges | `src/render/family/nwdiag/`, parser | **Low-medium** |

**No new family renderers are needed for PlantUML LRG parity.** Resource allocation should go to closing chapter-level ❌ rows in the families we already have.

---

## Section 4: Recommendations ranked by ROI

| Rank | Action | Effort | Impact | Ticket |
|---|---|---:|---:|---|
| 1 | **Fix Azure/GCP stdlib arg-order bug** | XS (single PR, ~16 file edits or 1-line fix in two commons) | **P1** — unblocks cloud-architecture demos | [#1493](https://github.com/alliecatowo/puml/issues/1493) |
| 2 | **Land safe consolidation in this PR** (1 nwdiag dup deletion) | done in this PR | low — clean prefix collision | in-PR |
| 3 | **Activity_new/ → activity/ merge** | S (1 test edit, 8 file moves) | medium — eliminates a confusing directory split | [#1498](https://github.com/alliecatowo/puml/issues/1498) |
| 4 | **Bundle stdlib `kubernetes` pack** | M (new directory + ~20 shim files) | **P1** high — biggest unanswered cloud-arch request | [#1495](https://github.com/alliecatowo/puml/issues/1495) |
| 5 | **Per-family canonical example reduction** (sequence: 49→12, class: 34→10, themes: 31→10) | L (4 child PRs; coordinate test-path edits) | high — Allie's stated goal, "easier to see issues in 1" | [#1504](https://github.com/alliecatowo/puml/issues/1504) |
| 6 | **Sprite art for AWS/Azure/GCP** | L (parse upstream sprite definitions, normalize) | medium — visual polish, not blocking core use | [#1500](https://github.com/alliecatowo/puml/issues/1500) |
| 7 | **Creole block-level markup (ch22 depth)** | L (parser pass + text rendering) | **P1** high — improves all diagram-with-notes outputs | [#1502](https://github.com/alliecatowo/puml/issues/1502) |
| 8 | **Salt advanced widget set** | M (renderer additions for 3-4 widget types) | medium | [#1503](https://github.com/alliecatowo/puml/issues/1503) |
| 9 | **Top-level dedup**: move `basic_hello.puml` etc. into `sequence/` and update oracle + benchmark refs | M (3-4 doc edits + 1 oracle JSON edit) | low-medium — cleaner repo root | [#1501](https://github.com/alliecatowo/puml/issues/1501) |

### Out of scope for this investigation

- Verifying every individual stdlib shim against upstream PlantUML — covered by `docs/internal/spec/audit/ch27-stdlib.md`
- Fixing the renderer-side cloud icon styling for Azure/GCP — already exists, the bug is purely on the stdlib shim side
- Rewriting the visual-baseline manifest — out of scope; the manifest already correctly tracks ~44 fixtures with the deferral pattern

---

## Appendix A: Files deleted in this PR

Verified by `grep -rE` across `tests/`, `scripts/`, `docs/`, `src/`, `crates/`, `agent-pack/`:

- `docs/examples/nwdiag/02_multiple_nets.puml` — zero references outside `docs/examples/` and `docs/benchmarks/` (auto-derived)
- `docs/examples/nwdiag/02_multiple_nets.svg` — auto-derived artifact

The benchmark JSON will reconcile on next `python3 scripts/render_check.py --fail-on-doc-drift --quiet --output docs/benchmarks/render_check_latest.json` run as part of a normal renderer-change wave.

## Appendix C: References

- `docs/internal/spec/plantuml-spec.md` — per-chapter LRG support matrix
- `docs/internal/spec/audit/ch27-stdlib.md` — stdlib pack-by-pack audit
- `stdlib/README.md` — bundled stdlib pack catalog
- `docs/examples/GALLERY.md` — per-family browse index
- `docs/examples/README.md` — corpus rationale
- `tests/visual_regression/manifest.json` — visual fixtures (53 entries)
- `tests/oracle_promoted_fixtures.json` — oracle conformance gates
