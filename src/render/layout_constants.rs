//! Named constants for layout and rendering geometry.
//!
//! This module centralises the tuneable numbers that appear throughout the
//! render pipeline so that:
//!
//! 1. **Reviewability** — a constant like `PKG_TAB_HEIGHT = 40` with a doc
//!    comment is far easier to inspect in a code review than a bare `40`
//!    buried inside geometry math.
//! 2. **Single grep target** — anyone wanting to understand "where does the
//!    40-pixel package-tab come from?" can search for `PKG_TAB_HEIGHT` rather
//!    than chasing bare literals across six files.
//! 3. **Future tuning** — when the layout engine gains a user-configurable
//!    spacing knob, changing one constant here propagates everywhere.
//!
//! **All values are identical to the literals they replace.**  This module
//! does not change any behaviour; it is a pure rename/extract refactor.

// ─────────────────────────────────────────────────────────────────────────────
// Graph layout defaults (graph_layout.rs / LayoutOptions::default)
// ─────────────────────────────────────────────────────────────────────────────

/// Default vertical gap between rank rows in the hierarchical Sugiyama layout,
/// in user units (pixels at 1× scale).  Increasing this widens diagrams
/// vertically; decreasing it can cause package-frame labels to overlap the
/// nodes in the rank above.
///
/// Tuned to approximate PlantUML's default rank separation (~40px) as part of
/// the parity density retune (#1346). The constraint
/// `DEFAULT_RANK_SEPARATION >= DEFAULT_NODE_SEPARATION` is preserved via the
/// compile-time assertion below.
pub const DEFAULT_RANK_SEPARATION: f64 = 44.0;

/// Default horizontal gap between nodes that share the same rank, in user
/// units.  Applies to all diagram families that use the hierarchical layout
/// engine (component, deployment, class with relations, usecase).
///
/// Tuned to approximate PlantUML's default node separation (~30px) as part of
/// the parity density retune (#1346).
pub const DEFAULT_NODE_SEPARATION: f64 = 30.0;

/// Default inset padding inside a group/package container, in user units.
/// The rendered group frame rectangle is expanded outward by this amount on
/// every side relative to the bounding box of its member nodes.
///
/// Tuned to approximate PlantUML's default group padding (~10-12px) as part
/// of the parity density retune (#1346).
pub const DEFAULT_GROUP_PADDING: f64 = 12.0;

/// Default left/top canvas margin, in user units.  Also used as the x-origin
/// for the first node column.  Titles and package-label tabs are drawn inside
/// this margin, so it must be large enough to accommodate them.
///
/// Tuned to approximate PlantUML's default canvas margin (~5-10px) as part of
/// the parity density retune (#1346).
pub const DEFAULT_CANVAS_MARGIN: f64 = 8.0;

// ─────────────────────────────────────────────────────────────────────────────
// Orthogonal edge routing (graph_layout.rs / route_edges)
// ─────────────────────────────────────────────────────────────────────────────

/// Vertical spacing between parallel edge tracks within a single inter-rank
/// routing channel, in user units.  Increasing this fans parallel edges further
/// apart; decreasing it may cause adjacent horizontal segments to overlap.
pub const TRACK_SPACING: f64 = 8.0;

/// Horizontal spacing between adjacent endpoint port slots when multiple
/// adjacent-rank cross edges share the same node endpoint (top or bottom) in
/// one channel. Keeps K2,2-style backend edges from collapsing into one
/// ambiguous center port.
pub const EDGE_PORT_FAN_SPACING: f64 = 10.0;

/// Maximum absolute horizontal shift applied by endpoint port fanning, in user
/// units. Caps fan-out to avoid overcorrection on dense hubs.
pub const EDGE_PORT_FAN_MAX_SHIFT: f64 = 18.0;

/// Soft upper bound on the number of edge tracks allocated per routing channel
/// before track indices wrap (greedy assignment; wrapping is safe — it just
/// means two edges may share a track in extreme graphs).
pub const MAX_TRACKS: usize = 12;

/// Height of the package-header band that orthogonal edge routing must not
/// cross, in user units.  Equals the package label-tab height
/// (`PKG_TAB_HEIGHT`) plus a 8-pixel safety margin so arrow shafts clear the
/// label text.
pub const PKG_HEADER_ROUTING_CLEARANCE: f64 = 48.0;

