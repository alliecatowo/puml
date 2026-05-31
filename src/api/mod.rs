pub mod compile;
mod lsp;
mod markdown;
pub(crate) mod pipeline;
mod render;
mod render_scene;
mod render_summary;
mod types;
pub mod worker;

pub use compile::{
    compile, compile_with_options, CompileResult, DiagnosticDto, DocumentSymbolDto,
    LanguageServiceSurface, ModelSummary, SemanticTokenDto, SpanDto,
};
pub use lsp::lsp_capabilities;
pub use markdown::extract_markdown_diagrams;
pub use pipeline::{
    detect_diagram_family, normalize, normalize_family, parse, parse_with_pipeline_options,
    parse_with_pipeline_result_options, preprocess_with_pipeline_options,
    preprocess_with_pipeline_result_options,
};
pub use render::{
    render_artifact_pages_from_model, render_family_document_artifact, render_family_document_svg,
    render_source_to_artifacts, render_source_to_artifacts_for_family, render_source_to_svg,
    render_source_to_svg_for_family, render_source_to_svgs, render_source_to_svgs_for_family,
    render_source_to_text, render_source_to_texts, render_svg_pages_from_model,
};
pub use render_scene::{
    normalized_artifact_scene_summary_to_json, normalized_scene_summary_to_json,
};
pub use render_summary::normalized_model_summary_to_json;
pub use types::{
    CompatMode, DiagramFamily, DiagramInput, FrontendSelection, ParsePipelineOptions,
    ParsePipelineResult, PreprocessPipelineResult,
};
pub use worker::{
    dispatch, dispatch_with_options, WorkerRequest, WorkerRequestPayload, WorkerResponse,
    WorkerResponsePayload,
};
