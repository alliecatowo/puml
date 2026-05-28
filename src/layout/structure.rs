use std::collections::BTreeMap;

use crate::scene::{LayoutOptions, StructureKind, StructureLine};

use super::geometry::structure_bounds;
use super::text::row_units_for_height;

/// Push a structure line of the given `kind` at the current event row and
/// advance `event_rows` by the appropriate amount.
///
/// Delay, Divider, and Separator each consume one row.  Spacer consumes as
/// many rows as needed to satisfy the requested pixel height.
// Layout helper coordinates sequence event geometry (row position, vertical bounds)
// and centers collection across caller's event-processing loop; parameters reflect
// the distinct state threads and cannot be meaningfully grouped without breaking
// the caller's mutable-borrow patterns.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_structure_line(
    kind: StructureKind,
    label: Option<String>,
    pixels: Option<i32>,
    event_rows: &mut i32,
    events_top: i32,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
    structures: &mut Vec<StructureLine>,
) {
    let y = events_top + (*event_rows * options.message_row_height);
    let (x1, x2) = structure_bounds(centers_by_id, options);
    structures.push(StructureLine {
        kind,
        y,
        x1,
        x2,
        label,
    });
    if let Some(px) = pixels {
        let px = px.max(1);
        *event_rows += row_units_for_height(px, options.message_row_height);
    } else {
        *event_rows += 1;
    }
}
