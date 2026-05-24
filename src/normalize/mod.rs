use std::collections::BTreeMap;

use crate::ast::{
    ActivityStepKind, ClassMember, ComponentNodeKind, DiagramKind, Document,
    ParticipantRole as AstRole, Statement, StatementKind, TimingDeclKind,
};
use crate::diagnostic::Diagnostic;
use crate::model::FamilyStyle;
use crate::model::{
    ArchimateDocument, ArchimateElement, ArchimateRelation, ChartAnnotation, ChartAxis,
    ChartDocument, ChartLegend, ChartPoint, ChartSeries, ChartSubtype, DitaaDocument, EbnfDocument,
    EbnfRule, EbnfToken, FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind,
    FamilyOrientation, FamilyRelation as ModelFamilyRelation, JsonDocument, JsonTreeNode,
    LegendHAlign, LegendVAlign, MathDocument, MetadataHAlign, MindMapSide, NormalizedDocument,
    NwdiagDocument, NwdiagGroup, NwdiagNetwork, NwdiagNode, Participant, ParticipantRole,
    RegexDocument, RegexPattern, RegexToken, RepeatKind, ScaleSpec, SdlDocument, SdlState,
    SdlStateKind, SdlTransition, SequenceDocument, SequenceEvent, SequenceEventKind,
    SequenceMessageStyle, SequencePage, StateDocument,
    StateInternalAction as ModelStateInternalAction, StateNode, StateNodeKind, StateNodeStyle,
    StateTransition as ModelStateTransition, TimelineChronologyEvent, TimelineClosedRange,
    TimelineConstraint, TimelineDayMarker, TimelineDocument, TimelineMilestone, TimelineNamedDate,
    TimelineNote, TimelineOpenRange, TimelineResourceAllocation, TimelineResourceOffRange,
    TimelineSeparator, TimelineTask, TimelineTaskPauseRange, WbsCheckbox, YamlDocument,
    YamlTreeNode,
};
use crate::scene::TextOverflowPolicy;
use crate::theme::{
    activity_style_from_sequence_theme, apply_monochrome_to_activity_style,
    apply_monochrome_to_chart_style, apply_monochrome_to_class_style,
    apply_monochrome_to_component_style, apply_monochrome_to_sequence_style,
    apply_monochrome_to_state_style, apply_monochrome_to_timing_style,
    chart_style_from_sequence_theme, class_style_from_sequence_theme, classify_activity_skinparam,
    classify_chart_skinparam, classify_class_skinparam, classify_component_skinparam,
    classify_sequence_skinparam, classify_state_skinparam, classify_timing_skinparam,
    component_style_from_sequence_theme, mindmap_style_from_sequence_theme,
    resolve_sequence_theme_preset, state_style_from_sequence_theme,
    timing_style_from_sequence_theme, ActivityStyle, ChartStyle, ClassStyle, ComponentStyle,
    MindMapDepthStyle, MindMapStyle, SequenceSkinParamSupport, SequenceSkinParamValue,
    SequenceStyle, SkinParamSupport, StateStyle, TimingStyle,
};

mod archimate;
mod chart;
mod common;
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
        DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase => {
            normalize_family_pages(document, family::normalize_stub_family)
        }
        DiagramKind::Salt => family::normalize_stub_family(document).map(NormalizedDocument::Family),
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
        DiagramKind::Unknown => Err(Diagnostic::error(
            "[E_FAMILY_UNKNOWN] unable to detect supported diagram family; expected sequence/class/object/usecase/gantt/chronology syntax",
        )),
    }
}

fn normalize_family_pages(
    document: Document,
    normalizer: fn(Document) -> Result<FamilyDocument, Diagnostic>,
) -> Result<NormalizedDocument, Diagnostic> {
    let pages = split_family_newpages(document);
    if pages.len() == 1 {
        return normalizer(pages.into_iter().next().expect("single page"))
            .map(NormalizedDocument::Family);
    }

    pages
        .into_iter()
        .map(normalizer)
        .collect::<Result<Vec<_>, _>>()
        .map(NormalizedDocument::FamilyPages)
}

fn split_family_newpages(document: Document) -> Vec<Document> {
    let kind = document.kind;
    let mut ignore_newpage = false;
    let mut common = Vec::new();
    let mut current = Vec::new();
    let mut pages = Vec::new();
    let mut seen_page_break = false;

    for stmt in document.statements {
        match &stmt.kind {
            StatementKind::IgnoreNewPage => {
                ignore_newpage = true;
                common.push(stmt);
            }
            StatementKind::NewPage(next_title) if !ignore_newpage => {
                seen_page_break = true;
                pages.push(Document {
                    kind,
                    statements: build_family_page_statements(&common, std::mem::take(&mut current)),
                });
                if let Some(title) = next_title.as_ref().filter(|title| !title.trim().is_empty()) {
                    current.push(Statement {
                        span: stmt.span,
                        kind: StatementKind::Title(title.trim().to_string()),
                    });
                }
            }
            _ if !seen_page_break && is_family_page_common_statement(&stmt.kind) => {
                common.push(stmt);
            }
            _ => current.push(stmt),
        }
    }

    pages.push(Document {
        kind,
        statements: build_family_page_statements(&common, current),
    });
    pages
}

fn build_family_page_statements(common: &[Statement], mut page: Vec<Statement>) -> Vec<Statement> {
    let mut statements = common.to_vec();
    statements.append(&mut page);
    statements
}

fn is_family_page_common_statement(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Title(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::Footbox(_)
            | StatementKind::Scale(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::HideUnlinked
            | StatementKind::Mainframe(_)
    )
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
