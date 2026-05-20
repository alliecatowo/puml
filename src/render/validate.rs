//! Render-time invariants pass: makes visually-broken diagrams structurally impossible.
//!
//! This module enforces correctness invariants on the completed SVG output.
//! Each invariant either auto-corrects (mutating the SVG string in place) or
//! emits a structured diagnostic explaining what would have been broken.
//!
//! Priority order (from the issue body):
//!   1. Edge-vs-node non-intersection  [auto-correct: segment re-route]
//!   2. Label-inside-viewBox           [auto-correct: expand viewBox]
//!   3. Label-vs-edge-stroke clearance [auto-correct: background rect]
//!   4. Package-header reservation     [auto-correct: segment re-route]
//!   5. Pseudo-state deduplication     [normalization assertion — see normalize/state.rs]
//!   6. Edge endpoint connectivity     [diagnostic only]
//!   7. Self-loop row allocation       [diagnostic only]
//!
//! The main entry point is [`run`].

use std::fmt;

mod edge;
mod package;
mod semantic;
mod state;
mod svg;
mod text;

pub use edge::{
    check_edge_node_clearance, check_endpoint_connectivity, check_label_edge_clearance, Segment,
};
pub use package::{check_package_headers, PackageFrame};
pub use semantic::{
    check_canonical_graph_hooks, check_canonical_semantic_hook_attrs,
    check_labels_clear_non_owner_nodes, check_primary_node_non_overlap,
    check_semantic_bboxes_inside_viewbox, expand_viewbox_to_semantic_bboxes,
    extract_semantic_edges, extract_semantic_labels, extract_semantic_nodes, NodeBbox,
    SemanticEdge, SemanticLabel, SemanticNode,
};
pub use state::check_pseudo_state_dedup;
pub use text::check_labels_inside_viewbox;

pub(crate) use svg::parse_viewbox;

/// Which invariant was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantKind {
    /// An edge segment passed through a non-endpoint node bounding box.
    EdgeCrossesNode {
        /// SVG `data-uml-from` attribute of the offending relation.
        from: String,
        /// SVG `data-uml-to` attribute of the offending relation.
        to: String,
        /// ID of the node whose bounding box was crossed.
        node_id: String,
    },
    /// A `<text>` element's estimated bounding box extends outside the viewBox.
    LabelOutsideViewbox {
        /// Approximate text content.
        snippet: String,
        /// How many pixels outside the right edge.
        overflow_px: i32,
    },
    /// A relation label has insufficient clearance from the edge stroke.
    LabelEdgeClearance {
        from: String,
        to: String,
        clearance_px: i32,
    },
    /// An edge segment passes through a package/group header strip.
    EdgeThroughPackageHeader {
        from: String,
        to: String,
        package: String,
    },
    /// Duplicate pseudo-states detected at normalization time.
    DuplicatePseudoState {
        kind: PseudoStateKind,
        scope: String,
        count: usize,
    },
    /// An edge endpoint does not connect to its declared node port.
    FloatingEndpoint {
        from: String,
        to: String,
        side: EndpointSide,
    },
    /// A self-loop does not have enough vertical space for the label.
    SelfLoopTooShort {
        node: String,
        allocated_px: i32,
        minimum_px: i32,
    },
    /// A canonical `data-puml-bbox` lies outside the root SVG viewBox.
    SemanticBBoxOutsideViewbox {
        role: SemanticRole,
        id: String,
        overflow_px: i32,
    },
    /// Two primary canonical `puml-node` boxes overlap.
    PrimaryNodeOverlap { a: String, b: String },
    /// A canonical `puml-label` overlaps a `puml-node` that it does not own.
    LabelOverlapsNonOwnerNode {
        owner: String,
        label_kind: String,
        node_id: String,
    },
    /// A graph-profile render is missing one of the canonical `puml-*` hooks.
    CanonicalGraphHookMissing { element: String, hook: String },
}

/// Whether the invariant was auto-corrected or only recorded as a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoCorrect {
    /// Mutate the SVG/model to correct the violation silently.
    Apply,
    /// Emit a diagnostic but do not mutate.
    EmitDiagnostic,
}

/// A recorded invariant violation.
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub kind: InvariantKind,
    pub corrected: bool,
    pub message: String,
}

/// Which pseudo-state kind is duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoStateKind {
    Initial,
    Final,
}

/// Which endpoint of the edge is floating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointSide {
    Source,
    Target,
}

/// Semantic SVG role extracted from canonical `puml-*` hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRole {
    Node,
    Edge,
    Label,
}

/// Optional validation profile requested by visual/invariant callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphValidationProfile {
    None,
    Graph,
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}",
            if self.corrected {
                "CORRECTED"
            } else {
                "VIOLATION"
            },
            self.message
        )
    }
}

/// Result of a full invariant run.
#[derive(Debug, Clone, Default)]
pub struct InvariantReport {
    pub violations: Vec<InvariantViolation>,
    pub expansions: usize,
    pub background_rects_added: usize,
}

/// Run all applicable SVG-level invariants on a completed SVG render.
///
/// `mode` controls whether auto-corrections are applied to the SVG string.
///
/// This is the main entry point; call it at the end of every render function.
pub fn run(svg: &mut String, mode: AutoCorrect) -> InvariantReport {
    let mut report = InvariantReport::default();

    // Invariant #2: labels inside viewBox (auto-correct: expand viewBox).
    // This is safe to auto-correct at render time because it only expands the
    // viewBox dimensions — it never changes any element positions.
    {
        let v = check_labels_inside_viewbox(svg, mode);
        let expansions = v.iter().filter(|x| x.corrected).count();
        report.expansions += expansions;
        report.violations.extend(v);
    }

    // Invariant #3: label-vs-edge-stroke clearance.
    // NOT applied in run() because scanning all text elements against all
    // edge segments produces false positives on node header/stereotype labels
    // that happen to be near edges.  Callers that have precise label/edge
    // coordinates (e.g. the sequence renderer) should call
    // check_label_edge_clearance() directly.
    // The function is still available and tested via the public API.

    // Invariant #1: edge-vs-node intersection (diagnostic; layout engine auto-corrects)
    report.violations.extend(check_edge_node_clearance(svg));

    // Invariant #6: edge endpoint connectivity (diagnostic)
    report.violations.extend(check_endpoint_connectivity(svg));

    report
}

#[cfg(test)]
mod tests;