/// Maximum number of collision-resolution passes in the post-layout
/// group-bounds overlap fixer.  In practice 1–2 passes are always sufficient;
/// the cap prevents an infinite loop on degenerate inputs.
pub const GROUP_COLLISION_MAX_PASSES: usize = 4;

/// Minimum horizontal gap between two group bounding boxes after the
/// collision-resolution shift, in user units.  Ensures adjacent package frames
/// never touch even after the shift.
pub const GROUP_COLLISION_MIN_GAP: f64 = 40.0;

// ─────────────────────────────────────────────────────────────────────────────
// Package / group frame geometry (family.rs component & class renderers)
// ─────────────────────────────────────────────────────────────────────────────

/// Height of the package label tab drawn at the top of a component/deployment
/// package frame, in user units.  Must match the `label_reserve` used in
/// `compute_group_bounds` (graph_layout.rs) so that group bounding boxes
/// accurately reflect the rendered frame top.
pub const PKG_TAB_HEIGHT: i32 = 40;

/// Horizontal padding inside a component/deployment package frame, in user
/// units.  The frame rectangle is expanded outward by this amount on the left
/// and right sides relative to its members' bounding box.
///
/// Tuned to approximate PlantUML's package padding (~10-12px) as part of the
/// parity density retune (#1346). The compile-time invariant
/// `PKG_PADDING < PKG_INNER_GAP` is preserved below.
pub const PKG_PADDING: i32 = 12;

/// Horizontal gap between component nodes inside a package, and also the
/// minimum visible gutter between adjacent package frames, in user units.
///
/// Tuned to approximate PlantUML's package inner gap (~20px) as part of the
/// parity density retune (#1346).
pub const PKG_INNER_GAP: i32 = 20;

/// Width of a component box in the component/deployment renderer, in user
/// units.  All component nodes share this fixed width; height varies with label
/// line count.
pub const COMPONENT_BOX_WIDTH: i32 = 200;

/// Height of a component box in the component/deployment renderer (single-line
/// label), in user units.
pub const COMPONENT_BOX_HEIGHT: i32 = 80;

/// Canvas margin used in the component/deployment renderer, in user units.
/// Titles and package-label tabs are drawn inside this margin. Derived from
/// `DEFAULT_CANVAS_MARGIN` so the component renderer and graph layout keep the
/// same outer gutter.
pub const COMPONENT_CANVAS_MARGIN: i32 = DEFAULT_CANVAS_MARGIN as i32;

// ─────────────────────────────────────────────────────────────────────────────
// Component diagram per-node geometry (box_grid.rs / render_component_artifact)
//
// Tuned as part of the component-family density retune to close the 2-5× area
// gap vs PlantUML for component fixtures (02_interfaces 4.09×, 07_ports 2.89×,
// 08_cloud_db_queue 3.43×).  These constants override the shared
// COMPONENT_BOX_WIDTH/HEIGHT defaults (which remain as graph-layout fallbacks)
// when family == "component" in box_grid.rs.  PlantUML renders component nodes
// at roughly 120×50px; 140×56 gives comfortable label legibility while closing
// the density gap to ≤2.0× on all three audit fixtures.
// ─────────────────────────────────────────────────────────────────────────────

/// Width of a component node when rendered in the component diagram family,
/// in user units.  Narrower than the shared `COMPONENT_BOX_WIDTH` default to
/// produce denser layouts closer to PlantUML's output.
pub const COMPONENT_NODE_BOX_WIDTH: i32 = 130;

/// Height of a component node when rendered in the component diagram family
/// (single-line label), in user units.  Shorter than the shared
/// `COMPONENT_BOX_HEIGHT` default to reduce vertical whitespace.
pub const COMPONENT_NODE_BOX_HEIGHT: i32 = 50;

/// Extra vertical gap added to `cell_h + inner_gap` when computing rank
/// separation for the component diagram family, in user units.  Smaller than
/// the 40px used as the generic default, matching PlantUML's tighter inter-rank
/// spacing for component diagrams.
pub const COMPONENT_RANK_EXTRA_GAP: f64 = 8.0;

// Class diagram geometry (class_render.rs / class_layout.rs)
//
// Tuned as part of the per-family density retune to close the 2-4× area gap
// vs PlantUML for class fixtures identified in the wave-4 audit:
//   - class/01_basic               3.24×  (worst class fixture)
//   - class/03_composition         2.99×
//   - class/11_generics            2.50×  (post-#1383 correctness win)
//   - class/05_visibility          1.85×
// These constants are class-specific and intentionally isolated from the
// component/deployment/usecase families.
// ─────────────────────────────────────────────────────────────────────────────

