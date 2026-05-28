use std::collections::BTreeMap;

use crate::model::ParticipantRole;
use crate::scene::{LayoutOptions, LifecycleMarker, LifecycleMarkerKind, ParticipantBox};

use super::geometry::default_center;
use super::text::row_units_for_height;

/// Handle a `destroy <Participant>` event during layout.
///
/// Emits an X-marker lifecycle event at the current event row and advances
/// `event_rows` by one.
pub(super) fn handle_destroy_event(
    id: &str,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
    events_top: i32,
    event_rows: &mut i32,
    lifecycle_markers: &mut Vec<LifecycleMarker>,
) {
    let y = events_top + (*event_rows * options.message_row_height);
    let x = centers_by_id
        .get(id)
        .copied()
        .unwrap_or_else(|| default_center(options));
    lifecycle_markers.push(LifecycleMarker {
        participant_id: id.to_owned(),
        x,
        y,
        kind: LifecycleMarkerKind::Destroy,
    });
    *event_rows += 1;
}

/// Handle a `create <Participant>` event during layout.
///
/// For mid-flow created participants (those in `created_participants`), this
/// renders their header box inline at the creation row *and* emits a
/// `sequence-create` lifecycle marker so the `sequence-create` CSS class
/// is present in the SVG output.
///
/// For participants not in `created_participants` (i.e., the `create`
/// keyword was used for a participant that already appears in the header),
/// only the lifecycle marker is emitted.
pub(super) fn handle_create_event(
    id: &str,
    created_participants: &std::collections::BTreeSet<String>,
    created_display_lines: &BTreeMap<String, (Vec<String>, ParticipantRole)>,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
    events_top: i32,
    event_rows: &mut i32,
    participant_height: i32,
    participants: &mut Vec<ParticipantBox>,
    lifecycle_markers: &mut Vec<LifecycleMarker>,
) {
    let y = events_top + (*event_rows * options.message_row_height);
    let x = centers_by_id
        .get(id)
        .copied()
        .unwrap_or_else(|| default_center(options));

    // Always emit the lifecycle marker so `sequence-create` CSS class is
    // present in the SVG for all create events.
    lifecycle_markers.push(LifecycleMarker {
        participant_id: id.to_owned(),
        x,
        y,
        kind: LifecycleMarkerKind::Create,
    });

    if created_participants.contains(id) {
        // Mid-flow created participant: also render the header box at this row.
        let participant_x = x - options.participant_width / 2;
        let (display_lines, role) = created_display_lines
            .get(id)
            .cloned()
            .unwrap_or_else(|| (vec![id.to_owned()], ParticipantRole::Participant));
        participants.push(ParticipantBox {
            id: id.to_owned(),
            display_lines,
            role,
            x: participant_x,
            y,
            width: options.participant_width,
            height: participant_height,
        });
        *event_rows += row_units_for_height(participant_height, options.message_row_height);
    }
}
