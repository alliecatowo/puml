use super::{InvariantKind, InvariantViolation, PseudoStateKind};

/// Assert that the flat `nodes` list (post-normalization) contains at most one
/// canonical initial pseudo-state and at most one canonical final pseudo-state
/// at each nesting level.
///
/// Returns violations describing duplicates found.
///
/// Note: this function operates on the already-normalized model (after
/// `normalize/state.rs` has run) — it is an assertion, not a deduplication
/// pass.  The normalization pass is the authoritative place where `[*]` is
/// split into initial + final; this function just verifies the invariant held.
pub fn check_pseudo_state_dedup(
    nodes: &[crate::model::StateNode],
    scope: &str,
) -> Vec<InvariantViolation> {
    use crate::model::StateNodeKind;
    let mut violations = Vec::new();

    // Count StartEnd nodes (initial pseudo-state = has outgoing transitions
    // from [*]; final is canonicalized to End).  At the flat level, only one
    // [*] node should remain.
    let start_count = nodes
        .iter()
        .filter(|n| n.kind == StateNodeKind::StartEnd)
        .count();
    if start_count > 1 {
        violations.push(InvariantViolation {
            kind: InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Initial,
                scope: scope.to_string(),
                count: start_count,
            },
            corrected: false,
            message: format!(
                "[INV-5] scope {scope:?} has {start_count} initial pseudo-states; expected ≤1"
            ),
        });
    }

    let end_count = nodes
        .iter()
        .filter(|n| n.kind == StateNodeKind::End || n.name == "[*]__end")
        .count();
    if end_count > 1 {
        violations.push(InvariantViolation {
            kind: InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Final,
                scope: scope.to_string(),
                count: end_count,
            },
            corrected: false,
            message: format!(
                "[INV-5] scope {scope:?} has {end_count} final pseudo-states; expected ≤1"
            ),
        });
    }

    violations
}
