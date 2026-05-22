use crate::ast::{Document, ParticipantDecl, StatementKind};
use crate::diagnostic::{Diagnostic, Severity};
use crate::formatter;
use crate::source::Span;
use crate::{
    normalize_family, parse_with_pipeline_options, NormalizedDocument, ParsePipelineOptions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: &'static str,
    pub kind: CompletionItemKind,
    pub detail: &'static str,
    pub documentation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionItemKind {
    Keyword,
    Operator,
    Snippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionList {
    pub is_incomplete: bool,
    pub items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: DocumentSymbolKind,
    pub span: Span,
    pub selection_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentSymbolKind {
    Participant,
    Message,
}

pub fn completion_items() -> CompletionList {
    CompletionList {
        is_incomplete: false,
        items: completion_specs().to_vec(),
    }
}

pub fn resolve_completion_item(label: &str) -> Option<CompletionItem> {
    completion_specs()
        .iter()
        .find(|entry| entry.label == label)
        .cloned()
}

pub fn hover(source: &str, position: (u64, u64)) -> Option<Hover> {
    if let Some(symbol) = symbol_at_pos(source, position) {
        if let Some(spec) = resolve_completion_item(symbol) {
            return Some(hover_for_completion(&spec));
        }
    }
    let (start, end) = word_range_at_pos(source, position)?;
    let word = &source[start..end];
    if let Some(spec) = resolve_completion_item(word) {
        return Some(hover_for_completion(&spec));
    }
    Some(Hover {
        markdown: format!("`{word}`"),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsReport {
    pub diagnostics: Vec<LanguageDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDiagnostic {
    pub code: Option<String>,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub range: Option<SourceRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// One-based line number in the source document.
    pub line: usize,
    /// One-based Unicode scalar column in the source line.
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub span: Span,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatDocumentResult {
    pub edits: Vec<TextEdit>,
    pub formatted: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Keyword,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    pub span: Span,
    pub kind: SemanticTokenKind,
}

pub fn document_symbols(document: &Document) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for statement in &document.statements {
        match &statement.kind {
            StatementKind::Participant(ParticipantDecl { name, .. }) => {
                symbols.push(DocumentSymbol {
                    name: name.clone(),
                    kind: DocumentSymbolKind::Participant,
                    span: statement.span,
                    selection_span: statement.span,
                });
            }
            StatementKind::Message(message) => {
                symbols.push(DocumentSymbol {
                    name: format!("{} {} {}", message.from, message.arrow, message.to),
                    kind: DocumentSymbolKind::Message,
                    span: statement.span,
                    selection_span: statement.span,
                });
            }
            _ => {}
        }
    }
    symbols
}

fn hover_for_completion(spec: &CompletionItem) -> Hover {
    Hover {
        markdown: format!("`{}`\n\n{}", spec.label, spec.documentation),
    }
}

fn completion_specs() -> &'static [CompletionItem] {
    use CompletionItemKind::{Keyword, Operator, Snippet};
    &[
        CompletionItem {
            label: "@startuml",
            kind: Keyword,
            detail: "Directive",
            documentation: "Start a sequence diagram block.",
        },
        CompletionItem {
            label: "@enduml",
            kind: Keyword,
            detail: "Directive",
            documentation: "End a sequence diagram block.",
        },
        CompletionItem {
            label: "title",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram title.",
        },
        CompletionItem {
            label: "header",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram header.",
        },
        CompletionItem {
            label: "footer",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram footer.",
        },
        CompletionItem {
            label: "caption",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram caption.",
        },
        CompletionItem {
            label: "legend",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Start a legend block.",
        },
        CompletionItem {
            label: "participant",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a participant.",
        },
        CompletionItem {
            label: "actor",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare an actor participant.",
        },
        CompletionItem {
            label: "boundary",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a boundary participant.",
        },
        CompletionItem {
            label: "control",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a control participant.",
        },
        CompletionItem {
            label: "entity",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare an entity participant.",
        },
        CompletionItem {
            label: "database",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a database participant.",
        },
        CompletionItem {
            label: "collections",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a collections participant.",
        },
        CompletionItem {
            label: "queue",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a queue participant.",
        },
        CompletionItem {
            label: "box",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a participant box group.",
        },
        CompletionItem {
            label: "end box",
            kind: Keyword,
            detail: "Group",
            documentation: "End a participant box group.",
        },
        CompletionItem {
            label: "note left of",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note to the left side of a target.",
        },
        CompletionItem {
            label: "note right of",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note to the right side of a target.",
        },
        CompletionItem {
            label: "note over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note over one or more targets.",
        },
        CompletionItem {
            label: "note across",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note across all participants.",
        },
        CompletionItem {
            label: "hnote over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a hex note over one or more targets.",
        },
        CompletionItem {
            label: "rnote over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a rectangle note over one or more targets.",
        },
        CompletionItem {
            label: "ref over",
            kind: Keyword,
            detail: "Reference",
            documentation: "Declare a reference block over participants.",
        },
        CompletionItem {
            label: "alt",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an alt block.",
        },
        CompletionItem {
            label: "else",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an alternate branch within alt/par.",
        },
        CompletionItem {
            label: "opt",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an opt block.",
        },
        CompletionItem {
            label: "loop",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a loop block.",
        },
        CompletionItem {
            label: "par",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a parallel block.",
        },
        CompletionItem {
            label: "break",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a break block.",
        },
        CompletionItem {
            label: "critical",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a critical block.",
        },
        CompletionItem {
            label: "group",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a generic group block.",
        },
        CompletionItem {
            label: "end",
            kind: Keyword,
            detail: "Group",
            documentation: "End the current group or note block.",
        },
        CompletionItem {
            label: "activate",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Activate a participant lifeline.",
        },
        CompletionItem {
            label: "deactivate",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Deactivate a participant lifeline.",
        },
        CompletionItem {
            label: "create",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Create a participant instance.",
        },
        CompletionItem {
            label: "destroy",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Destroy a participant instance.",
        },
        CompletionItem {
            label: "return",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Emit a return message.",
        },
        CompletionItem {
            label: "autoactivate on",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Enable auto-activation on messages.",
        },
        CompletionItem {
            label: "autoactivate off",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Disable auto-activation on messages.",
        },
        CompletionItem {
            label: "autonumber",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Enable automatic message numbering.",
        },
        CompletionItem {
            label: "autonumber stop",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Stop automatic message numbering.",
        },
        CompletionItem {
            label: "autonumber resume",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Resume automatic message numbering.",
        },
        CompletionItem {
            label: "hide footbox",
            kind: Keyword,
            detail: "Style",
            documentation: "Hide participant footboxes.",
        },
        CompletionItem {
            label: "show footbox",
            kind: Keyword,
            detail: "Style",
            documentation: "Show participant footboxes.",
        },
        CompletionItem {
            label: "skinparam sequence {}",
            kind: Snippet,
            detail: "Style",
            documentation: "Insert a sequence skinparam block.",
        },
        CompletionItem {
            label: "!include",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Include another source file.",
        },
        CompletionItem {
            label: "!define",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Define a preprocessor macro.",
        },
        CompletionItem {
            label: "!undef",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Undefine a preprocessor macro.",
        },
        CompletionItem {
            label: "newpage",
            kind: Keyword,
            detail: "Pagination",
            documentation: "Split output into a new page.",
        },
        CompletionItem {
            label: "class",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare a class node.",
        },
        CompletionItem {
            label: "interface",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an interface node.",
        },
        CompletionItem {
            label: "enum",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an enum node.",
        },
        CompletionItem {
            label: "abstract class",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an abstract class node.",
        },
        CompletionItem {
            label: "package",
            kind: Keyword,
            detail: "Family Diagram",
            documentation: "Group family diagram nodes under a package.",
        },
        CompletionItem {
            label: "namespace",
            kind: Keyword,
            detail: "Family Diagram",
            documentation: "Group family diagram nodes under a namespace.",
        },
        CompletionItem {
            label: "state",
            kind: Keyword,
            detail: "State Diagram",
            documentation: "Declare a state node.",
        },
        CompletionItem {
            label: "[*]",
            kind: Keyword,
            detail: "State Diagram",
            documentation: "State diagram start or end marker.",
        },
        CompletionItem {
            label: "start",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Start an activity diagram flow.",
        },
        CompletionItem {
            label: "stop",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Stop an activity diagram flow.",
        },
        CompletionItem {
            label: "if",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Start an activity branch.",
        },
        CompletionItem {
            label: "then",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Mark the positive activity branch.",
        },
        CompletionItem {
            label: "endif",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "End an activity branch.",
        },
        CompletionItem {
            label: "== divider ==",
            kind: Snippet,
            detail: "Structure",
            documentation: "Insert a divider row.",
        },
        CompletionItem {
            label: "... delay ...",
            kind: Snippet,
            detail: "Structure",
            documentation: "Insert a delay row.",
        },
        CompletionItem {
            label: "|||",
            kind: Keyword,
            detail: "Structure",
            documentation: "Insert a spacer row.",
        },
        CompletionItem {
            label: "->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Solid message arrow.",
        },
        CompletionItem {
            label: "-->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Dashed message arrow.",
        },
        CompletionItem {
            label: "<-",
            kind: Operator,
            detail: "Arrow",
            documentation: "Solid reverse message arrow.",
        },
        CompletionItem {
            label: "<--",
            kind: Operator,
            detail: "Arrow",
            documentation: "Dashed reverse message arrow.",
        },
        CompletionItem {
            label: "->>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head forward arrow.",
        },
        CompletionItem {
            label: "-->>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head dashed forward arrow.",
        },
        CompletionItem {
            label: "<<-",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head reverse arrow.",
        },
        CompletionItem {
            label: "<<--",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head dashed reverse arrow.",
        },
        CompletionItem {
            label: "->x",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow to lost endpoint.",
        },
        CompletionItem {
            label: "x->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow from found endpoint.",
        },
        CompletionItem {
            label: "-x",
            kind: Operator,
            detail: "Arrow",
            documentation: "Endpoint loss marker in expanded forms.",
        },
        CompletionItem {
            label: "->o",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow to open endpoint.",
        },
        CompletionItem {
            label: "o->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow from open endpoint.",
        },
        CompletionItem {
            label: "<->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Bidirectional solid arrow.",
        },
        CompletionItem {
            label: "<-->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Bidirectional dashed arrow.",
        },
        CompletionItem {
            label: "-[#color]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow with custom color.",
        },
        CompletionItem {
            label: "-[#color,dashed]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward dashed arrow with custom color.",
        },
        CompletionItem {
            label: "-[#color,bold]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward bold arrow with custom color.",
        },
        CompletionItem {
            label: "++",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Activate target lifeline after this message.",
        },
        CompletionItem {
            label: "--",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Deactivate source lifeline after this message.",
        },
        CompletionItem {
            label: "**",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Create target participant from this message.",
        },
        CompletionItem {
            label: "!!",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Destroy target participant from this message.",
        },
    ]
}

fn symbol_at_pos(src: &str, posn: (u64, u64)) -> Option<&'static str> {
    let off = lc_to_offset(src, posn.0 as usize, posn.1 as usize);
    if off >= src.len() {
        return None;
    }
    const SYMBOLS: &[&str] = &[
        "-[#color,dashed]>",
        "-[#color,bold]>",
        "-[#color]>",
        "-->>",
        "<<--",
        "<-->",
        "->>",
        "<<-",
        "-->",
        "<--",
        "<->",
        "->x",
        "x->",
        "->o",
        "o->",
        "->",
        "<-",
        "-x",
        "++",
        "--",
        "**",
        "!!",
    ];
    for symbol in SYMBOLS {
        for (start, _) in src.match_indices(symbol) {
            let end = start + symbol.len();
            if off >= start && off < end {
                return Some(symbol);
            }
        }
    }
    None
}

fn word_range_at_pos(src: &str, posn: (u64, u64)) -> Option<(usize, usize)> {
    let off = lc_to_offset(src, posn.0 as usize, posn.1 as usize);
    if off >= src.len() {
        return None;
    }
    let b = src.as_bytes();
    if !is_ident(b[off] as char) {
        return None;
    }
    let mut s = off;
    while s > 0 && is_ident(b[s - 1] as char) {
        s -= 1;
    }
    let mut e = off;
    while e < b.len() && is_ident(b[e] as char) {
        e += 1;
    }
    Some((s, e))
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if l == line && c == ch {
            return i;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += 1;
        }
    }
    src.len()
}

pub fn diagnostics(source: &str) -> DiagnosticsReport {
    diagnostics_with_options(source, &ParsePipelineOptions::default())
}

pub fn diagnostics_with_options(source: &str, options: &ParsePipelineOptions) -> DiagnosticsReport {
    let diagnostics = match parse_with_pipeline_options(source, options).and_then(normalize_family)
    {
        Ok(model) => normalized_warnings(&model)
            .iter()
            .map(|diagnostic| language_diagnostic(source, diagnostic))
            .collect(),
        Err(diagnostic) => vec![language_diagnostic(source, &diagnostic)],
    };

    DiagnosticsReport { diagnostics }
}

fn language_diagnostic(source: &str, diagnostic: &Diagnostic) -> LanguageDiagnostic {
    LanguageDiagnostic {
        code: diagnostic_code(&diagnostic.message),
        severity: diagnostic.severity,
        message: diagnostic.message.clone(),
        span: diagnostic.span,
        range: diagnostic.span.map(|span| SourceRange {
            start: source_position(source, span.start),
            end: source_position(source, span.end.max(span.start + 1)),
        }),
    }
}

fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
        NormalizedDocument::FamilyPages(pages) => pages
            .iter()
            .find_map(|page| (!page.warnings.is_empty()).then_some(page.warnings.as_slice()))
            .unwrap_or(&[]),
        NormalizedDocument::Timeline(timeline) => &timeline.warnings,
        NormalizedDocument::State(state) => &state.warnings,
        NormalizedDocument::Json(doc) => &doc.warnings,
        NormalizedDocument::Yaml(doc) => &doc.warnings,
        NormalizedDocument::Nwdiag(doc) => &doc.warnings,
        NormalizedDocument::Archimate(doc) => &doc.warnings,
        NormalizedDocument::Regex(doc) => &doc.warnings,
        NormalizedDocument::Ebnf(doc) => &doc.warnings,
        NormalizedDocument::Math(doc) => &doc.warnings,
        NormalizedDocument::Sdl(doc) => &doc.warnings,
        NormalizedDocument::Ditaa(doc) => &doc.warnings,
        NormalizedDocument::Chart(doc) => &doc.warnings,
    }
}

fn diagnostic_code(message: &str) -> Option<String> {
    let rest = message.strip_prefix('[')?;
    let (code, _tail) = rest.split_once("] ")?;
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}

fn source_position(source: &str, offset: usize) -> SourcePosition {
    let off = offset.min(source.len());
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (idx, ch) in source.char_indices() {
        if idx >= off {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    SourcePosition {
        line,
        column: source[line_start..off].chars().count() + 1,
    }
}

pub fn format_document(source: &str) -> FormatDocumentResult {
    let formatted = formatter::format_source(source);
    let edits = if formatted.changed {
        vec![TextEdit {
            span: Span {
                start: 0,
                end: source.len(),
            },
            new_text: formatted.formatted.clone(),
        }]
    } else {
        Vec::new()
    };
    FormatDocumentResult {
        edits,
        formatted: formatted.formatted,
        changed: formatted.changed,
    }
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let mut hits = Vec::<SemanticToken>::new();
    for (text, kind) in [
        ("participant", SemanticTokenKind::Keyword),
        ("actor", SemanticTokenKind::Keyword),
        ("note", SemanticTokenKind::Keyword),
        ("alt", SemanticTokenKind::Keyword),
        ("else", SemanticTokenKind::Keyword),
        ("end", SemanticTokenKind::Keyword),
        ("activate", SemanticTokenKind::Keyword),
        ("deactivate", SemanticTokenKind::Keyword),
        ("create", SemanticTokenKind::Keyword),
        ("destroy", SemanticTokenKind::Keyword),
        ("return", SemanticTokenKind::Keyword),
        ("autonumber", SemanticTokenKind::Keyword),
        ("-->", SemanticTokenKind::Operator),
        ("<--", SemanticTokenKind::Operator),
        ("->", SemanticTokenKind::Operator),
    ] {
        for span in find_token_spans(source, text) {
            hits.push(SemanticToken { span, kind });
        }
    }
    hits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then_with(|| (b.span.end - b.span.start).cmp(&(a.span.end - a.span.start)))
    });

    let mut filtered = Vec::<SemanticToken>::new();
    let mut last_end = 0usize;
    for hit in hits {
        if filtered.is_empty() || hit.span.start >= last_end {
            last_end = hit.span.end;
            filtered.push(hit);
        }
    }
    filtered
}

fn find_token_spans(source: &str, token: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    if token.is_empty() {
        return spans;
    }
    let bytes = source.as_bytes();
    let token_bytes = token.as_bytes();
    let mut start = 0;
    while start + token_bytes.len() <= bytes.len() {
        if &bytes[start..start + token_bytes.len()] == token_bytes {
            let left = start == 0 || !is_ident(bytes[start - 1] as char);
            let end = start + token_bytes.len();
            let right = end == bytes.len() || !is_ident(bytes[end] as char);
            if left && right {
                spans.push(Span { start, end });
            }
            start += token_bytes.len();
        } else {
            start += 1;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tokens_prefer_longest_operator_match() {
        let source = "@startuml\nAlice --> Bob\n@enduml\n";

        let tokens = semantic_tokens(source);

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, SemanticTokenKind::Operator);
        assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "-->");
    }

    #[test]
    fn semantic_tokens_keep_stable_order_and_keyword_boundaries() {
        let source =
            "@startuml\nparticipant Alice\nAlice -> Bob\nparticipantAlias -> Bob\n@enduml\n";

        let tokens = semantic_tokens(source);
        let rendered = tokens
            .iter()
            .map(|token| (&source[token.span.start..token.span.end], token.kind))
            .collect::<Vec<_>>();

        assert_eq!(
            rendered,
            vec![
                ("participant", SemanticTokenKind::Keyword),
                ("->", SemanticTokenKind::Operator),
                ("->", SemanticTokenKind::Operator),
            ]
        );
    }

    #[test]
    fn format_document_returns_transport_neutral_full_document_edit() {
        let source = "@startuml\n  alt ok  \nAlice -> Bob\nend\n@enduml\n";

        let result = format_document(source);

        assert!(result.changed);
        assert_eq!(result.edits.len(), 1);
        assert_eq!(
            result.edits[0].span,
            Span {
                start: 0,
                end: source.len()
            }
        );
        assert_eq!(
            result.formatted,
            "@startuml\nalt ok\n  Alice -> Bob\nend\n@enduml\n"
        );
        assert_eq!(result.edits[0].new_text, result.formatted);
    }

    #[test]
    fn hover_returns_completion_docs_for_symbol_and_word() {
        let source = "@startuml\nAlice --> Bob\nparticipant User\n@enduml\n";

        let symbol_hover = hover(source, (1, 7)).expect("symbol hover should resolve");
        assert!(symbol_hover.markdown.contains("`-->`"));
        assert!(symbol_hover.markdown.contains("Dashed message arrow."));

        let keyword_hover = hover(source, (2, 2)).expect("keyword hover should resolve");
        assert!(keyword_hover.markdown.contains("`participant`"));
        assert!(keyword_hover.markdown.contains("Declare a participant."));
    }

    #[test]
    fn hover_falls_back_to_word_literal_for_unknown_identifier() {
        let source = "@startuml\nfoobar\n@enduml\n";
        let h = hover(source, (1, 1)).expect("hover should produce fallback");
        assert_eq!(h.markdown, "`foobar`");
    }

    #[test]
    fn diagnostics_extracts_code_and_range() {
        let source = "@startuml\nfoo bar\n@enduml\n";
        let report = diagnostics(source);
        assert!(!report.diagnostics.is_empty());
        assert_eq!(
            report.diagnostics[0].code.as_deref(),
            Some("E_FAMILY_UNKNOWN")
        );
        assert!(report.diagnostics[0].range.is_none());
    }

    #[test]
    fn format_document_has_no_edit_when_already_formatted() {
        let source = "@startuml\nAlice -> Bob\n@enduml\n";
        let result = format_document(source);
        assert!(!result.changed);
        assert!(result.edits.is_empty());
        assert_eq!(result.formatted, source);
    }
}
