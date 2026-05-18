pub mod ast;
pub mod creole;
pub mod diagnostic;
pub mod formatter;
pub mod language_service;
// Frontend adapters translate non-default input surfaces into PlantUML-shaped
// source before the shared parser, normalizer, layout, and renderer run.
mod frontend;
pub mod layout;
pub mod metadata;
pub mod model;
pub mod normalize;
pub mod parser;
mod preproc;
pub mod render;
pub mod scene;
pub mod source;
pub mod specialized;
pub mod theme;

pub use ast::Document;
pub use diagnostic::{Diagnostic, DiagnosticJson};
pub use metadata::{extract_metadata, DiagramMetadata};
pub use model::{
    FamilyDocument, FamilyGroup, LegendHAlign, LegendVAlign, NormalizedDocument, ScaleSpec,
    SequenceDocument, SequencePage, StateDocument, TimelineDocument,
};
pub use render::TextOutputMode;

pub use scene::{LayoutOptions, Scene};
use source::Span;
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
    /// When true, permit `!include https://...`, `!includeurl`, and `file://` URL targets.
    /// Default: false, so parsing never performs network IO or URL-addressed
    /// local file reads unless the caller opts in.
    pub allow_url_includes: bool,
}

impl Default for ParsePipelineOptions {
    fn default() -> Self {
        Self {
            frontend: FrontendSelection::Auto,
            compat: CompatMode::Strict,
            determinism: DeterminismMode::Strict,
            include_root: None,
            allow_url_includes: false,
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
    let parser_options = interpret_parser_contract(options)?;
    interpret_determinism_contract(options.determinism);

    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => {
            parser::parse_with_options(source, &parser_options)
        }
        FrontendSelection::Mermaid => {
            let adapted = frontend::mermaid::adapt_to_plantuml(source)?;
            parser::parse_with_options(&adapted, &parser_options)
        }
        FrontendSelection::Picouml => {
            let adapted = frontend::picouml::adapt_to_plantuml(source)?;
            parser::parse_with_options(&adapted, &parser_options)
        }
    }
}

pub fn preprocess_with_pipeline_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<String, Diagnostic> {
    let parser_options = interpret_parser_contract(options)?;
    interpret_determinism_contract(options.determinism);

    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => {
            parser::preprocess_with_options(source, &parser_options)
        }
        FrontendSelection::Mermaid => {
            let adapted = frontend::mermaid::adapt_to_plantuml(source)?;
            parser::preprocess_with_options(&adapted, &parser_options)
        }
        FrontendSelection::Picouml => {
            let adapted = frontend::picouml::adapt_to_plantuml(source)?;
            parser::preprocess_with_options(&adapted, &parser_options)
        }
    }
}

fn interpret_parser_contract(
    options: &ParsePipelineOptions,
) -> Result<parser::ParseOptions, Diagnostic> {
    let include_root = match options.compat {
        CompatMode::Strict => options.include_root.clone(),
        CompatMode::Extended => options.include_root.clone(),
    };
    Ok(parser::ParseOptions {
        include_root,
        allow_url_includes: options.allow_url_includes,
    })
}

fn interpret_determinism_contract(_mode: DeterminismMode) {
    // Determinism behavior is currently fully deterministic across modes.
    // Keep this explicit interpretation point to avoid split-brain routing.
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize::normalize(document)
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize::normalize_family(document)
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
    // Intercept specialized families before the main AST pipeline, but only
    // after applying the same preprocessing pass used by parse/check routes.
    if specialized::is_specialized_source(source) {
        let preprocessed =
            parser::preprocess_with_options(source, &parser::ParseOptions::default())?;
        let result = specialized::try_render_specialized(&preprocessed).ok_or_else(|| {
            Diagnostic::error(
                "[E_SPECIALIZED_PREPROC] preprocessed specialized source changed family",
            )
        })?;
        return result.map(|svg| vec![svg]);
    }
    let document = parse(source)?;
    let family = map_ast_kind_to_family(document.kind);
    render_document_for_family(document, family)
}

