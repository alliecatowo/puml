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
pub use model::{SequenceDocument, SequencePage};
pub use scene::{LayoutOptions, Scene};

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parser::parse(source)
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize::normalize(document)
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
    let sequence = normalize(document)?;
    let scenes = layout::layout_pages(&sequence, LayoutOptions::default());
    Ok(scenes.iter().map(render::render_svg).collect())
}
