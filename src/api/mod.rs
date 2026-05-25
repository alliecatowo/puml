mod lsp;
mod markdown;
mod pipeline;
mod render;
mod types;

pub use lsp::lsp_capabilities;
pub use markdown::extract_markdown_diagrams;
pub use pipeline::{
    detect_diagram_family, normalize, normalize_family, parse, parse_with_pipeline_options,
    parse_with_pipeline_result_options, preprocess_with_pipeline_options,
};
pub use render::{
    normalized_model_summary_to_json, normalized_scene_summary_to_json,
    render_family_document_artifact, render_family_document_svg, render_source_to_svg,
    render_source_to_svg_for_family, render_source_to_svgs, render_source_to_svgs_for_family,
    render_source_to_text, render_source_to_texts, render_svg_pages_from_model,
};
pub use types::{
    CompatMode, DeterminismMode, DiagramFamily, DiagramInput, FrontendSelection,
    ParsePipelineOptions, ParsePipelineResult,
};
