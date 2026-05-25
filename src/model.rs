mod chen;
mod common;
mod family;
mod sequence;
mod state;
mod stdlib;
mod structured;
mod wire;

pub use chen::{
    ChenAttribute, ChenDocument, ChenInheritance, ChenNode, ChenNodeKind, ChenRelation,
};
pub use common::{LegendHAlign, LegendVAlign, MetadataHAlign, ScaleSpec};
pub use family::{
    FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyRelation,
    FamilyRelationArrow, FamilyRelationColor, FamilyRelationDirection,
    FamilyRelationEndpointMarker, FamilyRelationLineKind, FamilyStyle, JsonProjection, MindMapSide,
    WbsCheckbox,
};
pub use sequence::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    SequenceMessageStyle, SequencePage, SequenceParticipantGroup, VirtualEndpoint,
    VirtualEndpointKind, VirtualEndpointSide,
};
pub use state::{
    StateDocument, StateInternalAction, StateNode, StateNodeKind, StateNodeStyle, StateTransition,
};
pub use stdlib::StdlibDocument;
pub use structured::{
    ArchimateDocument, ArchimateElement, ArchimateRelation, BoardCard, BoardColumn, BoardDocument,
    ChartAnnotation, ChartAxis, ChartDocument, ChartLabelMode, ChartLegend, ChartPoint,
    ChartSeries, ChartSubtype, DitaaDocument, EbnfDocument, EbnfRule, EbnfToken, FileTreeNode,
    FilesDocument, JsonDocument, JsonTreeNode, MathDocument, NwdiagDocument, NwdiagGroup,
    NwdiagNetwork, NwdiagNode, NwdiagPeerLink, RegexDocument, RegexPattern, RegexToken, RepeatKind,
    SdlDocument, SdlState, SdlStateKind, SdlTransition, TimelineChronologyEvent,
    TimelineClosedRange, TimelineConstraint, TimelineDatePrecision, TimelineDayMarker,
    TimelineDocument, TimelineMilestone, TimelineNamedDate, TimelineNote, TimelineOpenRange,
    TimelineResourceAllocation, TimelineResourceOffRange, TimelineSeparator, TimelineTask,
    TimelineTaskPauseRange, YamlDocument, YamlTreeNode,
};
pub use wire::{
    WireComponent, WireDocument, WireEndpoint, WireLabel, WireLink, WirePort, WirePortSide,
};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NormalizedDocument {
    Sequence(SequenceDocument),
    Family(FamilyDocument),
    FamilyPages(Vec<FamilyDocument>),
    Timeline(TimelineDocument),
    State(StateDocument),
    Json(JsonDocument),
    Yaml(YamlDocument),
    Nwdiag(NwdiagDocument),
    Archimate(ArchimateDocument),
    Regex(RegexDocument),
    Ebnf(EbnfDocument),
    Math(MathDocument),
    Sdl(SdlDocument),
    Ditaa(DitaaDocument),
    Chart(ChartDocument),
    Stdlib(StdlibDocument),
    Chen(ChenDocument),
    Board(BoardDocument),
    Files(FilesDocument),
    Wire(WireDocument),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_facade_preserves_common_reexports() {
        let sequence = SequenceDocument::default();
        assert_eq!(sequence.legend_halign, LegendHAlign::Center);
        assert_eq!(FamilyOrientation::LeftToRight.as_str(), "LeftToRight");
    }
}
