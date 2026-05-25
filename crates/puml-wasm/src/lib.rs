//! In-browser puml renderer. Exposes `render_svg` / `render_svgs` to JS via
//! wasm-bindgen so the studio editor can render diagrams without a backend.

use puml::ast::DiagramKind;
use puml::diagnostic::Severity;
use puml::language_service::{
    diagnostics_with_options, language_service_surface_json, semantic_tokens, SemanticTokenKind,
};
use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model, Diagnostic,
    FrontendSelection, ParsePipelineOptions,
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

/// JSON-encoded Studio compile DTO for browser/runtime callers.
#[wasm_bindgen]
pub fn compile_json(source: &str) -> String {
    compile_json_for_frontend(source, FrontendSelection::Auto)
}

/// JSON-encoded Studio compile DTO using an explicit frontend/dialect hint.
#[wasm_bindgen]
pub fn compile_json_with_frontend(source: &str, frontend: &str) -> String {
    let frontend = match frontend_selection_from_hint(frontend) {
        Ok(frontend) => frontend,
        Err(diag) => {
            return serde_json::json!({
                "schema": "puml.compile",
                "schemaVersion": 1,
                "ok": false,
                "family": "unknown",
                "pages": [],
                "diagnostics": [diag.to_json_with_source(source)],
                "semanticTokens": semantic_tokens_json(source),
                "languageService": language_service_surface_json(),
            })
            .to_string();
        }
    };
    compile_json_for_frontend(source, frontend)
}

/// JSON-encoded language-service surface for Studio workers and editor adapters.
#[wasm_bindgen]
pub fn language_service_json() -> String {
    language_service_surface_json().to_string()
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
    Ok(render_artifact_pages_from_model(&model)
        .into_iter()
        .map(|artifact| artifact.svg)
        .collect())
}

fn compile_json_for_frontend(source: &str, frontend: FrontendSelection) -> String {
    let options = wasm_parse_options(frontend);
    let diagnostics = diagnostics_with_options(source, &options)
        .diagnostics
        .iter()
        .map(language_diagnostic_json)
        .collect::<Vec<_>>();

    match render_svgs_for_frontend(source, frontend) {
        Ok(pages) => {
            let family = detect_family_for_frontend(source, frontend)
                .map(|family| family.as_str().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            serde_json::json!({
                "schema": "puml.compile",
                "schemaVersion": 1,
                "ok": true,
                "family": family,
                "pageCount": pages.len(),
                "pages": pages.iter().enumerate().map(|(index, svg)| {
                    serde_json::json!({
                        "index": index,
                        "format": "svg",
                        "svg": svg,
                    })
                }).collect::<Vec<_>>(),
                "diagnostics": diagnostics,
                "semanticTokens": semantic_tokens_json(source),
                "languageService": language_service_surface_json(),
            })
            .to_string()
        }
        Err(diag) => serde_json::json!({
            "schema": "puml.compile",
            "schemaVersion": 1,
            "ok": false,
            "family": "unknown",
            "pageCount": 0,
            "pages": [],
            "diagnostics": if diagnostics.is_empty() {
                vec![serde_json::to_value(diag.to_json_with_source(source))
                    .expect("diagnostic json serializes")]
            } else {
                diagnostics
            },
            "semanticTokens": semantic_tokens_json(source),
            "languageService": language_service_surface_json(),
        })
        .to_string(),
    }
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
        DiagramKind::Stdlib => puml::DiagramFamily::Stdlib,
        DiagramKind::Chen => puml::DiagramFamily::Chen,
        DiagramKind::Board => puml::DiagramFamily::Board,
        DiagramKind::Files => puml::DiagramFamily::Files,
        DiagramKind::Wire => puml::DiagramFamily::Wire,
        DiagramKind::Unknown => puml::DiagramFamily::Unknown,
    })
}

