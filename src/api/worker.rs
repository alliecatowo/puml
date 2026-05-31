//! Browser / IDE worker protocol message types.
//!
//! This module defines the request/response envelope that browser Web Workers
//! and VS Code extension workers use to communicate with the PUML engine.
//! All types are serde-serialisable so they can be passed over `postMessage`
//! (browser) or JSON-RPC stdio (VS Code).
//!
//! # Protocol versioning
//!
//! The `version` field in [`WorkerRequest`] and [`WorkerResponse`] is a
//! monotonically-increasing integer. Currently `1`. Consumers must reject
//! responses whose version differs from the request version.
//!
//! # Stability guarantee
//!
//! Same as [`super::compile`]: fields are additive-only within a semver minor.

use serde::{Deserialize, Serialize};

use crate::api::compile::CompileResult;
use crate::api::types::ParsePipelineOptions;

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// A single message sent from a host (browser page or VS Code extension) to
/// the PUML worker.
///
/// Each variant corresponds to one operation. The `id` field is echoed back
/// verbatim in [`WorkerResponse::id`] so callers can correlate async replies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRequest {
    /// Caller-assigned correlation ID. Opaque string; echoed in the response.
    pub id: String,
    /// Protocol version. Must be `1`.
    pub version: u32,
    /// The operation to perform.
    pub payload: WorkerRequestPayload,
}

/// Enumeration of all worker operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum WorkerRequestPayload {
    /// Full compile: parse → normalize → render → diagnostics → tokens → symbols.
    Compile(CompileRequest),
    /// Render only: produce SVG pages from already-valid source. Faster than
    /// `Compile` when diagnostics and tokens are not needed.
    Render(RenderRequest),
    /// Hover information at a source position.
    Hover(HoverRequest),
    /// Completion items at a source position.
    Completion(CompletionRequest),
    /// Full diagnostics report without rendering.
    Diagnostics(DiagnosticsRequest),
    /// Semantic tokens for the entire document.
    SemanticTokens(SemanticTokensRequest),
}

/// Payload for the `compile` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileRequest {
    /// PlantUML (or Mermaid / PicoUML) source text.
    pub source: String,
    /// Optional frontend hint. `null` / omitted means auto-detect.
    #[serde(default)]
    pub frontend: Option<String>,
}

/// Payload for the `render` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderRequest {
    /// Diagram source text.
    pub source: String,
    /// Output format. `"svg"` is the only format currently supported over the
    /// worker protocol; other formats are CLI-only.
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "svg".to_string()
}

/// Payload for the `hover` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoverRequest {
    /// Diagram source text.
    pub source: String,
    /// One-based line number.
    pub line: u64,
    /// One-based column number (Unicode scalar column).
    pub column: u64,
}

/// Payload for the `completion` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Diagram source text.
    pub source: String,
    /// One-based line number of the cursor.
    pub line: u64,
    /// One-based column number of the cursor.
    pub column: u64,
}

/// Payload for the `diagnostics` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsRequest {
    /// Diagram source text.
    pub source: String,
}

/// Payload for the `semanticTokens` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTokensRequest {
    /// Diagram source text.
    pub source: String,
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// A single message sent from the PUML worker back to the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerResponse {
    /// Correlation ID from the originating [`WorkerRequest`].
    pub id: String,
    /// Protocol version. Always `1`.
    pub version: u32,
    /// `true` when the operation completed without infrastructure errors.
    /// Note: a successful compile that produces parse errors still has `ok:
    /// true`; inspect the payload diagnostics instead.
    pub ok: bool,
    /// Human-readable error message when `ok` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The operation result. `null` / absent when `ok` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<WorkerResponsePayload>,
}

/// Enumeration of worker response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum WorkerResponsePayload {
    /// Response to a [`CompileRequest`]: full typed compile result.
    Compile(CompileResult),
    /// Response to a [`RenderRequest`]: SVG pages.
    Render(RenderResponse),
    /// Response to a [`HoverRequest`].
    Hover(HoverResponse),
    /// Response to a [`CompletionRequest`].
    Completion(CompletionResponse),
    /// Response to a [`DiagnosticsRequest`].
    Diagnostics(DiagnosticsResponse),
    /// Response to a [`SemanticTokensRequest`].
    SemanticTokens(SemanticTokensResponse),
}

/// Payload for the `render` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderResponse {
    /// Rendered SVG pages (usually one; multiple when `newpage` is used).
    pub svg_pages: Vec<String>,
}

/// Payload for the `hover` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoverResponse {
    /// Markdown-formatted hover text. `null` when no hover is available.
    pub markdown: Option<String>,
}

/// Payload for the `completion` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Whether the list is incomplete (more items exist beyond this slice).
    pub is_incomplete: bool,
    /// Completion items.
    pub items: Vec<CompletionItemDto>,
}

/// Serialisable completion item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItemDto {
    /// Display label shown in the completion menu.
    pub label: String,
    /// Item kind: `"keyword"`, `"operator"`, or `"snippet"`.
    pub kind: String,
    /// Short detail / type annotation shown beside the label.
    pub detail: String,
    /// Long markdown documentation string.
    pub documentation: String,
}

/// Payload for the `diagnostics` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsResponse {
    /// Diagnostics (errors and warnings).
    pub diagnostics: Vec<DiagnosticItemDto>,
}

