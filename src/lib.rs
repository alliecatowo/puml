mod api;
pub mod ast;
mod bootstrap_icons;
pub mod creole;
pub mod diagnostic;
pub mod formatter;
pub mod language_service;
// Frontend adapters translate non-default input surfaces into PlantUML-shaped
// source before the shared parser, normalizer, layout, and renderer run.
mod frontend;
pub mod layout;
mod material_icons;
pub mod metadata;
pub mod model;
pub mod normalize;
mod openiconic;
pub mod output;
pub mod parser;
mod preproc;
pub mod registry;
pub mod render;
pub mod render_core;
pub mod scene;
pub mod source;
pub mod specialized;
pub mod sprites;
pub mod stdlib;
mod text_markup;
pub mod text_truncate;
pub mod theme;

pub use api::{
    detect_diagram_family, extract_markdown_diagrams, lsp_capabilities, normalize,
    normalize_family, normalized_model_summary_to_json, normalized_scene_summary_to_json, parse,
    parse_with_pipeline_options, parse_with_pipeline_result_options,
    preprocess_with_pipeline_options, render_family_document_artifact, render_family_document_svg,
    render_source_to_svg, render_source_to_svg_for_family, render_source_to_svgs,
    render_source_to_svgs_for_family, render_source_to_text, render_source_to_texts,
    render_svg_pages_from_model, CompatMode, DeterminismMode, DiagramFamily, DiagramInput,
    FrontendSelection, ParsePipelineOptions, ParsePipelineResult,
};
pub use ast::Document;
pub use diagnostic::{Diagnostic, DiagnosticJson};
pub use metadata::{extract_metadata, DiagramMetadata};
pub use model::{
    FamilyDocument, FamilyGroup, LegendHAlign, LegendVAlign, NormalizedDocument, ScaleSpec,
    SequenceDocument, SequencePage, StateDocument, TimelineDocument,
};
pub use render::TextOutputMode;
pub use scene::{LayoutOptions, Scene};
