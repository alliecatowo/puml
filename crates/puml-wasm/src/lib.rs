//! In-browser puml renderer. Exposes `render_svg` / `render_svgs` to JS via
//! wasm-bindgen so the studio editor can render diagrams without a backend.

use puml::ast::DiagramKind;
use puml::{
    layout, normalize_family, parse_with_pipeline_options, render, Diagnostic, FamilyDocument,
    FrontendSelection, NormalizedDocument, ParsePipelineOptions,
};
use wasm_bindgen::prelude::*;

/// Render a single-page puml diagram to SVG. Multi-page diagrams return an
/// error; callers wanting multi-page should use [`render_svgs`].
#[wasm_bindgen]
pub fn render_svg(source: &str) -> Result<String, JsValue> {
    let pages =
        render_svgs_for_frontend(source, FrontendSelection::Auto).map_err(diagnostic_to_js)?;
    if pages.len() > 1 {
        return Err(diagnostic_to_js(Diagnostic::error(
            "multiple pages detected; use render_svgs or render_svgs_json",
        )));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

/// Render every page of a multi-page puml diagram to its own SVG string.
#[wasm_bindgen]
pub fn render_svgs(source: &str) -> Result<Box<[JsValue]>, JsValue> {
    render_svgs_for_frontend(source, FrontendSelection::Auto)
        .map(|pages| {
            pages
                .into_iter()
                .map(JsValue::from)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .map_err(diagnostic_to_js)
}

/// JSON-encoded render result: `{ ok: ["svg", ...] }` on success or
/// `{ error: <DiagnosticJson> }` on failure. Convenient for JS callers that
/// want a single round-trip without try/catch on every call.
#[wasm_bindgen]
pub fn render_svgs_json(source: &str) -> String {
    match render_svgs_for_frontend(source, FrontendSelection::Auto) {
        Ok(pages) => serde_json::json!({ "ok": pages }).to_string(),
        Err(diag) => serde_json::json!({
            "error": diag.to_json_with_source(source),
        })
        .to_string(),
    }
}

/// JSON-encoded render result using an explicit frontend/dialect hint.
///
/// `frontend` accepts `auto`, `puml`, `plantuml`, `picouml`, or `mermaid`
/// plus the Markdown fence aliases used by the site.
#[wasm_bindgen]
pub fn render_svgs_json_with_frontend(source: &str, frontend: &str) -> String {
    let frontend = match frontend_selection_from_hint(frontend) {
        Ok(frontend) => frontend,
        Err(diag) => {
            return serde_json::json!({
                "error": diag.to_json_with_source(source),
            })
            .to_string();
        }
    };
    match render_svgs_for_frontend(source, frontend) {
        Ok(pages) => serde_json::json!({ "ok": pages }).to_string(),
        Err(diag) => serde_json::json!({
            "error": diag.to_json_with_source(source),
        })
        .to_string(),
    }
}

/// Detect the diagram family of the source (sequence, class, state, ...).
/// Returns the family identifier from [`puml::DiagramFamily::as_str`], or an
/// error if the source fails to parse.
#[wasm_bindgen]
pub fn detect_family(source: &str) -> Result<String, JsValue> {
    detect_family_for_frontend(source, FrontendSelection::Auto)
        .map(|f| f.as_str().to_string())
        .map_err(diagnostic_to_js)
}

/// Detect the diagram family using an explicit frontend/dialect hint.
#[wasm_bindgen]
pub fn detect_family_with_frontend(source: &str, frontend: &str) -> Result<String, JsValue> {
    let frontend = frontend_selection_from_hint(frontend).map_err(diagnostic_to_js)?;
    detect_family_for_frontend(source, frontend)
        .map(|f| f.as_str().to_string())
        .map_err(diagnostic_to_js)
}

fn render_svgs_for_frontend(
    source: &str,
    frontend: FrontendSelection,
) -> Result<Vec<String>, Diagnostic> {
    if matches!(
        frontend,
        FrontendSelection::Auto | FrontendSelection::Plantuml
    ) {
        if let Some(result) = puml::specialized::try_render_specialized(source) {
            return result.map(|svg| vec![svg]);
        }
    }

    let document = parse_with_pipeline_options(source, &wasm_parse_options(frontend))?;
    let model = normalize_family(document)?;
    Ok(render_svg_pages_from_model(&model))
}

fn detect_family_for_frontend(
    source: &str,
    frontend: FrontendSelection,
) -> Result<puml::DiagramFamily, Diagnostic> {
    let document = parse_with_pipeline_options(source, &wasm_parse_options(frontend))?;
    Ok(match document.kind {
        DiagramKind::Sequence => puml::DiagramFamily::Sequence,
        DiagramKind::Class => puml::DiagramFamily::Class,
        DiagramKind::State => puml::DiagramFamily::State,
        DiagramKind::Activity => puml::DiagramFamily::Activity,
        DiagramKind::Timing => puml::DiagramFamily::Timing,
        DiagramKind::Component => puml::DiagramFamily::Component,
        DiagramKind::Deployment => puml::DiagramFamily::Deployment,
        DiagramKind::UseCase => puml::DiagramFamily::UseCase,
        DiagramKind::Object => puml::DiagramFamily::Object,
        DiagramKind::Salt => puml::DiagramFamily::Salt,
        DiagramKind::MindMap => puml::DiagramFamily::MindMap,
        DiagramKind::Wbs => puml::DiagramFamily::Wbs,
        DiagramKind::Gantt => puml::DiagramFamily::Gantt,
        DiagramKind::Chronology => puml::DiagramFamily::Chronology,
        DiagramKind::Json => puml::DiagramFamily::Json,
        DiagramKind::Yaml => puml::DiagramFamily::Yaml,
        DiagramKind::Nwdiag => puml::DiagramFamily::Nwdiag,
        DiagramKind::Archimate => puml::DiagramFamily::Archimate,
        DiagramKind::Regex => puml::DiagramFamily::Regex,
        DiagramKind::Ebnf => puml::DiagramFamily::Ebnf,
        DiagramKind::Math => puml::DiagramFamily::Math,
        DiagramKind::Sdl => puml::DiagramFamily::Sdl,
        DiagramKind::Ditaa => puml::DiagramFamily::Ditaa,
        DiagramKind::Chart => puml::DiagramFamily::Chart,
        DiagramKind::Unknown => puml::DiagramFamily::Unknown,
    })
}

fn wasm_parse_options(frontend: FrontendSelection) -> ParsePipelineOptions {
    ParsePipelineOptions {
        frontend,
        no_url_includes: true,
        ..ParsePipelineOptions::default()
    }
}

fn frontend_selection_from_hint(raw: &str) -> Result<FrontendSelection, Diagnostic> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "auto" | "puml" | "pumlx" => Ok(FrontendSelection::Auto),
        "plantuml" | "uml" | "puml-sequence" | "uml-sequence" => Ok(FrontendSelection::Plantuml),
        "mermaid" | "mmd" => Ok(FrontendSelection::Mermaid),
        "picouml" | "pico" => Ok(FrontendSelection::Picouml),
        other => Err(Diagnostic::error_code(
            "E_FRONTEND_UNKNOWN",
            format!("unknown frontend/dialect hint `{other}`"),
        )),
    }
}

fn render_svg_pages_from_model(model: &NormalizedDocument) -> Vec<String> {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            let scenes = layout::layout_pages(sequence, puml::LayoutOptions::default());
            scenes.iter().map(render::render_svg).collect::<Vec<_>>()
        }
        NormalizedDocument::Family(family) => vec![render_family_document_svg(family)],
        NormalizedDocument::Timeline(timeline) => vec![render::render_timeline_svg(timeline)],
        NormalizedDocument::State(state) => vec![render::render_state_svg(state)],
        NormalizedDocument::Json(doc) => vec![render::render_json_svg(doc)],
        NormalizedDocument::Yaml(doc) => vec![render::render_yaml_svg(doc)],
        NormalizedDocument::Nwdiag(doc) => vec![render::render_nwdiag_svg(doc)],
        NormalizedDocument::Archimate(doc) => vec![render::render_archimate_svg(doc)],
        NormalizedDocument::Regex(doc) => vec![render::render_regex_svg(doc)],
        NormalizedDocument::Ebnf(doc) => vec![render::render_ebnf_svg(doc)],
        NormalizedDocument::Math(doc) => vec![render::render_math_svg(doc)],
        NormalizedDocument::Sdl(doc) => vec![render::render_sdl_svg(doc)],
        NormalizedDocument::Ditaa(doc) => vec![render::render_ditaa_svg(doc)],
        NormalizedDocument::Chart(doc) => vec![render::render_chart_svg(doc)],
    }
}

