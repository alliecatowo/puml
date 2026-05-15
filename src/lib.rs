pub mod ast;
pub mod diagnostic;
pub mod layout;
pub mod model;
pub mod normalize;
pub mod parser;
pub mod render;
pub mod scene;
pub mod source;
pub mod theme;

pub use ast::Document;
pub use diagnostic::{Diagnostic, DiagnosticJson};
pub use model::{SequenceDocument, SequencePage};
pub use scene::{LayoutOptions, Scene};
use source::Span;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramFamily {
    Sequence,
    Class,
    State,
    Activity,
    Component,
    Deployment,
    UseCase,
    Object,
    Unknown,
}

impl DiagramFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Class => "class",
            Self::State => "state",
            Self::Activity => "activity",
            Self::Component => "component",
            Self::Deployment => "deployment",
            Self::UseCase => "usecase",
            Self::Object => "object",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramInput {
    pub source: String,
    pub span_in_input: Span,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminismMode {
    Strict,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePipelineOptions {
    pub frontend: FrontendSelection,
    pub compat: CompatMode,
    pub determinism: DeterminismMode,
    pub include_root: Option<PathBuf>,
}

impl Default for ParsePipelineOptions {
    fn default() -> Self {
        Self {
            frontend: FrontendSelection::Auto,
            compat: CompatMode::Strict,
            determinism: DeterminismMode::Strict,
            include_root: None,
        }
    }
}

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parser::parse(source)
}

pub fn parse_with_pipeline_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<Document, Diagnostic> {
    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => parser::parse_with_options(
            source,
            &parser::ParseOptions {
                include_root: options.include_root.clone(),
            },
        ),
        FrontendSelection::Mermaid => Err(Diagnostic::error(
            "frontend 'mermaid' is not implemented yet",
        )),
        FrontendSelection::Picouml => Err(Diagnostic::error(
            "frontend 'picouml' is not implemented yet",
        )),
    }
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize::normalize(document)
}

pub fn detect_diagram_family(source: &str) -> Result<DiagramFamily, Diagnostic> {
    let document = parse(source)?;
    Ok(map_ast_kind_to_family(document.kind))
}

pub fn render_source_to_svg(source: &str) -> Result<String, Diagnostic> {
    let pages = render_source_to_svgs(source)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_svgs or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_svgs(source: &str) -> Result<Vec<String>, Diagnostic> {
    render_source_to_svgs_for_family(source, DiagramFamily::Sequence)
}

pub fn render_source_to_svg_for_family(
    source: &str,
    family: DiagramFamily,
) -> Result<String, Diagnostic> {
    let pages = render_source_to_svgs_for_family(source, family)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_svgs or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_svgs_for_family(
    source: &str,
    family: DiagramFamily,
) -> Result<Vec<String>, Diagnostic> {
    match family {
        DiagramFamily::Sequence => render_sequence_source_to_svgs(source),
        other => Err(Diagnostic::error(format!(
            "diagram family `{}` is not implemented yet; sequence is currently supported",
            other.as_str()
        ))),
    }
}

fn render_sequence_source_to_svgs(source: &str) -> Result<Vec<String>, Diagnostic> {
    let document = parse(source)?;
    let sequence = normalize(document)?;
    let scenes = layout::layout_pages(&sequence, LayoutOptions::default());
    Ok(scenes.iter().map(render::render_svg).collect())
}

fn map_ast_kind_to_family(kind: ast::DiagramKind) -> DiagramFamily {
    match kind {
        ast::DiagramKind::Sequence => DiagramFamily::Sequence,
        ast::DiagramKind::Unknown => DiagramFamily::Unknown,
    }
}

pub fn extract_markdown_diagrams(source: &str) -> Vec<DiagramInput> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_len = 0usize;
    let mut content_start = 0usize;
    let mut cursor = 0usize;

    for line in source.split_inclusive('\n') {
        let line_start = cursor;
        cursor += line.len();

        let trimmed = line.trim_start();
        let backticks = trimmed.chars().take_while(|ch| *ch == '`').count();
        let rest = trimmed[backticks..].trim();

        if !in_fence {
            if backticks >= 3 && is_diagram_fence_info(rest) {
                in_fence = true;
                fence_len = backticks;
                content_start = cursor;
            }
            continue;
        }

        if backticks >= fence_len && rest.is_empty() {
            let span = Span::new(content_start, line_start);
            out.push(DiagramInput {
                source: source[span.start.min(source.len())..span.end.min(source.len())]
                    .to_string(),
                span_in_input: span,
            });
            in_fence = false;
            continue;
        }
    }

    out
}

fn is_diagram_fence_info(info: &str) -> bool {
    let lang = info.split_ascii_whitespace().next().unwrap_or_default();
    lang.eq_ignore_ascii_case("puml")
        || lang.eq_ignore_ascii_case("plantuml")
        || lang.eq_ignore_ascii_case("uml")
        || lang.eq_ignore_ascii_case("puml-sequence")
        || lang.eq_ignore_ascii_case("uml-sequence")
}
