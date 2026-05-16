pub mod ast;
pub mod creole;
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
pub use model::{
    FamilyDocument, FamilyGroup, LegendHAlign, LegendVAlign, NormalizedDocument, ScaleSpec,
    SequenceDocument, SequencePage, TimelineDocument,
};
pub use scene::{LayoutOptions, Scene};
use source::Span;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramFamily {
    Sequence,
    Class,
    Gantt,
    Chronology,
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
    let parser_options = interpret_parser_contract(options)?;
    interpret_determinism_contract(options.determinism);

    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => {
            parser::parse_with_options(source, &parser_options)
        }
        FrontendSelection::Mermaid => {
            let adapted = adapt_mermaid_to_plantuml(source)?;
            parser::parse_with_options(&adapted, &parser_options)
        }
        FrontendSelection::Picouml => {
            let adapted = adapt_picouml_to_plantuml(source)?;
            parser::parse_with_options(&adapted, &parser_options)
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
    Ok(parser::ParseOptions { include_root })
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
    let document = parse(source)?;
    let family = map_ast_kind_to_family(document.kind);
    render_document_for_family(document, family)
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
        DiagramFamily::Class | DiagramFamily::Object | DiagramFamily::UseCase | DiagramFamily::Salt => {
            match normalize::normalize_family(document)? {
                model::NormalizedDocument::Family(family_doc) => {
                    Ok(vec![render::render_class_svg(&family_doc)])
                }
                model::NormalizedDocument::Sequence(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected sequence model during family render",
                )),
                model::NormalizedDocument::Timeline(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected timeline model during family render",
                )),
                _ => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected non-family model during family stub render",
                )),
            }
        }
        DiagramFamily::Gantt | DiagramFamily::Chronology => {
            match normalize::normalize_family(document)? {
                model::NormalizedDocument::Timeline(timeline) => {
                    Ok(vec![render::render_timeline_svg(&timeline)])
                }
                model::NormalizedDocument::Sequence(_) => Err(Diagnostic::error(
                    "[E_TIMELINE_INTERNAL] unexpected sequence model during timeline render",
                )),
                model::NormalizedDocument::Family(_) => Err(Diagnostic::error(
                    "[E_TIMELINE_INTERNAL] unexpected family model during timeline render",
                )),
                _ => Err(Diagnostic::error(
                    "[E_TIMELINE_INTERNAL] unexpected model during timeline render",
                )),
            }
        }
        DiagramFamily::Component => render_family_with(document, render::render_component_svg),
        DiagramFamily::Deployment => render_family_with(document, render::render_deployment_svg),
        DiagramFamily::State => render_family_with(document, render::render_state_svg),
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
        DiagramFamily::MindMap
        | DiagramFamily::Wbs
        | DiagramFamily::Unknown => Err(unsupported_render_family_diagnostic(family)),
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
        DiagramFamily::State => "E_RENDER_STATE_UNSUPPORTED",
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

fn adapt_picouml_to_plantuml(source: &str) -> Result<String, Diagnostic> {
    let mut out = String::new();
    let mut saw_picouml_markers = false;
    let mut saw_uml_markers = false;
    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if matches_prefixed_uml_marker(trimmed, "@startpicouml") {
            saw_picouml_markers = true;
            let converted = replace_prefixed_marker(raw_line, "@startpicouml", "@startuml");
            out.push_str(&converted);
            out.push('\n');
            continue;
        }
        if matches_prefixed_uml_marker(trimmed, "@endpicouml") {
            saw_picouml_markers = true;
            let converted = replace_prefixed_marker(raw_line, "@endpicouml", "@enduml");
            out.push_str(&converted);
            out.push('\n');
            continue;
        }
        if matches_prefixed_uml_marker(trimmed, "@startuml")
            || matches_prefixed_uml_marker(trimmed, "@enduml")
        {
            saw_uml_markers = true;
        }
        out.push_str(raw_line);
        out.push('\n');
    }

    if saw_picouml_markers && saw_uml_markers {
        return Err(Diagnostic::error_code(
            "E_PICOUML_MARKER_MIXED",
            "picouml frontend does not allow mixing `@startpicouml/@endpicouml` with `@startuml/@enduml` markers",
        ));
    }

    Ok(out)
}

