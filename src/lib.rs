pub mod ast;
pub mod diagnostic;
pub mod model;
pub mod normalize;
pub mod parser;
pub mod source;
pub mod theme;

pub use ast::Document;
pub use diagnostic::Diagnostic;
pub use model::SequenceDocument;

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parser::parse(source)
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize::normalize(document)
}

pub fn render_source_to_svg(source: &str) -> Result<String, Diagnostic> {
    let document = parse(source)?;
    let sequence = normalize(document)?;

    let width = 200 + (sequence.participants.len() as i32 * 140);
    let height = 120 + (sequence.events.len() as i32 * 40);
    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<text x=\"16\" y=\"28\" font-family=\"monospace\" font-size=\"14\">puml sequence scaffold</text>");
    svg.push_str("</svg>");
    Ok(svg)
}
