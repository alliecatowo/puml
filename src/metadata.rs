use std::collections::BTreeMap;

use serde::Serialize;

use crate::ast::{DiagramKind, Document, StatementKind};
use crate::diagnostic::{Diagnostic, Severity};
use crate::model::{FamilyNodeKind, NormalizedDocument, SequenceEventKind};
use crate::normalize;

pub const METADATA_SCHEMA: &str = "puml.metadata";
pub const METADATA_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagramMetadata {
    pub schema: &'static str,
    pub schema_version: u32,
    pub family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub counts: BTreeMap<String, usize>,
    pub warnings: Vec<MetadataWarning>,
    pub skinparams: Vec<SkinParamMetadata>,
    pub themes: Vec<String>,
    pub pages: Vec<PageMetadata>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MetadataWarning {
    pub severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkinParamMetadata {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PageMetadata {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub event_count: usize,
}

pub fn extract_metadata(document: &Document, model: &NormalizedDocument) -> DiagramMetadata {
    let mut counts = BTreeMap::new();
    let (family, title, warnings, skinparams, pages) = match model {
        NormalizedDocument::Sequence(sequence) => {
            counts.insert("participants".to_string(), sequence.participants.len());
            counts.insert(
                "messages".to_string(),
                sequence
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, SequenceEventKind::Message { .. }))
                    .count(),
            );
            counts.insert(
                "notes".to_string(),
                sequence
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, SequenceEventKind::Note { .. }))
                    .count(),
            );
            counts.insert(
                "groups".to_string(),
                sequence
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, SequenceEventKind::GroupStart { .. }))
                    .count(),
            );

            let pages = normalize::paginate(sequence)
                .into_iter()
                .enumerate()
                .map(|(idx, page)| PageMetadata {
                    index: idx + 1,
                    title: page.title,
                    event_count: page.events.len(),
                })
                .collect::<Vec<_>>();
            counts.insert("pages".to_string(), pages.len());

            (
                "sequence".to_string(),
                sequence.title.clone(),
                metadata_warnings(&sequence.warnings),
                sequence
                    .skinparams
                    .iter()
                    .map(|(key, value)| SkinParamMetadata {
                        key: key.clone(),
                        value: value.clone(),
                    })
                    .collect::<Vec<_>>(),
                pages,
            )
        }
        NormalizedDocument::Family(family) => {
            if matches!(family.kind, DiagramKind::Class) {
                counts.insert(
                    "classes".to_string(),
                    family
                        .nodes
                        .iter()
                        .filter(|node| matches!(node.kind, FamilyNodeKind::Class))
                        .count(),
                );
                counts.insert("relations".to_string(), family.relations.len());
            } else {
                counts.insert("nodes".to_string(), family.nodes.len());
                counts.insert("relations".to_string(), family.relations.len());
                counts.insert("groups".to_string(), family.groups.len());
            }
            (
                diagram_kind_name(family.kind).to_string(),
                family.title.clone(),
                metadata_warnings(&family.warnings),
                ast_skinparams(document),
                Vec::new(),
            )
        }
        NormalizedDocument::Timeline(timeline) => {
            counts.insert("tasks".to_string(), timeline.tasks.len());
            counts.insert("milestones".to_string(), timeline.milestones.len());
            counts.insert("constraints".to_string(), timeline.constraints.len());
            counts.insert(
                "chronology_events".to_string(),
                timeline.chronology_events.len(),
            );
            (
                diagram_kind_name(timeline.kind).to_string(),
                timeline.title.clone(),
                metadata_warnings(&timeline.warnings),
                ast_skinparams(document),
                Vec::new(),
            )
        }
        NormalizedDocument::State(state) => {
            counts.insert("nodes".to_string(), state.nodes.len());
            counts.insert("transitions".to_string(), state.transitions.len());
            (
                "state".to_string(),
                state.title.clone(),
                metadata_warnings(&state.warnings),
                ast_skinparams(document),
                Vec::new(),
            )
        }
        NormalizedDocument::Json(doc) => metadata_for_simple(
            "json",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("nodes", doc.nodes.len())],
            &mut counts,
        ),
        NormalizedDocument::Yaml(doc) => metadata_for_simple(
            "yaml",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("nodes", doc.nodes.len())],
            &mut counts,
        ),
        NormalizedDocument::Nwdiag(doc) => metadata_for_simple(
            "nwdiag",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [
                ("networks", doc.networks.len()),
                ("groups", doc.groups.len()),
            ],
            &mut counts,
        ),
        NormalizedDocument::Archimate(doc) => metadata_for_simple(
            "archimate",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [
                ("elements", doc.elements.len()),
                ("relations", doc.relations.len()),
            ],
            &mut counts,
        ),
        NormalizedDocument::Regex(doc) => metadata_for_simple(
            "regex",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("patterns", doc.patterns.len())],
            &mut counts,
        ),
        NormalizedDocument::Ebnf(doc) => metadata_for_simple(
            "ebnf",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("rules", doc.rules.len())],
            &mut counts,
        ),
        NormalizedDocument::Math(doc) => metadata_for_simple(
            "math",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("body_bytes", doc.body.len())],
            &mut counts,
        ),
        NormalizedDocument::Sdl(doc) => metadata_for_simple(
            "sdl",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [
                ("states", doc.states.len()),
                ("transitions", doc.transitions.len()),
            ],
            &mut counts,
        ),
        NormalizedDocument::Ditaa(doc) => metadata_for_simple(
            "ditaa",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [("body_bytes", doc.body.len())],
            &mut counts,
        ),
        NormalizedDocument::Chart(doc) => metadata_for_simple(
            "chart",
            doc.title.clone(),
            &doc.warnings,
            ast_skinparams(document),
            [
                ("data_points", doc.data.len()),
                ("series", doc.series.len()),
            ],
            &mut counts,
        ),
    };

    DiagramMetadata {
        schema: METADATA_SCHEMA,
        schema_version: METADATA_SCHEMA_VERSION,
        family,
        title,
        counts,
        warnings,
        skinparams,
        themes: ast_themes(document),
        pages,
    }
}

fn metadata_for_simple<const N: usize>(
    family: &str,
    title: Option<String>,
    warnings: &[Diagnostic],
    skinparams: Vec<SkinParamMetadata>,
    entries: [(&str, usize); N],
    counts: &mut BTreeMap<String, usize>,
) -> (
    String,
    Option<String>,
    Vec<MetadataWarning>,
    Vec<SkinParamMetadata>,
    Vec<PageMetadata>,
) {
    for (key, value) in entries {
        counts.insert(key.to_string(), value);
    }
    (
        family.to_string(),
        title,
        metadata_warnings(warnings),
        skinparams,
        Vec::new(),
    )
}

fn metadata_warnings(warnings: &[Diagnostic]) -> Vec<MetadataWarning> {
    warnings
        .iter()
        .map(|warning| MetadataWarning {
            severity: match warning.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            code: diagnostic_code(&warning.message),
            message: warning.message.clone(),
        })
        .collect()
}

fn diagnostic_code(message: &str) -> Option<String> {
    let rest = message.strip_prefix('[')?;
    let (code, _) = rest.split_once("] ")?;
    (!code.is_empty()).then(|| code.to_string())
}

fn ast_skinparams(document: &Document) -> Vec<SkinParamMetadata> {
    document
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::SkinParam { key, value } => Some(SkinParamMetadata {
                key: key.clone(),
                value: value.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn ast_themes(document: &Document) -> Vec<String> {
    document
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::Theme(theme) => Some(theme.clone()),
            _ => None,
        })
        .collect()
}

fn diagram_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Unknown => "unknown",
    }
}
