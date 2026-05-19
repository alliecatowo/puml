use std::collections::BTreeMap;

use crate::ast::{
    ActivityStepKind, ClassMember, ComponentNodeKind, DiagramKind, Document,
    ParticipantRole as AstRole, StatementKind, TimingDeclKind,
};
use crate::diagnostic::Diagnostic;
use crate::model::FamilyStyle;
use crate::model::{
    ArchimateDocument, ArchimateElement, ArchimateRelation, ChartAnnotation, ChartAxis,
    ChartDocument, ChartLegend, ChartPoint, ChartSeries, ChartSubtype, ChenDocument, DitaaDocument,
    EbnfDocument, EbnfRule, EbnfToken, FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind,
    FamilyOrientation, FamilyRelation as ModelFamilyRelation, JsonDocument, JsonTreeNode,
    LegendHAlign, LegendVAlign, MathDocument, MindMapSide, NormalizedDocument, NwdiagDocument,
    NwdiagGroup, NwdiagNetwork, NwdiagNode, Participant, ParticipantRole, RegexDocument,
    RegexPattern, RegexToken, RepeatKind, ScaleSpec, SdlDocument, SdlState, SdlStateKind,
    SdlTransition, SequenceDocument, SequenceEvent, SequenceEventKind, SequenceMessageStyle,
    SequencePage, StateDocument, StateInternalAction as ModelStateInternalAction, StateNode,
    StateNodeKind, StateTransition as ModelStateTransition, TimelineChronologyEvent,
    TimelineClosedRange, TimelineConstraint, TimelineDocument, TimelineMilestone,
    TimelineOpenRange, TimelineResourceAllocation, TimelineSeparator, TimelineTask,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide, WbsCheckbox, YamlDocument,
    YamlTreeNode,
};
use crate::scene::TextOverflowPolicy;
use crate::theme::{
    activity_style_from_sequence_theme, chart_style_from_sequence_theme,
    class_style_from_sequence_theme, classify_activity_skinparam, classify_chart_skinparam,
    classify_class_skinparam, classify_component_skinparam, classify_sequence_skinparam,
    classify_state_skinparam, classify_timing_skinparam, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, state_style_from_sequence_theme,
    timing_style_from_sequence_theme, ActivityStyle, ChartStyle, ClassStyle, ComponentStyle,
    SequenceSkinParamSupport, SequenceSkinParamValue, SequenceStyle, SkinParamSupport, StateStyle,
    TimingStyle,
};

mod archimate;
mod chart;
mod chen;
mod ebnf;
mod family;
mod nwdiag;
mod raw;
mod regex;
mod sdl;
mod sequence;
mod state;
mod structured;
mod timeline;

#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub include_root: Option<std::path::PathBuf>,
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize_family_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family_with_options(
    document: Document,
    options: &NormalizeOptions,
) -> Result<NormalizedDocument, Diagnostic> {
    match document.kind {
        DiagramKind::Sequence => {
            sequence::normalize_with_options(document, options).map(NormalizedDocument::Sequence)
        }
        DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase | DiagramKind::Salt => {
            family::normalize_stub_family(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Gantt | DiagramKind::Chronology => {
            timeline::normalize_timeline_baseline(document).map(NormalizedDocument::Timeline)
        }
        DiagramKind::State => state::normalize_state(document).map(NormalizedDocument::State),
        DiagramKind::MindMap | DiagramKind::Wbs => {
            family::normalize_family_tree(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Json => structured::normalize_json_document(document).map(NormalizedDocument::Json),
        DiagramKind::Yaml => structured::normalize_yaml_document(document).map(NormalizedDocument::Yaml),
        DiagramKind::Nwdiag => {
            nwdiag::normalize_nwdiag_document(document).map(NormalizedDocument::Nwdiag)
        }
        DiagramKind::Archimate => {
            archimate::normalize_archimate_document(document).map(NormalizedDocument::Archimate)
        }
        DiagramKind::Regex => regex::normalize_regex(document).map(NormalizedDocument::Regex),
        DiagramKind::Ebnf => ebnf::normalize_ebnf(document).map(NormalizedDocument::Ebnf),
        DiagramKind::Math => raw::normalize_math(document).map(NormalizedDocument::Math),
        DiagramKind::Sdl => sdl::normalize_sdl(document).map(NormalizedDocument::Sdl),
        DiagramKind::Ditaa => raw::normalize_ditaa(document).map(NormalizedDocument::Ditaa),
        DiagramKind::Chart => chart::normalize_chart(document).map(NormalizedDocument::Chart),
        DiagramKind::Component
        | DiagramKind::Deployment
        | DiagramKind::Activity
        | DiagramKind::Timing => family::normalize_extended_family(document).map(NormalizedDocument::Family),
        DiagramKind::Chen => chen::normalize_chen(document).map(NormalizedDocument::Chen),
        DiagramKind::Unknown => Err(Diagnostic::error(
            "[E_FAMILY_UNKNOWN] unable to detect supported diagram family; expected sequence/class/object/usecase/gantt/chronology syntax",
        )),
    }
}

pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    sequence::paginate(document)
}

pub fn normalize_with_options(
    document: Document,
    options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    sequence::normalize_with_options(document, options)
}

pub(super) fn collect_raw_body(document: &Document) -> (Option<String>, Vec<String>) {
    let mut title: Option<String> = None;
    let mut body: Vec<String> = Vec::new();
    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::Title(v) => title = Some(v.clone()),
            StatementKind::RawBody(line) => {
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix("title ") {
                    if title.is_none() {
                        title = Some(rest.trim().to_string());
                    }
                    continue;
                }
                if trimmed.eq_ignore_ascii_case("title") {
                    continue;
                }
                body.push(line.clone());
            }
            _ => {}
        }
    }
    (title, body)
}

pub(super) fn collect_raw_block(document: &Document) -> (String, Option<String>) {
    let mut lines: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::RawBlockContent(s) | StatementKind::RawBody(s) => lines.push(s.clone()),
            StatementKind::Title(v) => title = Some(v.clone()),
            _ => {}
        }
    }
    (lines.join("\n"), title)
}
