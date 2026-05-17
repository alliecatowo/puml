//! In-browser puml renderer. Exposes `render_svg` / `render_svgs` to JS via
//! wasm-bindgen so the studio editor can render diagrams without a backend.

use wasm_bindgen::prelude::*;

/// Render a single-page puml diagram to SVG. Multi-page diagrams return an
/// error; callers wanting multi-page should use [`render_svgs`].
#[wasm_bindgen]
pub fn render_svg(source: &str) -> Result<String, JsValue> {
    puml::render_source_to_svg(source).map_err(diagnostic_to_js)
}

/// Render every page of a multi-page puml diagram to its own SVG string.
#[wasm_bindgen]
pub fn render_svgs(source: &str) -> Result<Box<[JsValue]>, JsValue> {
    puml::render_source_to_svgs(source)
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
    match puml::render_source_to_svgs(source) {
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
    puml::detect_diagram_family(source)
        .map(|f| f.as_str().to_string())
        .map_err(diagnostic_to_js)
}

fn diagnostic_to_js(diag: puml::Diagnostic) -> JsValue {
    JsValue::from_str(&diag.message)
}
