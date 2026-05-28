use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Which invariant was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantKind {
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