fn render_family_document_svg(family: &FamilyDocument) -> String {
    match family.kind {
        DiagramKind::Salt => render::render_salt_svg(family),
        DiagramKind::Component => render::render_component_svg(family),
        DiagramKind::Deployment => render::render_deployment_svg(family),
        DiagramKind::Activity => render::render_activity_svg(family),
        DiagramKind::Timing => render::render_timing_svg(family),
        DiagramKind::MindMap => render::render_mindmap_svg(family),
        DiagramKind::Wbs => render::render_wbs_svg(family),
        _ => render::render_family_stub_svg(family),
    }
}

fn diagnostic_to_js(diag: puml::Diagnostic) -> JsValue {
    JsValue::from_str(&diag.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn ok_pages(json: &str) -> Vec<String> {
        let parsed: Value = serde_json::from_str(json).expect("json");
        parsed["ok"]
            .as_array()
            .expect("ok pages")
            .iter()
            .map(|v| v.as_str().expect("svg").to_string())
            .collect()
    }

    #[test]
    fn render_json_with_frontend_routes_picouml_sequence() {
        let pages = ok_pages(&render_svgs_json_with_frontend(
            "@startpicouml\nAlice => Bob : request\n@endpicouml\n",
            "picouml",
        ));
        assert_eq!(pages.len(), 1);
        assert!(pages[0].contains("<svg"));
        assert!(pages[0].contains("request"));
    }

    #[test]
    fn render_json_with_frontend_routes_mermaid_class() {
        let pages = ok_pages(&render_svgs_json_with_frontend(
            "classDiagram\nclass User\nUser : +id\n",
            "mermaid",
        ));
        assert_eq!(pages.len(), 1);
        assert!(pages[0].contains("<svg"));
        assert!(pages[0].contains("User"));
    }

    #[test]
    fn render_json_with_frontend_rejects_unknown_hint() {
        let parsed: Value = serde_json::from_str(&render_svgs_json_with_frontend(
            "@startuml\nA -> B\n@enduml\n",
            "bogus",
        ))
        .expect("json");
        assert_eq!(parsed["error"]["code"], "E_FRONTEND_UNKNOWN");
    }
}
