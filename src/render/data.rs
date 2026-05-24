use super::*;

mod model;
mod parse;
mod svg;

use model::*;
use parse::*;
use svg::*;

pub fn render_json_svg(document: &JsonDocument) -> String {
    let controls = parse_structured_controls(&document.raw, DataFamily::Json);
    let rows = json_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    render_structured_svg(
        document.title.as_deref(),
        DataFamily::Json,
        &rows,
        &controls,
    )
}

pub fn render_yaml_svg(document: &YamlDocument) -> String {
    let controls = parse_structured_controls(&document.raw, DataFamily::Yaml);
    let rows = yaml_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    render_structured_svg(
        document.title.as_deref(),
        DataFamily::Yaml,
        &rows,
        &controls,
    )
}
