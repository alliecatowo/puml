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
pub use model::{
    FamilyDocument, NormalizedDocument, SequenceDocument, SequencePage, StateDocument,
    TimelineDocument,
};

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
    MindMap,
    Wbs,
    Gantt,
    Chronology,
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
            Self::MindMap => "mindmap",
            Self::Wbs => "wbs",
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
        DiagramFamily::Class | DiagramFamily::Object | DiagramFamily::UseCase => {
            match normalize::normalize_family(document)? {
                model::NormalizedDocument::Family(stub) => {
                    Ok(vec![render::render_family_stub_svg(&stub)])
                }
                model::NormalizedDocument::Sequence(_)
                | model::NormalizedDocument::Timeline(_)
                | model::NormalizedDocument::State(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected model during family stub render",
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
        DiagramFamily::Gantt | DiagramFamily::Chronology => {
            match normalize::normalize_family(document)? {
                model::NormalizedDocument::Timeline(timeline) => {
                    Ok(vec![render::render_timeline_svg(&timeline)])
                }
                model::NormalizedDocument::Sequence(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected sequence model during timeline render",
                )),
                model::NormalizedDocument::Family(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected family model during timeline render",
                )),
                model::NormalizedDocument::State(_) => Err(Diagnostic::error(
                    "[E_FAMILY_STUB_INTERNAL] unexpected state model during timeline render",
                )),
            }
        }
        DiagramFamily::Component
        | DiagramFamily::Deployment
        | DiagramFamily::Activity
        | DiagramFamily::Timing
        | DiagramFamily::MindMap
        | DiagramFamily::Wbs
        | DiagramFamily::Unknown => Err(unsupported_render_family_diagnostic(family)),
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

/// Top-level Mermaid → PlantUML adapter.  Inspects the leading directive and
/// routes to the appropriate family-specific sub-adapter.
fn adapt_mermaid_to_plantuml(source: &str) -> Result<String, Diagnostic> {
    // Scan for the first non-empty, non-comment line to detect the family.
    let mut first_directive: Option<(&str, Span)> = None;
    let mut offset = 0usize;
    for raw_line in source.lines() {
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        first_directive = Some((line, span));
        break;
    }

    let (directive, directive_span) = match first_directive {
        Some(d) => d,
        None => {
            return Err(Diagnostic::error_code(
                "E_MERMAID_EMPTY",
                "mermaid input is empty or contains only comments",
            ));
        }
    };

    let lower = directive.to_ascii_lowercase();
    // Route by leading directive keyword.
    if lower == "sequencediagram" {
        return adapt_mermaid_sequence(source);
    }
    if lower.starts_with("flowchart ") || lower.starts_with("graph ") {
        return adapt_mermaid_flowchart(source);
    }
    if lower == "classdiagram" {
        return adapt_mermaid_classdiagram(source);
    }
    if lower == "statediagram" || lower == "statediagram-v2" {
        return adapt_mermaid_statediagram(source);
    }
    if lower == "erdiagram" {
        return adapt_mermaid_erdiagram(source);
    }

    Err(Diagnostic::error_code(
        "E_MERMAID_FAMILY_UNSUPPORTED",
        format!(
            "mermaid frontend does not support this diagram type: `{directive}`; \
             supported families are sequenceDiagram, flowchart, classDiagram, stateDiagram, erDiagram"
        ),
    )
    .with_span(directive_span))
}

// ---------------------------------------------------------------------------
// sequenceDiagram adapter (unchanged from original)
// ---------------------------------------------------------------------------

fn adapt_mermaid_sequence(source: &str) -> Result<String, Diagnostic> {
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

// ---------------------------------------------------------------------------
// flowchart / graph adapter → PlantUML component-style
// ---------------------------------------------------------------------------

/// Translate a Mermaid `flowchart TD` / `graph TD` block into a PlantUML
/// component-style diagram that the existing component renderer accepts.
///
/// Node shapes handled:
///   `A[Label]`   → `component "Label" as A`
///   `A{Label}`   → `component "Label" as A`   (decision – best-effort)
///   `A(Label)`   → `component "Label" as A`
///   `A`          → bare id kept as `component A`
///
/// Arrow forms:
///   `A --> B`           → `A --> B`
///   `A -->|cond| B`     → `A --> B : cond`
///   `A -- text --> B`   → `A --> B : text`
fn adapt_mermaid_flowchart(source: &str) -> Result<String, Diagnostic> {
    let mut out = Vec::new();
    out.push("@startuml".to_string());
    let mut first = true;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip the `flowchart TD` / `graph TD` directive line.
            continue;
        }

        // Try to parse as an arrow statement first.
        if let Some(converted) = adapt_flowchart_edge(line) {
            out.push(converted);
            continue;
        }

        // Node declaration: `ID[Label]`, `ID{Label}`, `ID(Label)`, bare `ID`.
        if let Some(converted) = adapt_flowchart_node(line) {
            out.push(converted);
            continue;
        }

        // Subgraph / end – map to `package`/`end`.
        if let Some(rest) = line.strip_prefix("subgraph ") {
            let label = rest.trim().trim_matches('"');
            out.push(format!("package \"{label}\" {{"));
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "end" || lower == "end subgraph" {
            out.push("}".to_string());
            continue;
        }

        // Unknown line — emit as comment so the parse still succeeds.
        out.push(format!("' [flowchart] {line}"));
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

/// Extract a node's canonical id and optional label from Mermaid node syntax.
/// Returns `(id, label)`.
fn parse_flowchart_node_id_label(token: &str) -> (String, Option<String>) {
    // Match `ID[Label]`, `ID{Label}`, `ID(Label)`, `ID((Label))`.
    for (open, close) in [("[", "]"), ("{", "}"), ("((", "))"), ("(", ")")] {
        if let Some(bracket_start) = token.find(open) {
            let id = token[..bracket_start].trim().to_string();
            if !id.is_empty() {
                let rest = &token[bracket_start + open.len()..];
                if let Some(end) = rest.rfind(close) {
                    let label = rest[..end].trim().to_string();
                    return (id, if label.is_empty() { None } else { Some(label) });
                }
            }
        }
    }
    // Bare id.
    (token.trim().to_string(), None)
}

fn adapt_flowchart_node(line: &str) -> Option<String> {
    // Must not contain `-->` or `---` to be a pure node declaration.
    if line.contains("-->") || line.contains("---") || line.contains("-.->") {
        return None;
    }
    let (id, label) = parse_flowchart_node_id_label(line);
    if id.is_empty() {
        return None;
    }
    if let Some(lbl) = label {
        Some(format!("component \"{lbl}\" as {id}"))
    } else {
        Some(format!("component {id}"))
    }
}

/// Parse a Mermaid flowchart edge: `A --> B`, `A -->|label| B`,
/// `A -- label --> B`, `A -.-> B`, etc.
fn adapt_flowchart_edge(line: &str) -> Option<String> {
    // Detect edge by presence of `-->`, `-.->`, `-->`
    let arrow_forms = ["-.->", "-->", "---"];
    let mut best: Option<(usize, &str)> = None;
    for arrow in &arrow_forms {
        if let Some(pos) = line.find(arrow) {
            if best.is_none_or(|(p, _)| pos < p) {
                best = Some((pos, arrow));
            }
        }
    }
    let (arrow_pos, arrow_str) = best?;

    let lhs_raw = line[..arrow_pos].trim();
    let rhs_raw = line[arrow_pos + arrow_str.len()..].trim();

    // Handle `A -->|label| B` form: label is embedded in arrow suffix `|label|`.
    let (rhs_token, edge_label) = if let Some(stripped) = rhs_raw.strip_prefix('|') {
        if let Some(end_pipe) = stripped.find('|') {
            let label = stripped[..end_pipe].trim().to_string();
            let rhs_after = stripped[end_pipe + 1..].trim();
            (rhs_after, Some(label))
        } else {
            (rhs_raw, None)
        }
    } else {
        (rhs_raw, None)
    };

    // Handle `A -- label --> B` form: label is embedded in lhs `A -- label`.
    let (lhs_token, lhs_label) = if lhs_raw.contains(" -- ") {
        let idx = lhs_raw.rfind(" -- ")?;
        let id = lhs_raw[..idx].trim();
        let label = lhs_raw[idx + 4..].trim();
        (id, Some(label.to_string()))
    } else {
        (lhs_raw, None)
    };

    let (from_id, _) = parse_flowchart_node_id_label(lhs_token);
    let (to_id, _) = parse_flowchart_node_id_label(rhs_token);

    if from_id.is_empty() || to_id.is_empty() {
        return None;
    }

    let label = edge_label.or(lhs_label);
    let plantuml_arrow = if arrow_str == "-.->)" { "..>" } else { "-->" };
    if let Some(lbl) = label {
        Some(format!("{from_id} {plantuml_arrow} {to_id} : {lbl}"))
    } else {
        Some(format!("{from_id} {plantuml_arrow} {to_id}"))
    }
}

// ---------------------------------------------------------------------------
// classDiagram adapter → PlantUML class diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `classDiagram` to PlantUML `@startuml` / `@enduml` with
/// `class Name { members }` blocks and `A <|-- B` relations.
///
/// Mermaid forms supported:
///   `Animal <|-- Dog`               → kept as-is (PlantUML-compatible)
///   `Animal : +String name`         → collected into class Animal { } block
///   `Animal : +eat()`               → collected into class Animal { } block
///   `class Dog { +bark() }`         → emit class Dog { +bark() }
///   `class Dog { \n+bark()\n}`      → multi-line form (each member on its own line)
fn adapt_mermaid_classdiagram(source: &str) -> Result<String, Diagnostic> {
    use std::collections::BTreeMap;

    // We do two passes: first gather all class members from `ClassName : member`
    // lines, then emit relations, inline classes, and finally gathered classes.
    let mut class_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut relations: Vec<String> = Vec::new();
    let mut inline_classes: Vec<String> = Vec::new();

    let mut first = true;
    let mut in_class_block: Option<(String, Vec<String>)> = None;
    let lines_iter = source.lines();

    for raw_line in lines_iter {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip `classDiagram` directive.
            continue;
        }

        // If we're inside a `class Foo {` block, accumulate members until `}`.
        if let Some((ref class_name, ref mut members)) = in_class_block {
            if line == "}" {
                let class_name = class_name.clone();
                let members = members.clone();
                inline_classes.push(format_class_block(&class_name, &members));
                in_class_block = None;
            } else {
                members.push(line.to_string());
            }
            continue;
        }

        // `ClassName : member` form – accumulate into class_members.
        if let Some(converted) = adapt_classdiagram_member_line(line) {
            let (cname, member) = converted;
            class_members.entry(cname).or_default().push(member);
            continue;
        }

        // Relation line: `A <|-- B`, `A --> B`, `A -- B`, etc.
        if let Some(rel) = adapt_classdiagram_relation(line) {
            relations.push(rel);
            continue;
        }

        // `class Foo {` — start of inline block.
        if let Some(rest) = line.strip_prefix("class ") {
            let rest = rest.trim();
            if rest.ends_with('{') {
                let class_name = rest.trim_end_matches('{').trim().to_string();
                in_class_block = Some((class_name, Vec::new()));
                continue;
            }
            // Bare `class Foo` declaration.
            let class_name = rest.trim().to_string();
            if !class_name.is_empty() {
                inline_classes.push(format!("class {class_name}"));
            }
            continue;
        }

        // Ignore `%%comment` and other unrecognised lines gracefully.
        inline_classes.push(format!("' [classDiagram] {line}"));
    }

    // If a block was never closed, flush it anyway.
    if let Some((class_name, members)) = in_class_block {
        inline_classes.push(format_class_block(&class_name, &members));
    }

    // Build output.
    let mut out = vec!["@startuml".to_string()];

    // Collect all class names that appear in relations so we can ensure they
    // have at least a bare `class X` declaration before the first relation.
    // This guarantees the parser detects `DiagramKind::Class` before it sees
    // the first relation line, which requires the kind to already be Class.
    let mut declared: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Emit gathered class blocks from `ClassName : member` lines first.
    for (class_name, members) in &class_members {
        declared.insert(class_name.clone());
        out.push(format_class_block(class_name, members));
    }

    // Emit inline class declarations / blocks.
    for item in &inline_classes {
        // Track names from `class Foo` or `class Foo { ... }` items.
        if let Some(rest) = item.strip_prefix("class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                declared.insert(name);
            }
        }
        out.push(item.clone());
    }

    // For any class referenced only in relations, emit a bare declaration first
    // so the family is established before we emit relation lines.
    for rel in &relations {
        // Extract lhs and rhs names separated by arrow tokens.
        for arrow in &[
            "<|--", "--|>", "*--", "--*", "o--", "--o", "-->", "<--", "--",
        ] {
            if let Some(idx) = rel.find(arrow) {
                let lhs = rel[..idx].trim().to_string();
                let rhs = rel[idx + arrow.len()..]
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !lhs.is_empty() && !declared.contains(&lhs) {
                    out.push(format!("class {lhs}"));
                    declared.insert(lhs);
                }
                if !rhs.is_empty() && !declared.contains(&rhs) {
                    out.push(format!("class {rhs}"));
                    declared.insert(rhs);
                }
                break;
            }
        }
    }

    // Emit relations after all class declarations.
    for rel in &relations {
        out.push(rel.clone());
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

fn format_class_block(name: &str, members: &[String]) -> String {
    if members.is_empty() {
        format!("class {name}")
    } else {
        let body = members.join("\n");
        format!("class {name} {{\n{body}\n}}")
    }
}

/// Parse a `ClassName : member` line.  Returns `(class_name, member)`.
fn adapt_classdiagram_member_line(line: &str) -> Option<(String, String)> {
    // Must not look like a relation (no `<`, `>`, `--`).
    if line.contains("--") || line.contains('<') || line.contains('>') {
        return None;
    }
    let (class_name, member) = line.split_once(':')?;
    let class_name = class_name.trim();
    let member = member.trim();
    if class_name.is_empty() || member.is_empty() {
        return None;
    }
    // Class name must not contain spaces (would indicate it's something else).
    if class_name.contains(' ') {
        return None;
    }
    Some((class_name.to_string(), member.to_string()))
}

/// Try to parse a Mermaid class relation line.
/// Mermaid relations that are already PlantUML-compatible are passed through.
fn adapt_classdiagram_relation(line: &str) -> Option<String> {
    // Must contain `--` to be a relation.
    if !line.contains("--") {
        return None;
    }
    // Mermaid relation forms:
    //   `A <|-- B`   inheritance  (PlantUML: `A <|-- B`)
    //   `A *-- B`    composition
    //   `A o-- B`    aggregation
    //   `A --> B`    association
    //   `A -- B`     link
    //   `A ..> B`    dependency
    //   `A ..|> B`   realization
    // Many of these are already valid PlantUML; we pass them through.
    // Strip optional label suffix `: label`.
    let (core, label) = if let Some((c, l)) = line.split_once(':') {
        // Make sure lhs contains `--` so we don't misparse member lines.
        if c.contains("--") {
            (c.trim(), Some(l.trim()))
        } else {
            (line, None)
        }
    } else {
        (line, None)
    };

    // Verify there's at least one `--` in core.
    if !core.contains("--") && !core.contains("..") {
        return None;
    }

    if let Some(lbl) = label {
        Some(format!("{core} : {lbl}"))
    } else {
        Some(core.to_string())
    }
}

// ---------------------------------------------------------------------------
// stateDiagram adapter → PlantUML state diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `stateDiagram`/`stateDiagram-v2` to PlantUML.
///
/// Supported forms:
///   `[*] --> Still`      → `[*] --> Still`
///   `Still --> Moving`   → `Still --> Moving`
///   `state "label" as X` → `state "label" as X`
///   `state X {`          → `state X {`
///   `}`                  → `}`
///   `note right of X ...` → emitted as comment (notes not yet supported in state renderer)
fn adapt_mermaid_statediagram(source: &str) -> Result<String, Diagnostic> {
    let mut out = vec!["@startuml".to_string()];
    let mut first = true;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip the `stateDiagram` / `stateDiagram-v2` directive.
            continue;
        }

        // Transition lines: `X --> Y` or `X --> Y : label` – pass through.
        if line.contains("-->") {
            out.push(line.to_string());
            continue;
        }

        // `state "label" as X` – pass through (PlantUML syntax).
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("state ") {
            out.push(line.to_string());
            continue;
        }

        // `[*]` bare pseudo-state declaration – pass through.
        if line == "[*]" {
            out.push(line.to_string());
            continue;
        }

        // `}` closing block.
        if line == "}" {
            out.push("}".to_string());
            continue;
        }

        // `note`, `--` dividers, etc. – emit as benign comment.
        out.push(format!("' [stateDiagram] {line}"));
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

// ---------------------------------------------------------------------------
// erDiagram adapter → PlantUML class-style diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `erDiagram` to a PlantUML class-style diagram.
///
/// Mermaid ER relation line:
///   `CUSTOMER ||--o{ ORDER : places`
///
/// We translate each entity name to a `class` declaration and each relation to
/// a PlantUML association arrow, carrying the cardinality string as an arrow
/// label for readability.
///
/// Cardinality glyph map (lossy but human-readable):
///   `||--o{`  →  `"1" --> "0..*"`
///   `||--|{`  →  `"1" --> "1..*"`
///   `}o--o{`  →  `"0..*" --> "0..*"`
///   etc.
///
/// Exact visual fidelity is not the goal; the output must parse cleanly.
fn adapt_mermaid_erdiagram(source: &str) -> Result<String, Diagnostic> {
    use std::collections::BTreeSet;

    let mut entities: BTreeSet<String> = BTreeSet::new();
    let mut relations: Vec<String> = Vec::new();
    let mut first = true;
    let mut in_entity_block: Option<String> = None;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip `erDiagram` directive.
            continue;
        }

        // If we're inside an entity attribute block.
        if in_entity_block.is_some() {
            if line == "}" {
                in_entity_block = None;
            }
            // Attribute lines are ignored for now – entity is already registered.
            continue;
        }

        // `ENTITY {` – start of attribute block.
        if line.ends_with('{') {
            let entity_name = line.trim_end_matches('{').trim().to_string();
            if !entity_name.is_empty() {
                entities.insert(entity_name.clone());
                in_entity_block = Some(entity_name);
            }
            continue;
        }

        // Relation line: `CUSTOMER ||--o{ ORDER : places`
        // Split on `:` to get core and label.
        if let Some((core, label)) = line.split_once(':') {
            if let Some(rel) = adapt_er_relation(core.trim(), label.trim(), &mut entities) {
                relations.push(rel);
                continue;
            }
        }

        // Bare entity name (no relation, no block).
        if !line.contains(' ') {
            entities.insert(line.to_string());
            continue;
        }

        // Unknown line – emit as comment.
        relations.push(format!("' [erDiagram] {line}"));
    }

    let mut out = vec!["@startuml".to_string()];

    // Emit entity class declarations.
    for entity in &entities {
        out.push(format!("class {entity}"));
    }

    // Emit relations.
    for rel in &relations {
        out.push(rel.clone());
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

/// Parse a Mermaid ER relation core like `CUSTOMER ||--o{ ORDER`.
/// Registers entity names as a side-effect.
fn adapt_er_relation(
    core: &str,
    label: &str,
    entities: &mut std::collections::BTreeSet<String>,
) -> Option<String> {
    // Mermaid ER cardinality tokens on the left and right of the `--`.
    // The double dash `--` separates the two sides.
    let dash_idx = core.find("--")?;
    // Split into: lhs_with_card `ENTITY ||` and rhs_with_card `o{ ENTITY`.
    let lhs_part = &core[..dash_idx]; // e.g. `CUSTOMER ||`
    let rhs_part = &core[dash_idx + 2..]; // e.g. `o{ ORDER`

    // lhs_part ends with the cardinality token; entity name is before the token.
    let (lhs_entity, lhs_card) = split_er_entity_and_card(lhs_part, true)?;
    let (rhs_entity, rhs_card) = split_er_entity_and_card(rhs_part, false)?;

    entities.insert(lhs_entity.clone());
    entities.insert(rhs_entity.clone());

    let card_str = format!("{lhs_card}--{rhs_card}");
    let rel_label = if label.is_empty() {
        card_str
    } else {
        format!("{card_str} {label}")
    };

    Some(format!("{lhs_entity} --> {rhs_entity} : {rel_label}"))
}

/// Split an ER half-line into `(entity_name, cardinality_string)`.
/// `is_lhs` controls whether the cardinality token is at the end (lhs) or start (rhs).
fn split_er_entity_and_card(part: &str, is_lhs: bool) -> Option<(String, String)> {
    let part = part.trim();
    // Cardinality tokens: `||`, `|{`, `|o`, `}{`, `}|`, `}o`, `o|`, `o{`, `o}`
    // These are 2-character tokens.
    let card_tokens = [
        "||", "|{", "|o", "}{", "}|", "}o", "o|", "o{", "o}", "{|", "{o",
    ];

    if is_lhs {
        // Entity is at the beginning; cardinality token at the end.
        for token in &card_tokens {
            if let Some(stripped) = part.strip_suffix(token) {
                let entity = stripped.trim();
                if !entity.is_empty() {
                    return Some((entity.to_string(), token.to_string()));
                }
            }
        }
        // No token found – treat the whole thing as the entity with empty card.
        if !part.is_empty() {
            return Some((part.to_string(), String::new()));
        }
    } else {
        // Entity is at the end; cardinality token at the start.
        for token in &card_tokens {
            if let Some(stripped) = part.strip_prefix(token) {
                let entity = stripped.trim();
                if !entity.is_empty() {
                    return Some((entity.to_string(), token.to_string()));
                }
            }
        }
        // No token found – treat the whole thing as the entity.
        if !part.is_empty() {
            return Some((part.to_string(), String::new()));
        }
    }
    None
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

fn classify_unsupported_mermaid_construct(line: &str) -> Option<&'static str> {
    let first = line.split_ascii_whitespace().next()?.to_ascii_lowercase();
    match first.as_str() {
        "alt" | "else" | "loop" | "opt" | "par" | "and" | "critical" | "option" | "break"
        | "rect" | "box" | "end" => Some("E_MERMAID_BLOCK_UNSUPPORTED"),
        "create" => Some("E_MERMAID_CREATE_UNSUPPORTED"),
        "link" | "links" => Some("E_MERMAID_LINK_UNSUPPORTED"),
        _ => None,
    }
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
