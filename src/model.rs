mod common;
mod family;
mod sequence;
mod state;
mod structured;

pub use common::{LegendHAlign, LegendVAlign, MetadataHAlign, ScaleSpec};
pub use family::{
    FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyRelation,
    FamilyStyle, JsonProjection, MindMapSide, WbsCheckbox,
};
pub use sequence::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    SequenceMessageStyle, SequencePage, SequenceParticipantGroup, VirtualEndpoint,
    VirtualEndpointKind, VirtualEndpointSide,
};
pub use state::{
    StateDocument, StateInternalAction, StateNode, StateNodeKind, StateNodeStyle, StateTransition,
};
pub use structured::{
    ArchimateDocument, ArchimateElement, ArchimateRelation, ChartAnnotation, ChartAxis,
    ChartDocument, ChartLabelMode, ChartLegend, ChartPoint, ChartSeries, ChartSubtype,
    DitaaDocument, EbnfDocument, EbnfRule, EbnfToken, JsonDocument, JsonTreeNode, MathDocument,
    NwdiagDocument, NwdiagGroup, NwdiagNetwork, NwdiagNode, RegexDocument, RegexPattern,
    RegexToken, RepeatKind, SdlDocument, SdlState, SdlStateKind, SdlTransition,
    TimelineChronologyEvent, TimelineClosedRange, TimelineConstraint, TimelineDayMarker,
    TimelineDocument, TimelineMilestone, TimelineNamedDate, TimelineNote, TimelineOpenRange,
    TimelineResourceAllocation, TimelineResourceOffRange, TimelineSeparator, TimelineTask,
    TimelineTaskPauseRange, YamlDocument, YamlTreeNode,
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
