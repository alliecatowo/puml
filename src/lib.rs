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
pub use diagnostic::Diagnostic;
pub use model::SequenceDocument;
pub use scene::{LayoutOptions, Scene};

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parser::parse(source)
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize::normalize(document)
}

pub fn render_source_to_svg(source: &str) -> Result<String, Diagnostic> {
    let document = parse(source)?;
    let sequence = normalize(document)?;
    let scene = layout::layout(&sequence, LayoutOptions::default());
    Ok(render::render_svg(&scene))
}
