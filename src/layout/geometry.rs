use crate::scene::{
    ActivationBox, GroupBox, LifecycleMarker, Lifeline, MessageLine, NoteBox, ParticipantBox,
    StructureLine,
};

pub(super) struct SceneGeometryRefs<'a> {
    pub(super) participants: &'a [ParticipantBox],
    pub(super) footboxes: &'a [ParticipantBox],
    pub(super) lifelines: &'a [Lifeline],
    pub(super) messages: &'a [MessageLine],
    pub(super) activations: &'a [ActivationBox],
    pub(super) lifecycle_markers: &'a [LifecycleMarker],
    pub(super) notes: &'a [NoteBox],
    pub(super) groups: &'a [GroupBox],
    pub(super) structures: &'a [StructureLine],
}

pub(super) struct SceneGeometryMut<'a> {
    pub(super) participants: &'a mut [ParticipantBox],
    pub(super) footboxes: &'a mut [ParticipantBox],
    pub(super) lifelines: &'a mut [Lifeline],
    pub(super) messages: &'a mut [MessageLine],
    pub(super) activations: &'a mut [ActivationBox],
    pub(super) lifecycle_markers: &'a mut [LifecycleMarker],
    pub(super) notes: &'a mut [NoteBox],
    pub(super) groups: &'a mut [GroupBox],
    pub(super) structures: &'a mut [StructureLine],
}

pub(super) fn scene_leftmost_geometry_x(geometry: SceneGeometryRefs<'_>) -> i32 {
    let participant_min = geometry
        .participants
        .iter()
        .map(|participant| participant.x)
        .min();
    let footbox_min = geometry.footboxes.iter().map(|footbox| footbox.x).min();
    let lifeline_min = geometry.lifelines.iter().map(|lifeline| lifeline.x).min();
    let message_min = geometry
        .messages
        .iter()
        .map(|message| message.x1.min(message.x2) - 8)
        .min();
    let activation_min = geometry
        .activations
        .iter()
        .map(|activation| activation.x - 5)
        .min();
    let lifecycle_min = geometry
        .lifecycle_markers
        .iter()
        .map(|marker| marker.x - 6)
        .min();
    let note_min = geometry.notes.iter().map(|note| note.x).min();
    let group_min = geometry.groups.iter().map(|group| group.x).min();
    let structure_min = geometry
        .structures
        .iter()
        .map(|structure| structure.x1.min(structure.x2))
        .min();

    participant_min
        .into_iter()
        .chain(footbox_min)
        .chain(lifeline_min)
        .chain(message_min)
        .chain(activation_min)
        .chain(lifecycle_min)
        .chain(note_min)
        .chain(group_min)
        .chain(structure_min)
        .min()
        .unwrap_or(0)
}

pub(super) fn shift_scene_geometry_x(delta: i32, geometry: SceneGeometryMut<'_>) {
    if delta <= 0 {
        return;
    }

    for participant in geometry.participants {
        participant.x += delta;
    }
    for footbox in geometry.footboxes {
        footbox.x += delta;
    }
    for lifeline in geometry.lifelines {
        lifeline.x += delta;
    }
    for message in geometry.messages {
        message.x1 += delta;
        message.x2 += delta;
    }
    for activation in geometry.activations {
        activation.x += delta;
    }
    for marker in geometry.lifecycle_markers {
        marker.x += delta;
    }
    for note in geometry.notes {
        note.x += delta;
    }
    for group in geometry.groups {
        group.x += delta;
    }
    for structure in geometry.structures {
        structure.x1 += delta;
        structure.x2 += delta;
    }
}
