# Chapter 13 — Network Diagram (nwdiag)

Audit of PlantUML nwdiag syntax against the puml Rust renderer.
Source: `/tmp/puml-spec/ch13-network-diagram-with-nwdiag.txt`.

### 13.0 `nwdiag { ... }` container — ✅
**Feature:** Network-diagram outer block, either via `@startnwdiag` or `@startuml`+`nwdiag {`.
**Status:** ✅
**Evidence:** `src/parser/blocks.rs:51` `@startnwdiag`, `src/normalize/nwdiag.rs:12` skips `nwdiag {` opener.
**Notes:** Block kind `Nwdiag`; routes to `NormalizedDocument::Nwdiag`.

### 13.1 Define a network (`network NAME { address = "..." }`) — ✅
**Feature:** Single network with CIDR address.
**Syntax example:** `network dmz { address = "210.x.x.x/24" }`
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:19-37` parses `network ` prefix; `:76-84` parses `address = "..."`.

### 13.1.2 / 13.1.3 Nodes inside network with address attr — ✅
**Feature:** `web01 [address = "210.x.x.1"]` and bare-name nodes.
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:95-99`, `:166-211` `parse_nwdiag_node_entry`; nodes accumulate in `NwdiagNetwork.nodes`.

### 13.2 Multiple addresses (comma-separated) — ✅
**Feature:** `address = "210.x.x.1, 210.x.x.20"`.
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:213-227` `parse_nwdiag_addresses` splits on comma via `archimate::split_csv_args`. Stored in `NwdiagNode.addresses` (`src/model.rs:174`).

### 13.3 Groups inside network — ✅
**Feature:** `group NAME { node1; node2; }` inside a network.
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:39-65` handles `group` keyword; collects nodes into `NwdiagGroup`.
**Notes:** Group is closed by network's `}` or its own `}`; ordering accepted.

### 13.3.2 Groups outside network definitions — ✅
**Feature:** Top-level `group { color = "#FFAAAA"; web01; web02; }`.
**Status:** ✅
**Evidence:** Same code path; `current_group` is independent of `current` network (`src/normalize/nwdiag.rs:39-65`, `:102-124`).

