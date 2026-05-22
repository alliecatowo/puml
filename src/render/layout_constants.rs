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
pub const DEFAULT_RANK_SEPARATION: f64 = 80.0;

/// Default horizontal gap between nodes that share the same rank, in user
/// units.  Applies to all diagram families that use the hierarchical layout
/// engine (component, deployment, class with relations, usecase).
pub const DEFAULT_NODE_SEPARATION: f64 = 60.0;

/// Default inset padding inside a group/package container, in user units.
/// The rendered group frame rectangle is expanded outward by this amount on
/// every side relative to the bounding box of its member nodes.
pub const DEFAULT_GROUP_PADDING: f64 = 28.0;

/// Default left/top canvas margin, in user units.  Also used as the x-origin
/// for the first node column.  Titles and package-label tabs are drawn inside
/// this margin, so it must be large enough to accommodate them.
pub const DEFAULT_CANVAS_MARGIN: f64 = 40.0;

// ─────────────────────────────────────────────────────────────────────────────
// Orthogonal edge routing (graph_layout.rs / route_edges)
// ─────────────────────────────────────────────────────────────────────────────

/// Vertical spacing between parallel edge tracks within a single inter-rank
/// routing channel, in user units.  Increasing this fans parallel edges further
/// apart; decreasing it may cause adjacent horizontal segments to overlap.
pub const TRACK_SPACING: f64 = 8.0;

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
pub const PKG_PADDING: i32 = 24;

/// Horizontal gap between component nodes inside a package, and also the
/// minimum visible gutter between adjacent package frames, in user units.
pub const PKG_INNER_GAP: i32 = 40;

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

/// Base width of the activity diagram lane area before branch/fork expansion,
/// in pixels.  Resized upward when the diagram contains if/else branches or
/// fork/join parallel sections.
pub const ACTIVITY_BASE_LANE_WIDTH: i32 = 416;

/// Horizontal offset between the centre column and each if/else branch column,
/// in pixels.  Controls the visual spread of conditional branches.
pub const ACTIVITY_BRANCH_X_OFFSET: i32 = 160;

/// Left margin of the swim-lane area within the activity canvas, in pixels.
pub const ACTIVITY_LANE_AREA_X: i32 = 32;

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