pub fn render_source_to_text(source: &str, mode: TextOutputMode) -> Result<String, Diagnostic> {
    let pages = render_source_to_texts(source, mode)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_texts or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_texts(
    source: &str,
    mode: TextOutputMode,
) -> Result<Vec<String>, Diagnostic> {
    let document = parse(source)?;
    let model = normalize_family(document)?;
    Ok(render::render_text_pages(&model, mode))
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
    let document = parse(source)?;
    let detected = map_ast_kind_to_family(document.kind);
    if family != detected {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_MISMATCH] requested diagram family `{}` but detected `{}`",
            family.as_str(),
            detected.as_str()
        )));
    }
    render_document_for_family(document, family)
}

fn render_document_for_family(
    document: Document,
    family: DiagramFamily,
) -> Result<Vec<String>, Diagnostic> {
    match family {
        DiagramFamily::Sequence => {
            let sequence = normalize(document)?;
            let scenes = layout::layout_pages(&sequence, LayoutOptions::default());
            Ok(scenes.iter().map(render::render_svg).collect())
        }
        DiagramFamily::Class
        | DiagramFamily::Object
        | DiagramFamily::UseCase => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Family(family_doc) => {
                Ok(vec![render::render_class_svg(&family_doc)])
            }
            model::NormalizedDocument::Sequence(_)
            | model::NormalizedDocument::Timeline(_)
            | model::NormalizedDocument::State(_) => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during family stub render",
            )),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected non-family model during family stub render",
            )),
        },
        DiagramFamily::Salt => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Family(family_doc) => {
                Ok(vec![render::render_salt_svg(&family_doc)])
            }
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during salt render",
            )),
        },
        DiagramFamily::Gantt | DiagramFamily::Chronology => {
            match normalize::normalize_family(document)? {
                model::NormalizedDocument::Timeline(timeline) => {
                    Ok(vec![render::render_timeline_svg(&timeline)])
                }
                _ => Err(Diagnostic::error(
                    "[E_TIMELINE_INTERNAL] unexpected model during timeline render",
                )),
            }
        }
        DiagramFamily::State => match normalize::normalize_family(document)? {
            model::NormalizedDocument::State(state_doc) => {
                Ok(vec![render::render_state_svg(&state_doc)])
            }
            _ => Err(Diagnostic::error(
                "[E_STATE_INTERNAL] unexpected model variant during state render",
            )),
        },
        DiagramFamily::Component => render_family_with(document, render::render_component_svg),
        DiagramFamily::Deployment => render_family_with(document, render::render_deployment_svg),
        DiagramFamily::Activity => render_family_with(document, render::render_activity_svg),
        DiagramFamily::Timing => render_family_with(document, render::render_timing_svg),
        DiagramFamily::Json => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Json(doc) => Ok(vec![render::render_json_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_JSON_INTERNAL] unexpected model during json render",
            )),
        },
        DiagramFamily::Yaml => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Yaml(doc) => Ok(vec![render::render_yaml_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_YAML_INTERNAL] unexpected model during yaml render",
            )),
        },
        DiagramFamily::Nwdiag => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Nwdiag(doc) => Ok(vec![render::render_nwdiag_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_NWDIAG_INTERNAL] unexpected model during nwdiag render",
            )),
        },
        DiagramFamily::Archimate => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Archimate(doc) => {
                Ok(vec![render::render_archimate_svg(&doc)])
            }
            _ => Err(Diagnostic::error(
                "[E_FAMILY_ARCHIMATE_INTERNAL] unexpected model during archimate render",
            )),
        },
        DiagramFamily::Regex => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Regex(doc) => Ok(vec![render::render_regex_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during regex render",
            )),
        },
        DiagramFamily::Ebnf => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Ebnf(doc) => Ok(vec![render::render_ebnf_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during ebnf render",
            )),
        },
        DiagramFamily::Math => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Math(doc) => Ok(vec![render::render_math_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during math render",
            )),
        },
        DiagramFamily::Sdl => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Sdl(doc) => Ok(vec![render::render_sdl_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during sdl render",
            )),
        },
        DiagramFamily::Ditaa => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Ditaa(doc) => Ok(vec![render::render_ditaa_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during ditaa render",
            )),
        },
        DiagramFamily::Chart => match normalize::normalize_family(document)? {
            model::NormalizedDocument::Chart(doc) => Ok(vec![render::render_chart_svg(&doc)]),
            _ => Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] unexpected model during chart render",
            )),
        },
        DiagramFamily::MindMap => render_family_with(document, render::render_mindmap_svg),
        DiagramFamily::Wbs => render_family_with(document, render::render_wbs_svg),
        DiagramFamily::Unknown => Err(unsupported_render_family_diagnostic(family)),
    }
}

