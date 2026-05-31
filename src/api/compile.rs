//! Stable typed compile API for cross-frontend reuse.
//!
//! [`compile`] is the single entry point for browser workers, the VS Code
//! extension live-preview pipeline, and the MCP server. It runs the full
//! parse → normalize → render pipeline and returns a fully-typed, serde-stable
//! [`CompileResult`] DTO.
//!
//! # JSON stability guarantee
//!
//! The public fields of [`CompileResult`] and its nested types are **additive
//! only**. No existing field will be renamed or removed in a semver-compatible
//! release. New optional fields may be added at any time; consumers must ignore
//! unknown keys.

use serde::{Deserialize, Serialize};

use crate::api::render::render_artifact_pages_from_model;
use crate::api::types::ParsePipelineOptions;
use crate::diagnostic::Diagnostic;
use crate::language_service::{
    diagnostics_with_options, document_symbols, semantic_tokens, DocumentSymbolKind,
    SemanticTokenKind,
};

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Top-level compile result returned by [`compile`] and [`compile_with_options`].
///
/// All fields are always present. Empty collections indicate that no
/// diagnostics / pages / tokens were produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompileResult {
    /// Whether the compile succeeded. `true` even if there are warnings; `false`
    /// only when at least one error-severity diagnostic was emitted.
    pub ok: bool,
    /// Detected diagram family (e.g. `"sequence"`, `"class"`).
    pub family: String,
    /// Rendered SVG pages. Usually one page; may be multiple for `newpage`-split
    /// diagrams. Empty when `ok` is `false`.
    pub svg_pages: Vec<String>,
    /// All diagnostics (errors and warnings), serialised in a stable JSON shape.
    pub diagnostics: Vec<DiagnosticDto>,
    /// High-level model summary (node/participant counts, title, etc.).
    pub model_summary: ModelSummary,
    /// Semantic tokens for the source document.
    pub semantic_tokens: Vec<SemanticTokenDto>,
    /// Document symbols (participants, messages, classes, …).
    pub symbols: Vec<DocumentSymbolDto>,
    /// Language-service surface: static capabilities JSON re-exported as a
    /// typed struct for convenience.
    pub language_service: LanguageServiceSurface,
}

/// Serialisable form of a [`crate::diagnostic::Diagnostic`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticDto {
    /// Optional error/warning code extracted from `[E_CODE]`-prefixed messages.
    pub code: Option<String>,
    /// `"error"` or `"warning"`.
    pub severity: String,
    /// Fine-grained category string (e.g. `"parse-error"`, `"unsupported-syntax"`).
    pub category: String,
    /// Human-readable message (without the leading `[CODE] ` prefix).
    pub message: String,
    /// Byte-offset span in the source, when available.
    pub span: Option<SpanDto>,
    /// One-based line number, when available.
    pub line: Option<usize>,
    /// One-based column number, when available.
    pub column: Option<usize>,
}

/// Byte-offset range within the source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanDto {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// High-level model summary: counts of structural elements in the diagram.
///
/// The exact set of populated fields depends on `family`; unset counts are
/// always `0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModelSummary {
    /// Diagram kind string matching `family`.
    pub kind: String,
    /// Number of warnings emitted during normalisation (subset of `diagnostics`).
    pub warning_count: usize,
    /// Participants / nodes / columns, depending on family.
    pub node_count: usize,
    /// Events / edges / relations, depending on family.
    pub edge_count: usize,
    /// Optional diagram title from `title …` directive.
    pub title: Option<String>,
}

/// Serialisable form of a [`SemanticToken`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTokenDto {
    /// Byte offset of token start.
    pub start: usize,
    /// Byte offset of token end (exclusive).
    pub end: usize,
    /// Token kind string: `"keyword"` or `"operator"`.
    pub kind: String,
}

/// Serialisable form of a [`DocumentSymbol`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSymbolDto {
    /// Symbol name (participant name, class name, etc.).
    pub name: String,
    /// Symbol kind: `"participant"`, `"message"`, or `"unknown"`.
    pub kind: String,
    /// Byte span of the whole symbol declaration.
    pub span: SpanDto,
}