/// Minimum width of a class node box before auto-sizing takes over, in user
/// units.  Auto-sizing widens the box when node names or member text require
/// more space (clamp upper bound 600).  Reduced from the old 160px to 130px,
/// then tightened to 120px in the wave-7 cross-family pass-2 density sweep to
/// closer match PlantUML's compact class box sizing (~110–125px for typical
/// short names).
pub const CLASS_BOX_MIN_WIDTH: i32 = 120;

/// Horizontal margin (left/right gutter) inside the class diagram canvas, in
/// user units.  The left margin is where the first column of node boxes starts;
/// the right margin (passed as `canvas_right_margin`) is the gutter after the
/// rightmost node.  Reduced from 32px to 16px in wave-4 retune, then to 8px
/// in the wave-7 cross-family pass-2 sweep to tighten horizontal whitespace
/// toward PlantUML parity (PlantUML uses ~4–8px outer gutter for class).
pub const CLASS_MARGIN_X: i32 = 8;

/// Horizontal gap between adjacent class nodes in the same rank (node
/// separation fed to the hierarchical layout engine), in user units.  Also the
/// minimum gap driven by relation-label width.  Reduced from 80px to 40px to
/// reduce horizontal spread on multi-column diagrams.
pub const CLASS_COL_GAP: i32 = 40;

/// Bottom-to-top inter-rank gap for class diagrams, in user units.  This value
/// is passed directly as `rank_separation` in the `LayoutOptions` fed to the
/// hierarchical layout engine.  The engine adds `max_node_height` to the
/// row-advance separately (see `coordinates.rs`), so this constant represents
/// only the whitespace between the bottom edge of one rank's tallest node and
/// the top edge of the next rank.  Reduced from 40px to 30px in the wave-7
/// cross-family pass-2 density sweep to approximate PlantUML's tighter
/// inter-rank gap (~25–30px) for class diagrams.
pub const CLASS_ROW_GAP: i32 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// Object diagram geometry (class_render.rs — object-specific density retune)
//
// PlantUML renders object boxes at ~120px wide with tight inter-element gaps.
// These constants override the wider class-diagram defaults when the renderer
// detects an all-Object-node diagram (is_object_diagram_layout check).
//
// Tuned to bring area ratios below 2.0× for the two wave-4 audit fixtures
// (object/02_with_attributes, object/05_ch04_parity) — refs #1425.
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum node width for object diagrams, in user units.
///
/// PlantUML renders object boxes at roughly 120px wide (content-driven but
/// capped tight).  Class diagrams use 160–600px.  Capping objects at 160
/// brings the horizontal canvas much closer to PlantUML parity while still
/// accommodating typical attribute-value text (#1425).
pub const OBJECT_NODE_WIDTH_MAX: i32 = 130;

/// Horizontal column gap between adjacent object nodes, in user units.
///
/// Tuned to ~20px to match PlantUML's tight horizontal spacing for object
/// diagrams.  Class diagrams use 80px or relation-label-driven spacing (#1425).
pub const OBJECT_COL_GAP: i32 = 20;

/// Vertical row gap between object diagram ranks, in user units.
///
/// Tuned to ~20px to approximate PlantUML's compact vertical spacing for
/// object diagrams.  Class diagrams use 64px (#1425).
pub const OBJECT_ROW_GAP: i32 = 20;

/// Horizontal canvas margin for object diagrams, in user units.
///
/// Object diagrams use a narrower outer margin than class diagrams (8px vs
/// 32px) to match PlantUML's tight canvas boundary (#1425).
pub const OBJECT_MARGIN_X: i32 = 8;

/// Top canvas margin for object diagrams, in user units.
///
/// Matched to OBJECT_MARGIN_X; titles and group labels add on top of this
/// value via the standard header-block computation (#1425).
pub const OBJECT_MARGIN_TOP: i32 = 8;

// ─────────────────────────────────────────────────────────────────────────────
// Sequence diagram geometry (sequence.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Vertical gap between consecutive label lines inside a sequence-message
/// label block, in pixels.  A value of 16 gives comfortable line spacing at
/// the default 12-pixel font size.
pub const MESSAGE_LABEL_LINE_GAP: i32 = 16;

/// Height of the "ref" combined-fragment header notch, in pixels.  The notch
/// visually separates the "ref" keyword from the fragment body content.
pub const REF_HEADER_HEIGHT: i32 = 20;