fn render_family_with(
    document: Document,
    renderer: fn(&FamilyDocument) -> String,
) -> Result<Vec<String>, Diagnostic> {
    match normalize::normalize_family(document)? {
        model::NormalizedDocument::Family(doc) => Ok(vec![renderer(&doc)]),
        model::NormalizedDocument::Sequence(_) => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected sequence model during extended family render",
        )),
        model::NormalizedDocument::Timeline(_) => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected timeline model during extended family render",
        )),
        _ => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected model during extended family render",
        )),
    }
}

fn unsupported_render_family_diagnostic(family: DiagramFamily) -> Diagnostic {
    let code = match family {
        DiagramFamily::Component => "E_RENDER_COMPONENT_UNSUPPORTED",
        DiagramFamily::Deployment => "E_RENDER_DEPLOYMENT_UNSUPPORTED",
        DiagramFamily::Activity => "E_RENDER_ACTIVITY_UNSUPPORTED",
        DiagramFamily::Timing => "E_RENDER_TIMING_UNSUPPORTED",
        DiagramFamily::MindMap => "E_RENDER_MINDMAP_UNSUPPORTED",
        DiagramFamily::Wbs => "E_RENDER_WBS_UNSUPPORTED",
        DiagramFamily::Gantt => "E_RENDER_GANTT_UNSUPPORTED",
        DiagramFamily::Chronology => "E_RENDER_CHRONOLOGY_UNSUPPORTED",
        _ => "E_RENDER_FAMILY_UNSUPPORTED",
    };
    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{}` is not implemented yet; sequence is currently supported",
            family.as_str()
        ),
    )
}

pub fn render_svg_pages_from_model(model: &NormalizedDocument) -> Vec<String> {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            let scenes = layout::layout_pages(sequence, LayoutOptions::default());
            scenes.iter().map(render::render_svg).collect::<Vec<_>>()
        }
        NormalizedDocument::Family(family) => vec![render_family_document_svg(family)],
        NormalizedDocument::Timeline(timeline) => vec![render::render_timeline_svg(timeline)],
        NormalizedDocument::State(state) => vec![render::render_state_svg(state)],
        NormalizedDocument::Json(doc) => vec![render::render_json_svg(doc)],
        NormalizedDocument::Yaml(doc) => vec![render::render_yaml_svg(doc)],
        NormalizedDocument::Nwdiag(doc) => vec![render::render_nwdiag_svg(doc)],
        NormalizedDocument::Archimate(doc) => vec![render::render_archimate_svg(doc)],
        NormalizedDocument::Regex(doc) => vec![render::render_regex_svg(doc)],
        NormalizedDocument::Ebnf(doc) => vec![render::render_ebnf_svg(doc)],
        NormalizedDocument::Math(doc) => vec![render::render_math_svg(doc)],
        NormalizedDocument::Sdl(doc) => vec![render::render_sdl_svg(doc)],
        NormalizedDocument::Ditaa(doc) => vec![render::render_ditaa_svg(doc)],
        NormalizedDocument::Chart(doc) => vec![render::render_chart_svg(doc)],
    }
}

pub fn render_family_document_svg(family: &FamilyDocument) -> String {
    match family.kind {
        ast::DiagramKind::Salt => render::render_salt_svg(family),
        ast::DiagramKind::Component => render::render_component_svg(family),
        ast::DiagramKind::Deployment => render::render_deployment_svg(family),
        ast::DiagramKind::Activity => render::render_activity_svg(family),
        ast::DiagramKind::Timing => render::render_timing_svg(family),
        ast::DiagramKind::MindMap => render::render_mindmap_svg(family),
        ast::DiagramKind::Wbs => render::render_wbs_svg(family),
        _ => render::render_family_stub_svg(family),
    }
}

fn map_ast_kind_to_family(kind: ast::DiagramKind) -> DiagramFamily {
    match kind {
        ast::DiagramKind::Sequence => DiagramFamily::Sequence,
        ast::DiagramKind::Class => DiagramFamily::Class,
        ast::DiagramKind::Object => DiagramFamily::Object,
        ast::DiagramKind::UseCase => DiagramFamily::UseCase,
        ast::DiagramKind::MindMap => DiagramFamily::MindMap,
        ast::DiagramKind::Wbs => DiagramFamily::Wbs,
        ast::DiagramKind::Gantt => DiagramFamily::Gantt,
        ast::DiagramKind::Chronology => DiagramFamily::Chronology,
        ast::DiagramKind::Component => DiagramFamily::Component,
        ast::DiagramKind::Deployment => DiagramFamily::Deployment,
        ast::DiagramKind::State => DiagramFamily::State,
        ast::DiagramKind::Activity => DiagramFamily::Activity,
        ast::DiagramKind::Timing => DiagramFamily::Timing,
        ast::DiagramKind::Salt => DiagramFamily::Salt,
        ast::DiagramKind::Json => DiagramFamily::Json,
        ast::DiagramKind::Yaml => DiagramFamily::Yaml,
        ast::DiagramKind::Nwdiag => DiagramFamily::Nwdiag,
        ast::DiagramKind::Archimate => DiagramFamily::Archimate,
        ast::DiagramKind::Regex => DiagramFamily::Regex,
        ast::DiagramKind::Ebnf => DiagramFamily::Ebnf,
        ast::DiagramKind::Math => DiagramFamily::Math,
        ast::DiagramKind::Sdl => DiagramFamily::Sdl,
        ast::DiagramKind::Ditaa => DiagramFamily::Ditaa,
        ast::DiagramKind::Chart => DiagramFamily::Chart,
        ast::DiagramKind::Unknown => DiagramFamily::Unknown,
    }
}

/// Returns the LSP server capabilities object that `puml-lsp` advertises
/// during the `initialize` handshake. Exposing this here lets both the
/// `puml-lsp` binary and the `puml --dump-capabilities` CLI flag share a
/// single source of truth.
pub fn lsp_capabilities() -> serde_json::Value {
    serde_json::json!({
        "textDocumentSync":{"openClose":true,"change":2,"save":{"includeText":true}},
        "completionProvider":{"resolveProvider":true},
        "hoverProvider":true,
        "definitionProvider":true,
        "referencesProvider":true,
        "renameProvider":{"prepareProvider":true},
        "documentSymbolProvider":true,
        "workspaceSymbolProvider":true,
        "semanticTokensProvider":{"legend":{"tokenTypes":["keyword","operator","string","comment","number","type","class","function","variable","parameter","property","namespace","label","decorator","modifier"],"tokenModifiers":[]},"full":true},
        "documentFormattingProvider":true,
        "documentRangeFormattingProvider":true,
        "foldingRangeProvider":true,
        "selectionRangeProvider":true,
        "documentLinkProvider":{},
        "colorProvider":true,
        "codeActionProvider":true,
        "executeCommandProvider":{"commands":["puml.applyFormat","puml.renderSvg"]},
        "workspace":{"workspaceFolders":{"supported":true,"changeNotifications":true}}
    })
}

pub fn extract_markdown_diagrams(source: &str) -> Vec<DiagramInput> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = '`';
    let mut fence_len = 0usize;
    let mut fence_frontend = FrontendSelection::Auto;
    let mut content_start = 0usize;
    let mut cursor = 0usize;

    for line in source.split_inclusive('\n') {
        let line_start = cursor;
        cursor += line.len();

        let (marker, marker_count, rest) = parse_fence_line(line);

        if !in_fence {
            if marker_count >= 3 {
                if let Some(frontend) = parse_diagram_fence_frontend(rest) {
                    in_fence = true;
                    fence_marker = marker;
                    fence_len = marker_count;
                    fence_frontend = frontend;
                    content_start = cursor;
                }
            }
            continue;
        }

        if marker == fence_marker && marker_count >= fence_len && rest.is_empty() {
            let span = Span::new(content_start, line_start);
            out.push(DiagramInput {
                source: source[span.start.min(source.len())..span.end.min(source.len())]
                    .to_string(),
                span_in_input: span,
                fence_frontend,
            });
            in_fence = false;
            continue;
        }
    }

    if in_fence {
        let span = Span::new(content_start, source.len());
        out.push(DiagramInput {
            source: source[span.start.min(source.len())..span.end.min(source.len())].to_string(),
            span_in_input: span,
            fence_frontend,
        });
    }

    out
}