/// Static language-service capability flags.
///
/// Mirrors the subset of the LSP `ServerCapabilities` structure that consumers
/// can inspect to decide which language-service operations to call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageServiceSurface {
    pub hover: bool,
    pub completion: bool,
    pub diagnostics: bool,
    pub semantic_tokens: bool,
    pub document_symbols: bool,
    pub formatting: bool,
    pub definition: bool,
    pub references: bool,
    pub rename: bool,
}

impl Default for LanguageServiceSurface {
    fn default() -> Self {
        Self {
            hover: true,
            completion: true,
            diagnostics: true,
            semantic_tokens: true,
            document_symbols: true,
            formatting: true,
            definition: true,
            references: true,
            rename: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile `source` with default options.
///
/// This is the primary entry point for cross-frontend diagram compilation. It
/// runs the full parse → normalize → render pipeline and returns a
/// fully-typed, JSON-serialisable [`CompileResult`].
///
/// # Errors
///
/// Returns `Err(Diagnostic)` only for unrecoverable infrastructure failures
/// (e.g. invalid UTF-8). Syntax and semantic errors are represented as items
/// inside [`CompileResult::diagnostics`] and the `ok` flag is set to `false`.
pub fn compile(source: &str) -> Result<CompileResult, Diagnostic> {
    compile_with_options(source, &ParsePipelineOptions::default())
}

/// Compile `source` with caller-supplied pipeline options.
///
/// See [`compile`] for the general contract.
pub fn compile_with_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<CompileResult, Diagnostic> {
    // 1. Diagnostics (parse + normalize warnings).
    let diagnostics_report = diagnostics_with_options(source, options);
    let has_errors = diagnostics_report
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::diagnostic::Severity::Error);

    let diag_dtos: Vec<DiagnosticDto> = diagnostics_report
        .diagnostics
        .iter()
        .map(|ld| DiagnosticDto {
            code: ld.code.clone(),
            severity: match ld.severity {
                crate::diagnostic::Severity::Error => "error".to_string(),
                crate::diagnostic::Severity::Warning => "warning".to_string(),
            },
            category: ld.category.as_str().to_string(),
            message: ld.message.clone(),
            span: ld.span.map(|s| SpanDto {
                start: s.start,
                end: s.end,
            }),
            line: ld.range.map(|r| r.start.line),
            column: ld.range.map(|r| r.start.column),
        })
        .collect();

    // 2. Semantic tokens.
    let token_dtos: Vec<SemanticTokenDto> = semantic_tokens(source)
        .into_iter()
        .map(|t| SemanticTokenDto {
            start: t.span.start,
            end: t.span.end,
            kind: match t.kind {
                SemanticTokenKind::Keyword => "keyword".to_string(),
                SemanticTokenKind::Operator => "operator".to_string(),
            },
        })
        .collect();

    // 3. Parse + normalize to extract symbols and model summary.
    //    We try a best-effort parse; on failure we return an empty model.
    let (svg_pages, model_summary, symbol_dtos, family_str) = match crate::parse(source) {
        Ok(document) => {
            // Document symbols from raw AST.
            let sym_dtos: Vec<DocumentSymbolDto> = document_symbols(&document)
                .into_iter()
                .map(|sym| DocumentSymbolDto {
                    name: sym.name.clone(),
                    kind: document_symbol_kind_str(sym.kind),
                    span: SpanDto {
                        start: sym.span.start,
                        end: sym.span.end,
                    },
                })
                .collect();

            let detected_family = crate::api::pipeline::map_ast_kind_to_family(document.kind);
            let family_str = detected_family.as_str().to_string();

            match crate::normalize::normalize_family(document) {
                Ok(model) => {
                    let summary = model_summary_from_normalized(&model);
                    let pages = if has_errors {
                        Vec::new()
                    } else {
                        render_artifact_pages_from_model(&model)
                            .into_iter()
                            .map(|a| a.svg)
                            .collect()
                    };
                    (pages, summary, sym_dtos, family_str)
                }
                Err(_) => (
                    Vec::new(),
                    ModelSummary {
                        kind: family_str.clone(),
                        ..ModelSummary::default()
                    },
                    sym_dtos,
                    family_str,
                ),
            }
        }
        Err(_) => (
            Vec::new(),
            ModelSummary::default(),
            Vec::new(),
            "unknown".to_string(),
        ),
    };

    Ok(CompileResult {
        ok: !has_errors,
        family: family_str,
        svg_pages,
        diagnostics: diag_dtos,
        model_summary,
        semantic_tokens: token_dtos,
        symbols: symbol_dtos,
        language_service: LanguageServiceSurface::default(),
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn document_symbol_kind_str(kind: DocumentSymbolKind) -> String {
    match kind {
        DocumentSymbolKind::Participant => "participant".to_string(),
        DocumentSymbolKind::Message => "message".to_string(),
    }
}

fn model_summary_from_normalized(model: &crate::model::NormalizedDocument) -> ModelSummary {
    use crate::model::NormalizedDocument;
    match model {
        NormalizedDocument::Sequence(seq) => ModelSummary {
            kind: "sequence".to_string(),
            warning_count: seq.warnings.len(),
            node_count: seq.participants.len(),
            edge_count: seq.events.len(),
            title: seq.title.clone(),
        },
        NormalizedDocument::Family(fam) => ModelSummary {
            kind: format!("{:?}", fam.kind).to_lowercase(),
            warning_count: fam.warnings.len(),
            node_count: fam.nodes.len(),
            edge_count: fam.relations.len(),
            title: fam.title.clone(),
        },
        NormalizedDocument::FamilyPages(pages) => {
            let first = pages.first();
            ModelSummary {
                kind: first
                    .map(|f| format!("{:?}", f.kind).to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string()),
                warning_count: pages.iter().map(|p| p.warnings.len()).sum(),
                node_count: pages.iter().map(|p| p.nodes.len()).sum(),
                edge_count: pages.iter().map(|p| p.relations.len()).sum(),
                title: first.and_then(|f| f.title.clone()),
            }
        }
        NormalizedDocument::Timeline(doc) => ModelSummary {
            kind: "timeline".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.tasks.len(),
            edge_count: doc.milestones.len(),
            title: doc.title.clone(),
        },
        NormalizedDocument::State(doc) => ModelSummary {
            kind: "state".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.nodes.len(),
            edge_count: doc.transitions.len(),
            title: doc.title.clone(),
        },
        NormalizedDocument::Json(doc) => ModelSummary {
            kind: "json".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Yaml(doc) => ModelSummary {
            kind: "yaml".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Nwdiag(doc) => ModelSummary {
            kind: "nwdiag".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Archimate(doc) => ModelSummary {
            kind: "archimate".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Regex(doc) => ModelSummary {
            kind: "regex".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Ebnf(doc) => ModelSummary {
            kind: "ebnf".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Math(doc) => ModelSummary {
            kind: "math".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Sdl(doc) => ModelSummary {
            kind: "sdl".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Ditaa(doc) => ModelSummary {
            kind: "ditaa".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Chart(doc) => ModelSummary {
            kind: "chart".to_string(),
            warning_count: doc.warnings.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Stdlib(doc) => ModelSummary {
            kind: "stdlib".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.entries.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Chen(doc) => ModelSummary {
            kind: "chen".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.nodes.len(),
            edge_count: doc.relations.len(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Board(doc) => ModelSummary {
            kind: "board".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.columns.len(),
            title: doc.title.clone(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Files(doc) => ModelSummary {
            kind: "files".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.roots.len(),
            title: doc.title.clone(),
            ..ModelSummary::default()
        },
        NormalizedDocument::Wire(doc) => ModelSummary {
            kind: "wire".to_string(),
            warning_count: doc.warnings.len(),
            node_count: doc.components.len(),
            edge_count: doc.links.len(),
            title: doc.title.clone(),
        },
    }
}
