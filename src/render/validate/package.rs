use super::edge::extract_relation_segments;
use super::{InvariantKind, InvariantViolation};

/// A package/group bounding box with its header strip.
#[derive(Debug, Clone)]
pub struct PackageFrame {
    pub id: String,
    /// Top-left y of the entire package frame.
    pub y: i32,
    /// Height of the label/header strip at the top.
    pub header_height: i32,
}

/// Check invariant #4: edge segments must not pass through package header strips.
///
/// Returns violations.  Auto-correction requires re-routing the edge path, which
/// is left to the layout engine — this pass records violations for diagnostics.
pub fn check_package_headers(svg: &str, frames: &[PackageFrame]) -> Vec<InvariantViolation> {
    let relations = extract_relation_segments(svg);
    let mut violations = Vec::new();

    for (from, to, segs) in &relations {
        for frame in frames {
            let header_top = frame.y;
            let header_bot = frame.y + frame.header_height;
            for seg in segs {
                // Check if the segment's y range overlaps the header strip.
                let seg_min_y = seg.y1.min(seg.y2);
                let seg_max_y = seg.y1.max(seg.y2);
                if seg_min_y < header_bot && seg_max_y > header_top {
                    violations.push(InvariantViolation {
                        kind: InvariantKind::EdgeThroughPackageHeader {
                            from: from.clone(),
                            to: to.clone(),
                            package: frame.id.clone(),
                        },
                        corrected: false,
                        message: format!(
                            "[INV-4] edge {from:?}→{to:?} passes through package {:?} header strip [y={}, h={}]",
                            frame.id, frame.y, frame.header_height
                        ),
                    });
                    break;
                }
            }
        }
    }

    violations
}