fn parse_fence_line(line: &str) -> (char, usize, &str) {
    let without_newline = line.trim_end_matches(['\n', '\r']);
    let leading_spaces = without_newline
        .as_bytes()
        .iter()
        .take_while(|&&b| b == b' ')
        .count();
    if leading_spaces > 3 {
        return ('\0', 0, without_newline);
    }

    let trimmed_line = &without_newline[leading_spaces..];
    let mut chars = trimmed_line.chars();
    let marker = match chars.next() {
        Some('`') => '`',
        Some('~') => '~',
        _ => return ('\0', 0, trimmed_line),
    };
    let marker_count = 1 + chars.take_while(|ch| *ch == marker).count();
    let rest = trimmed_line[marker_count..].trim();
    (marker, marker_count, rest)
}

fn parse_diagram_fence_frontend(info: &str) -> Option<FrontendSelection> {
    let lang = info.split_ascii_whitespace().next().unwrap_or_default();
    if lang.eq_ignore_ascii_case("mermaid") {
        return Some(FrontendSelection::Mermaid);
    }
    if lang.eq_ignore_ascii_case("picouml") {
        return Some(FrontendSelection::Picouml);
    }

    if is_plantuml_family_fence_lang(lang) {
        return Some(FrontendSelection::Auto);
    }

    None
}

fn is_plantuml_family_fence_lang(lang: &str) -> bool {
    lang.eq_ignore_ascii_case("puml")
        || lang.eq_ignore_ascii_case("pumlx")
        || lang.eq_ignore_ascii_case("plantuml")
        || lang.eq_ignore_ascii_case("uml")
        || lang.eq_ignore_ascii_case("puml-sequence")
        || lang.eq_ignore_ascii_case("uml-sequence")
}

#[cfg(test)]
mod tests {
    use super::{parse_with_pipeline_options, CompatMode, FrontendSelection, ParsePipelineOptions};

    #[test]
    fn extended_mode_without_include_root_does_not_fallback_to_cwd_in_library_api() {
        let options = ParsePipelineOptions {
            frontend: FrontendSelection::Auto,
            compat: CompatMode::Extended,
            include_root: None,
            ..ParsePipelineOptions::default()
        };
        let err = parse_with_pipeline_options("!include __puml_missing__.puml\n", &options)
            .expect_err("expected include-root diagnostic");
        assert!(
            err.message.contains("E_INCLUDE_ROOT_REQUIRED"),
            "unexpected error: {}",
            err.message
        );
    }
}