fn wasm_parse_options(frontend: FrontendSelection) -> ParsePipelineOptions {
    ParsePipelineOptions {
        frontend,
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

fn diagnostic_to_js(diag: puml::Diagnostic) -> JsValue {
    JsValue::from_str(&diag.message)
}

fn language_diagnostic_json(
    diagnostic: &puml::language_service::LanguageDiagnostic,
) -> serde_json::Value {
    serde_json::json!({
        "code": diagnostic.code,
        "severity": severity_name(diagnostic.severity),
        "message": diagnostic.message,
        "span": diagnostic.span.map(|span| serde_json::json!({
            "start": span.start,
            "end": span.end,
        })),
        "range": diagnostic.range.map(|range| serde_json::json!({
            "start": {
                "line": range.start.line,
                "column": range.start.column,
            },
            "end": {
                "line": range.end.line,
                "column": range.end.column,
            },
        })),
    })
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn semantic_tokens_json(source: &str) -> Vec<serde_json::Value> {
    semantic_tokens(source)
        .into_iter()
        .map(|token| {
            serde_json::json!({
                "start": token.span.start,
                "end": token.span.end,
                "kind": match token.kind {
                    SemanticTokenKind::Keyword => "keyword",
                    SemanticTokenKind::Operator => "operator",
                },
            })
        })
        .collect()
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
    fn compile_json_reports_pages_family_diagnostics_and_tokens() {
        let parsed: Value =
            serde_json::from_str(&compile_json("@startuml\nclass User\n@enduml\n")).expect("json");

        assert_eq!(parsed["schema"], "puml.compile");
        assert_eq!(parsed["schemaVersion"], 1);
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["family"], "class");
        assert_eq!(parsed["pageCount"], 1);
        assert!(parsed["pages"][0]["svg"]
            .as_str()
            .expect("svg")
            .contains("User"));
        assert!(parsed["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty());
        assert!(parsed["semanticTokens"]
            .as_array()
            .expect("semantic tokens")
            .iter()
            .any(|token| token["kind"] == "keyword"));
        assert_eq!(parsed["languageService"]["schema"], "puml.languageService");
        assert!(parsed["languageService"]["completion"]["items"]
            .as_array()
            .expect("completion items")
            .iter()
            .any(|item| item["label"] == "component"));
    }

    #[test]
    fn language_service_json_reports_shared_editor_surface() {
        let parsed: Value = serde_json::from_str(&language_service_json()).expect("json");

        assert_eq!(parsed["schema"], "puml.languageService");
        assert!(parsed["families"]
            .as_array()
            .expect("families")
            .iter()
            .any(|family| family["name"] == "sequence"));
        assert!(parsed["completion"]["items"]
            .as_array()
            .expect("completion items")
            .iter()
            .any(|item| item["label"] == "ArrowColor"
                && item["documentation"]
                    .as_str()
                    .expect("documentation")
                    .contains("Value type: color")));
    }

    #[test]
    fn compile_json_with_frontend_preserves_mermaid_warning_contract() {
        let parsed: Value = serde_json::from_str(&compile_json_with_frontend(
            "flowchart LR\nclassDef hot fill:#fef3c7,stroke:#92400e\nA[API]:::hot --> B\n",
            "mermaid",
        ))
        .expect("json");

        assert_eq!(parsed["schema"], "puml.compile");
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["family"], "component");
        assert_eq!(parsed["diagnostics"][0]["code"], "W_MERMAID_STYLE_PARTIAL");
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

    #[test]
    fn internal_render_frontend_returns_all_pages_without_js_values() {
        let source = "@startuml\nAlice -> Bob: one\nnewpage two\nBob -> Alice: two\n@enduml\n";

        let pages =
            render_svgs_for_frontend(source, FrontendSelection::Auto).expect("multi-page render");
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains("one"));
        assert!(pages[1].contains("two"));
    }

    #[test]
    fn detect_family_covers_frontend_hints_and_errors() {
        assert_eq!(
            detect_family_for_frontend("@startuml\nstate Idle\n@enduml\n", FrontendSelection::Auto)
                .expect("detect state family"),
            puml::DiagramFamily::State
        );
        assert_eq!(
            detect_family_for_frontend("classDiagram\nclass User\n", FrontendSelection::Mermaid)
                .expect("detect mermaid class"),
            puml::DiagramFamily::Class
        );

        let err = frontend_selection_from_hint("unknown").expect_err("unknown frontend hint");
        assert!(err
            .message
            .contains("unknown frontend/dialect hint `unknown`"));
    }

    #[test]
    fn render_json_auto_routes_specialized_families_and_reports_parse_errors() {
        let specialized = ok_pages(&render_svgs_json("@startregex\n[A-Z]+\\d?\n@endregex\n"));
        assert_eq!(specialized.len(), 1);
        assert!(specialized[0].contains("<svg"));
        assert!(specialized[0].contains("[A-Z]"));

        let parsed: Value =
            serde_json::from_str(&render_svgs_json("@startuml\nAlice ->\n@enduml\n"))
                .expect("json");
        assert_eq!(parsed["error"]["code"], "E_ARROW_INVALID");
        assert!(parsed["error"]["message"].as_str().expect("message").len() > 8);
    }
}