/// Y-baseline offset of the first body-text line inside a "ref"
/// combined-fragment relative to the fragment's top-left corner, in pixels.
/// Positions body text below the header notch with comfortable clearance.
pub const REF_BODY_BASELINE_Y: i32 = 32;

// ─────────────────────────────────────────────────────────────────────────────
// Deployment diagram geometry (box_grid.rs / render_deployment_artifact)
//
// Tuned as part of the per-shape density retune (#1426) to close the 2-5× area
// gap vs PlantUML for deployment fixtures (02_databases 4.90×, 03_cloud 3.68×,
// 06_kubernetes 2.21×).  These constants are intentionally smaller than the
// component-family equivalents: PlantUML renders deployment nodes at ~114×44px
// with ~60px rank separation, versus component nodes at ~120×50px.  Using
// separate constants lets us retune deployment without affecting component.
// ─────────────────────────────────────────────────────────────────────────────

/// Width of a deployment node in the deployment renderer, in user units.
/// Tuned to approximate PlantUML's default node width (~114px) for the
/// deployment family, reducing the 4.90× area ratio on 02_databases (#1426).
pub const DEPLOYMENT_BOX_WIDTH: i32 = 110;

/// Height of a deployment node in the deployment renderer (single-line label),
/// in user units.  Tuned to approximate PlantUML's default node height (~44px).
pub const DEPLOYMENT_BOX_HEIGHT: i32 = 44;

/// Extra gap added on top of DEPLOYMENT_BOX_HEIGHT to compute the deployment
/// rank separation, in user units.  Reduced from 30px to 16px in the wave-7
/// cross-family pass-2 density sweep: when there are no package groups the
/// 2×PKG_PADDING overhead is also skipped (see `box_grid.rs`), yielding
/// `DEPLOYMENT_BOX_HEIGHT + inner_gap + DEPLOYMENT_RANK_EXTRA_GAP` = 80px,
/// which is closer to PlantUML's ~75–85px effective inter-rank gap for flat
/// deployment diagrams.
pub const DEPLOYMENT_RANK_EXTRA_GAP: f64 = 16.0;

// ─────────────────────────────────────────────────────────────────────────────
// Activity diagram geometry (activity/mod.rs, activity/layout.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Height of each activity step slot, in pixels.  Every node (action, start,
/// stop, decision, fork/join) occupies one slot of this height in the vertical
/// layout pass.
pub const ACTIVITY_STEP_HEIGHT: i32 = 60;

/// Y offset within a step slot at which the outgoing arrow exits the node, in
/// pixels from the slot top.  Chosen so the arrow exits just below the visible
/// shape bottom for all node types.
pub const ACTIVITY_ARROW_OUT_OFFSET: i32 = 42;

// ─────────────────────────────────────────────────────────────────────────────
// Compile-time relational invariant assertions.
//
// These fire at compile time (const-eval) so a future edit that violates the
// layout's internal consistency requirements fails loudly rather than silently
// producing corrupt geometry.  Using `const { assert!() }` avoids the
// `clippy::assertions_on_constants` lint.
// ─────────────────────────────────────────────────────────────────────────────

// rank_separation >= node_separation: ranks further apart than nodes-in-rank
// prevents visually confusing layouts.
const _: () = const { assert!(DEFAULT_RANK_SEPARATION >= DEFAULT_NODE_SEPARATION) };

// PKG_HEADER_ROUTING_CLEARANCE > PKG_TAB_HEIGHT: the routing clearance includes
// a safety margin so edge shafts don't collide with the package label text.
const _: () = const { assert!(PKG_HEADER_ROUTING_CLEARANCE as i32 > PKG_TAB_HEIGHT) };

// PKG_PADDING < PKG_INNER_GAP: padding alone must not push adjacent frames
// together before the inter-frame gap is applied.
const _: () = const { assert!(PKG_PADDING < PKG_INNER_GAP) };

// ACTIVITY_ARROW_OUT_OFFSET < ACTIVITY_STEP_HEIGHT: the arrow exit must be
// within the step slot.
const _: () = const { assert!(ACTIVITY_ARROW_OUT_OFFSET < ACTIVITY_STEP_HEIGHT) };

// REF_BODY_BASELINE_Y > REF_HEADER_HEIGHT: body text must start below the
// header notch.
const _: () = const { assert!(REF_BODY_BASELINE_Y > REF_HEADER_HEIGHT) };