fn replace_prefixed_marker(line: &str, marker: &str, replacement: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let marker_len = marker.len();
    if !lower.trim_start().starts_with(marker) {
        return line.to_string();
    }
    let leading_ws = line.len() - line.trim_start().len();
    let rest_start = leading_ws + marker_len;
    let mut out = String::new();
    out.push_str(&line[..leading_ws]);
    out.push_str(replacement);
    out.push_str(line.get(rest_start..).unwrap_or_default());
    out
}

fn matches_prefixed_uml_marker(line: &str, marker: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let marker_len = marker.len();
    if !lower.starts_with(marker) {
        return false;
    }
    let rest = &line[marker_len..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn adapt_mermaid_to_plantuml(source: &str) -> Result<String, Diagnostic> {
    let mut out = Vec::new();
    let mut saw_non_empty = false;
    let mut saw_sequence_header = false;
    let mut offset = 0usize;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_non_empty {
            saw_non_empty = true;
            if line.eq_ignore_ascii_case("sequenceDiagram") {
                saw_sequence_header = true;
                continue;
            }
            return Err(Diagnostic::error_code(
                "E_MERMAID_FAMILY_UNSUPPORTED",
                "mermaid frontend currently supports sequence diagrams only (expected `sequenceDiagram`)",
            )
            .with_span(span));
        }

        if let Some(converted) = adapt_mermaid_declaration(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_message(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_note(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_lifecycle(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_block(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_create_destroy(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_link(line) {
            out.push(converted);
            continue;
        }

        if line.eq_ignore_ascii_case("autonumber") {
            out.push("autonumber".to_string());
            continue;
        }

        if let Some(title) = line.strip_prefix("title ") {
            if !title.trim().is_empty() {
                out.push(format!("title {}", title.trim()));
                continue;
            }
        }

        if let Some(code) = classify_unsupported_mermaid_construct(line) {
            return Err(Diagnostic::error_code(
                code,
                format!("unsupported mermaid sequence construct: `{line}`"),
            )
            .with_span(span));
        }

        return Err(Diagnostic::error_code(
            "E_MERMAID_CONSTRUCT_UNSUPPORTED",
            format!("unsupported mermaid sequence construct: `{line}`"),
        )
        .with_span(span));
    }

    if !saw_sequence_header {
        return Err(Diagnostic::error_code(
            "E_MERMAID_EMPTY",
            "mermaid sequence input is empty or missing `sequenceDiagram` header",
        ));
    }

    Ok(out.join("\n"))
}

fn adapt_mermaid_declaration(line: &str) -> Option<String> {
    let mut words = line.split_ascii_whitespace();
    let head = words.next()?;
    if !matches!(head, "participant" | "actor") {
        return None;
    }
    let tail = words.collect::<Vec<_>>().join(" ");
    if tail.is_empty() {
        return None;
    }
    Some(format!("{head} {tail}"))
}

fn adapt_mermaid_message(line: &str) -> Option<String> {
    let (core, label) = line.split_once(':')?;
    let (from, arrow, to) = split_mermaid_message_core(core.trim())?;
    let mapped_arrow = match arrow {
        "->>" => "->>",
        "-->>" => "-->>",
        "->" => "->",
        "-->" => "-->",
        _ => return None,
    };

    Some(format!(
        "{} {} {}: {}",
        from.trim(),
        mapped_arrow,
        to.trim(),
        label.trim()
    ))
}

fn adapt_mermaid_note(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let (head, body) = line.split_once(':')?;
    let prefix = &head["note ".len()..];
    let body = body.trim();
    if body.is_empty() {
        return None;
    }

    let lower_prefix = prefix.to_ascii_lowercase();
    if lower_prefix.starts_with("over ") {
        let target = prefix["over ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note over {target}: {body}"));
    }
    if lower_prefix.starts_with("left of ") {
        let target = prefix["left of ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note left of {target}: {body}"));
    }
    if lower_prefix.starts_with("right of ") {
        let target = prefix["right of ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note right of {target}: {body}"));
    }
    None
}

fn adapt_mermaid_lifecycle(line: &str) -> Option<String> {
    let mut parts = line.split_ascii_whitespace();
    let head = parts.next()?;
    if !matches!(head, "activate" | "deactivate" | "destroy") {
        return None;
    }
    let target = parts.collect::<Vec<_>>().join(" ");
    if target.is_empty() {
        return None;
    }
    Some(format!("{head} {target}"))
}

fn split_mermaid_message_core(core: &str) -> Option<(&str, &str, &str)> {
    for arrow in ["-->>", "->>", "-->", "->"] {
        if let Some(idx) = core.find(arrow) {
            let lhs = core[..idx].trim();
            let rhs = core[idx + arrow.len()..].trim();
            if lhs.is_empty() || rhs.is_empty() {
                return None;
            }
            return Some((lhs, arrow, rhs));
        }
    }
    None
}

fn strip_mermaid_comment(line: &str) -> &str {
    line.split_once("%%").map_or(line, |(prefix, _)| prefix)
}

fn classify_unsupported_mermaid_construct(_line: &str) -> Option<&'static str> {
    // All previously-unsupported block/create/destroy/link constructs now
    // have explicit adapter routes (see `adapt_mermaid_block`,
    // `adapt_mermaid_create_destroy`, `adapt_mermaid_link`). Leaving this
    // hook in place keeps the diagnostic shape stable in case we need to
    // re-introduce targeted classifications later.
    None
}

fn adapt_mermaid_block(line: &str) -> Option<String> {
    let first = line.split_ascii_whitespace().next()?.to_ascii_lowercase();
    match first.as_str() {
        "alt" => {
            let label = line["alt".len()..].trim();
            Some(if label.is_empty() {
                "alt".to_string()
            } else {
                format!("alt {label}")
            })
        }
        "else" => {
            let label = line["else".len()..].trim();
            Some(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            })
        }
        "opt" => {
            let label = line["opt".len()..].trim();
            Some(if label.is_empty() {
                "opt".to_string()
            } else {
                format!("opt {label}")
            })
        }
        "loop" => {
            let label = line["loop".len()..].trim();
            Some(if label.is_empty() {
                "loop".to_string()
            } else {
                format!("loop {label}")
            })
        }
        "par" => {
            let label = line["par".len()..].trim();
            Some(if label.is_empty() {
                "par".to_string()
            } else {
                format!("par {label}")
            })
        }
        "and" => {
            // Mermaid's `and` inside a par maps to PlantUML's `else` branch.
            let label = line["and".len()..].trim();
            Some(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            })
        }
        "critical" => {
            let label = line["critical".len()..].trim();
            Some(if label.is_empty() {
                "critical".to_string()
            } else {
                format!("critical {label}")
            })
        }
        "option" => {
            // Mermaid `option` inside `critical` maps to PlantUML's `else`.
            let label = line["option".len()..].trim();
            Some(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            })
        }
        "break" => {
            let label = line["break".len()..].trim();
            Some(if label.is_empty() {
                "break".to_string()
            } else {
                format!("break {label}")
            })
        }
        "rect" => {
            // `rect rgb(...)` becomes a `group` block (color is dropped).
            let label = line["rect".len()..].trim();
            Some(if label.is_empty() {
                "group".to_string()
            } else {
                format!("group {label}")
            })
        }
        "box" => {
            let label = line["box".len()..].trim();
            Some(if label.is_empty() {
                "box".to_string()
            } else {
                format!("box {label}")
            })
        }
        "end" => Some("end".to_string()),
        _ => None,
    }
}

fn adapt_mermaid_create_destroy(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("create ") {
        // Mermaid form: `create participant X` or `create X`.
        let trimmed = line[7..].trim();
        let payload = if let Some(p) = trimmed.strip_prefix("participant ") {
            p.trim()
        } else if let Some(p) = trimmed.strip_prefix("actor ") {
            p.trim()
        } else {
            trimmed
        };
        if payload.is_empty() || rest.trim().is_empty() {
            return None;
        }
        return Some(format!("create {payload}"));
    }
    if let Some(rest) = lower.strip_prefix("destroy ") {
        let target = line[8..].trim();
        if target.is_empty() || rest.trim().is_empty() {
            return None;
        }
        return Some(format!("destroy {target}"));
    }
    None
}

fn adapt_mermaid_link(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("link ") || lower.starts_with("links ")) {
        return None;
    }
    // We don't render real links yet, but we accept the syntax by collapsing
    // it to a benign comment-style placeholder that the downstream parser
    // will skip without complaint.
    Some(format!("' [link] {}", line.trim()))
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