/// Serialisable diagnostic item (for the worker protocol).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticItemDto {
    /// Optional error code.
    pub code: Option<String>,
    /// `"error"` or `"warning"`.
    pub severity: String,
    /// Fine-grained category.
    pub category: String,
    /// Human-readable message.
    pub message: String,
    /// One-based start line, when available.
    pub start_line: Option<usize>,
    /// One-based start column, when available.
    pub start_column: Option<usize>,
    /// One-based end line, when available.
    pub end_line: Option<usize>,
    /// One-based end column, when available.
    pub end_column: Option<usize>,
}

/// Payload for the `semanticTokens` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTokensResponse {
    /// Flat list of semantic tokens.
    pub tokens: Vec<SemanticTokenItemDto>,
}

/// Serialisable semantic token (for the worker protocol).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTokenItemDto {
    /// Byte offset of token start.
    pub start: usize,
    /// Byte offset of token end (exclusive).
    pub end: usize,
    /// Token kind: `"keyword"` or `"operator"`.
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Constructor helpers
// ---------------------------------------------------------------------------

impl WorkerResponse {
    /// Create a successful response with the given payload.
    pub fn ok(id: impl Into<String>, payload: WorkerResponsePayload) -> Self {
        Self {
            id: id.into(),
            version: 1,
            ok: true,
            error: None,
            payload: Some(payload),
        }
    }

    /// Create an error response.
    pub fn err(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: 1,
            ok: false,
            error: Some(message.into()),
            payload: None,
        }
    }
}

impl WorkerRequest {
    /// Construct a well-formed compile request.
    pub fn compile(id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: 1,
            payload: WorkerRequestPayload::Compile(CompileRequest {
                source: source.into(),
                frontend: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch helper
// ---------------------------------------------------------------------------

/// Dispatch a [`WorkerRequest`] using the default pipeline options and return
/// a [`WorkerResponse`].
///
/// This is a convenience function for environments (browser WASM, VS Code
/// extension, test harness) that want a single call surface without managing
/// the pipeline options themselves.
pub fn dispatch(request: WorkerRequest) -> WorkerResponse {
    let id = request.id.clone();
    dispatch_with_options(request, &ParsePipelineOptions::default())
        .unwrap_or_else(|e| WorkerResponse::err(id, e.message))
}

/// Dispatch a [`WorkerRequest`] with explicit pipeline options.
pub fn dispatch_with_options(
    request: WorkerRequest,
    options: &ParsePipelineOptions,
) -> Result<WorkerResponse, crate::diagnostic::Diagnostic> {
    use crate::language_service::{
        completion_items, diagnostics_with_options as ls_diagnostics, hover,
        semantic_tokens as ls_semantic_tokens,
    };

    let id = request.id.clone();

    let payload = match request.payload {
        WorkerRequestPayload::Compile(req) => {
            let result = crate::api::compile::compile_with_options(&req.source, options)?;
            WorkerResponsePayload::Compile(result)
        }

        WorkerRequestPayload::Render(req) => {
            let svgs = crate::api::render::render_source_to_svgs(&req.source).unwrap_or_default();
            WorkerResponsePayload::Render(RenderResponse { svg_pages: svgs })
        }

        WorkerRequestPayload::Hover(req) => {
            let markdown = hover(&req.source, (req.line, req.column)).map(|h| h.markdown);
            WorkerResponsePayload::Hover(HoverResponse { markdown })
        }

        WorkerRequestPayload::Completion(_req) => {
            let list = completion_items();
            let items = list
                .items
                .into_iter()
                .map(|item| CompletionItemDto {
                    label: item.label.to_string(),
                    kind: match item.kind {
                        crate::language_service::CompletionItemKind::Keyword => {
                            "keyword".to_string()
                        }
                        crate::language_service::CompletionItemKind::Operator => {
                            "operator".to_string()
                        }
                        crate::language_service::CompletionItemKind::Snippet => {
                            "snippet".to_string()
                        }
                    },
                    detail: item.detail.to_string(),
                    documentation: item.documentation.to_string(),
                })
                .collect();
            WorkerResponsePayload::Completion(CompletionResponse {
                is_incomplete: list.is_incomplete,
                items,
            })
        }

        WorkerRequestPayload::Diagnostics(req) => {
            let report = ls_diagnostics(&req.source, options);
            let items = report
                .diagnostics
                .into_iter()
                .map(|ld| DiagnosticItemDto {
                    code: ld.code,
                    severity: match ld.severity {
                        crate::diagnostic::Severity::Error => "error".to_string(),
                        crate::diagnostic::Severity::Warning => "warning".to_string(),
                    },
                    category: ld.category.as_str().to_string(),
                    message: ld.message,
                    start_line: ld.range.map(|r| r.start.line),
                    start_column: ld.range.map(|r| r.start.column),
                    end_line: ld.range.map(|r| r.end.line),
                    end_column: ld.range.map(|r| r.end.column),
                })
                .collect();
            WorkerResponsePayload::Diagnostics(DiagnosticsResponse { diagnostics: items })
        }

        WorkerRequestPayload::SemanticTokens(req) => {
            let tokens = ls_semantic_tokens(&req.source)
                .into_iter()
                .map(|t| SemanticTokenItemDto {
                    start: t.span.start,
                    end: t.span.end,
                    kind: match t.kind {
                        crate::language_service::SemanticTokenKind::Keyword => {
                            "keyword".to_string()
                        }
                        crate::language_service::SemanticTokenKind::Operator => {
                            "operator".to_string()
                        }
                    },
                })
                .collect();
            WorkerResponsePayload::SemanticTokens(SemanticTokensResponse { tokens })
        }
    };

    Ok(WorkerResponse::ok(id, payload))
}
