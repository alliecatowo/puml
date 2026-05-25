use super::*;

mod component;
mod directives;
mod extended;
mod mindmap;
mod nodes;
mod relations;
mod stub;
mod timing;
mod tree;
mod visibility;

use self::component::*;
use self::directives::*;
use self::mindmap::*;
use self::nodes::*;
use self::relations::*;
use self::timing::*;
use self::visibility::*;

pub(super) fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    stub::normalize_stub_family(document)
}

pub(super) fn normalize_family_tree(document: Document) -> Result<FamilyDocument, Diagnostic> {
    tree::normalize_family_tree(document)
}

pub(super) fn normalize_extended_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    extended::normalize_extended_family(document)
}

pub(super) fn family_kind_name(kind: DiagramKind) -> &'static str {
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
        DiagramKind::Stdlib => "stdlib",
        DiagramKind::Chen => "chen",
        DiagramKind::Unknown => "unknown",
    }
}
