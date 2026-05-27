use crate::ast::Document;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramFamily {
    Sequence,
    Class,
    State,
    Activity,
    Timing,
    Component,
    Deployment,
    UseCase,
    Object,
    Salt,
    MindMap,
    Wbs,
    Gantt,
    Chronology,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
    Regex,
    Ebnf,
    Math,
    Sdl,
    Ditaa,
    Chart,
    Stdlib,
    Chen,
    Board,
    Files,
    Wire,
    Unknown,
}

impl DiagramFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Class => "class",
            Self::Gantt => "gantt",
            Self::Chronology => "chronology",
            Self::State => "state",
            Self::Activity => "activity",
            Self::Timing => "timing",
            Self::Component => "component",
            Self::Deployment => "deployment",
            Self::UseCase => "usecase",
            Self::Object => "object",
            Self::Salt => "salt",
            Self::MindMap => "mindmap",
            Self::Wbs => "wbs",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Nwdiag => "nwdiag",
            Self::Archimate => "archimate",
            Self::Regex => "regex",
            Self::Ebnf => "ebnf",
            Self::Math => "math",
            Self::Sdl => "sdl",
            Self::Ditaa => "ditaa",
            Self::Chart => "chart",
            Self::Stdlib => "stdlib",
            Self::Chen => "chen",
            Self::Board => "board",
            Self::Files => "files",
            Self::Wire => "wire",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramInput {
    pub source: String,
    pub span_in_input: Span,
    pub fence_frontend: FrontendSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendSelection {
    Auto,
    Plantuml,
    Mermaid,
    Picouml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatMode {
    Strict,
    Extended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePipelineOptions {
    pub frontend: FrontendSelection,
    pub compat: CompatMode,
    pub include_root: Option<PathBuf>,
    /// When true, permit `!include https://...`, `!includeurl`, and `file://` URL targets.
    /// Default: false, so parsing never performs network IO or URL-addressed
    /// local file reads unless the caller opts in.
    pub allow_url_includes: bool,
    /// Variables pre-injected before preprocessing begins (e.g. from CLI `-D`
    /// flags). Keys are variable names; values are the string value. Variables
    /// are available as `$KEY` in diagram source.
    pub inject_vars: BTreeMap<String, String>,
}

impl Default for ParsePipelineOptions {
    fn default() -> Self {
        Self {
            frontend: FrontendSelection::Auto,
            compat: CompatMode::Strict,
            include_root: None,
            allow_url_includes: false,
            inject_vars: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsePipelineResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct PreprocessPipelineResult {
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
}