### 13.3.4 / 13.3.5 Multiple groups with color attrs — ✅
**Feature:** Multiple groups, each with `color = "#XXX"`.
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:103-110` parses `color`, `description`, `shape`, `style` for groups. Render overlay at `src/render/specialized/nwdiag.rs:118-150` colors group boxes.

### 13.4 Extended network/group syntax (color, description, shape) — ✅
**Feature:** `network X { color = "red" description = "..." }`, and `[shape = database]` on nodes.
**Status:** ✅
**Evidence:** `src/normalize/nwdiag.rs:85-94` network-level color/description/shape/style; `:186-198` node-level color/shape/style/label.

### 13.5 Sprites (`<$application_server>` in description) — 🟡
**Feature:** Sprite references inside `description = "<$sprite>\n web01"`.
**Status:** 🟡
**Evidence:** description string is captured (`src/normalize/nwdiag.rs:190`), but sprite-token rendering in nwdiag node bodies is not visible in `src/render/specialized/nwdiag.rs`. Likely emitted as literal text.
**Notes:** `!include <office/...>` may parse via preproc; sprite-into-svg substitution for nwdiag specifically — unverified, suspect text-only.

### 13.6 OpenIconic in descriptions (`<&clock>`, `<&cog*4>`) — 🟡
**Feature:** OpenIconic icon refs + size multiplier in `description`.
**Status:** 🟡
**Evidence:** Captured as raw text; no OpenIconic inline-run renderer hook found in nwdiag specialized path.
**Notes:** Same text-only limitation as 13.5.

### 13.7 Same nodes on more than two networks (`jump line`) — 🟡
**Feature:** A node defined in 3+ networks should draw vertical jump connectors.
**Status:** 🟡
**Evidence:** `src/render/specialized/nwdiag.rs` lays out nodes in columns per name (col widths logic at `:21-46`). Whether multi-network jump lines render correctly across 3+ networks is unverified — needs visual gate.

### 13.8 Peer networks (`inet -- router`) — ✅
**Feature:** Direct node-to-node connection outside a busbar network.
**Syntax example:** `inet [shape = cloud]; inet -- router;`
**Status:** ✅
**Evidence:** Top-level node declarations and peer-link chains are normalized at `src/normalize/nwdiag.rs:131-155`, stored on `NwdiagDocument.peer_links` / `top_level_nodes` (`src/model.rs:176-194`), and rendered as `nwdiag-peer-link` paths plus `nwdiag-toplevel` nodes in `src/render/specialized/nwdiag.rs:336-428`.
**Notes:** Covered by `tests/ch13_nwdiag_parity.rs:4-32`.

### 13.9 Peer networks combined with groups — ✅
**Feature:** Peer links + groups together.
**Status:** ✅
**Evidence:** Group overlays are computed from the unified node-rect map, which now includes network nodes and rendered top-level peers (`src/render/specialized/nwdiag.rs:108-221`), so peer endpoints and grouped members can coexist in one topology.
**Notes:** Covered by `tests/ch13_nwdiag_parity.rs:4-32`.

### 13.10 Title / header / footer / legend / caption — 🟡
**Feature:** `title`, `header`, `footer`, `legend ... end legend`, `caption` on nwdiag.
**Status:** 🟡
**Evidence:** `title` captured via `collect_raw_block` (`src/normalize/nwdiag.rs:4`). header/footer/legend/caption support is parser-level common; not confirmed plumbed through to nwdiag SVG output.
**Notes:** Needs verification on the renderer side.

### 13.11 With or without shadow (`<style> root { shadowing 0 }`) — ❌
**Feature:** Global shadow toggle via style.
**Status:** ❌
**Evidence:** `src/render/specialized/nwdiag.rs` has no style-engine integration visible for shadowing toggles.
**Notes:** Default appears to be no shadowing.

### 13.12 Network `width = full` — ✅
**Feature:** Per-network `width = full` to extend the busbar to common width.
**Status:** ✅
**Evidence:** `width = full` is captured in `NwdiagNetwork.width_full` (`src/model.rs:186-194`, `src/normalize/nwdiag.rs:88-97`) and applied by `network_geometry(...)` so only flagged networks expand to the shared full bus width (`src/render/specialized/nwdiag.rs:231-267`, `:447-470`).
**Notes:** Covered by `tests/ch13_nwdiag_parity.rs:18-25`.

### 13.13 Other internal networks (TCP/IP/USB/SERIAL via `switch -- equip` chain) — ✅
**Feature:** Chained peer link statements outside `network { }` blocks.
**Syntax example:** `switch -- equip; equip -- printer;`
**Status:** ✅
**Evidence:** Chained `A -- B -- C` statements are expanded into adjacent pairs during normalization (`src/normalize/nwdiag.rs:136-145`) and rendered as separate peer-link paths (`src/render/specialized/nwdiag.rs:403-428`).
**Notes:** Covered by `tests/ch13_nwdiag_parity.rs:8-11`.

### 13.14 Global style (`<style> nwdiagDiagram { network { ... } server { ... } arrow { ... } group { ... } }`) — ❌
**Feature:** Per-scope skinning (network, server, arrow, group) via `<style>`.
**Status:** ❌
**Evidence:** No style hooks in `src/render/specialized/nwdiag.rs` for these scopes; colors are hard-coded fall-backs (e.g. `#fef3c7` for groups at render specialized:146).
**Notes:** Network-level `color` attribute IS honored, but `<style>` block is not.

### 13.15 Shape inventory (actor, agent, artifact, boundary, card, cloud, collections, component, control, database, entity, file, folder, frame, hexagon, interface, label, node, package, person, queue, stack, rectangle, storage, usecase) — 🟡
**Feature:** Full PlantUML shape vocabulary on nwdiag nodes.
**Status:** 🟡
**Evidence:** `shape` attr is captured (`src/normalize/nwdiag.rs:191`); render path checks at least `cloud` (`src/render/specialized/nwdiag.rs:240`). Other shapes likely fall through to default rectangle — needs visual gate over the corpus.
**Notes:** Spec itself flags `hexagon` and `folder` overlap as broken in PlantUML; gap here is the same long tail.

---

**Tally ch13 (17 subsections audited):** ✅ 11 · 🟡 4 · ❌ 2
