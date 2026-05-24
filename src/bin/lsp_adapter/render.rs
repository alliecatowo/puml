use puml::{
    normalize_family, parse_with_pipeline_options, render_svg_pages_from_model, Document,
    FrontendSelection, ParsePipelineOptions,
};
use serde_json::{json, Value};

pub fn render_result(src: &str, frontend: Option<FrontendSelection>) -> Value {
    match lsp_parse_with_frontend(src, frontend).and_then(normalize_family) {
        Ok(model) => {
            let pages = render_svg_pages_from_model(&model);
            json!({
                "svg": pages.first().cloned().unwrap_or_default(),
                "svgs": pages,
                "width": 0,
                "height": 0,
                "diagnostics": []
            })
        }
        Err(d) => json!({
            "svg": "",
            "svgs": [],
            "width": 0,
            "height": 0,
            "diagnostics": [d.to_json_with_source(src)]
        }),
    }
}

pub fn lsp_parse(src: &str) -> Result<Document, puml::Diagnostic> {
    lsp_parse_with_frontend(src, None)
}

pub fn lsp_parse_with_frontend(
    src: &str,
    frontend: Option<FrontendSelection>,
) -> Result<Document, puml::Diagnostic> {
    parse_with_pipeline_options(
        src,
        &ParsePipelineOptions {
            frontend: frontend.unwrap_or(FrontendSelection::Auto),
            ..ParsePipelineOptions::default()
        },
    )
}

pub fn lsp_frontend_hint(params: &Value) -> Option<FrontendSelection> {
    let raw = params
        .get("frontend")
        .or_else(|| params.get("dialect"))
        .or_else(|| params.get("language"))
        .and_then(Value::as_str)?;
    frontend_selection_from_hint(raw)
}

fn frontend_selection_from_hint(raw: &str) -> Option<FrontendSelection> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "auto" | "puml" | "pumlx" => Some(FrontendSelection::Auto),
        "plantuml" | "uml" | "puml-sequence" | "uml-sequence" => Some(FrontendSelection::Plantuml),
        "mermaid" | "mmd" => Some(FrontendSelection::Mermaid),
        "picouml" | "pico" => Some(FrontendSelection::Picouml),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_result_uses_family_renderer_for_class_state_and_activity() {
        let cases = [
            ("@startuml\nclass User\n@enduml\n", "User"),
            (
                "@startuml\nstate Waiting\n[*] --> Waiting\n@enduml\n",
                "Waiting",
            ),
            ("@startuml\nstart\n:Work;\nstop\n@enduml\n", "Work"),
        ];

        for (src, needle) in cases {
            let out = render_result(src, None);
            assert_eq!(out["diagnostics"].as_array().expect("diagnostics").len(), 0);
            let svg = out["svg"].as_str().expect("svg");
            assert!(svg.contains("<svg"));
            assert!(
                svg.contains(needle),
                "family render output should contain {needle}: {svg}"
            );
        }
    }

    #[test]
    fn render_result_honors_frontend_hint_for_mermaid_and_picouml() {
        let mermaid = render_result(
            "classDiagram\nclass User\nUser : +id\n",
            Some(FrontendSelection::Mermaid),
        );
        assert_eq!(
            mermaid["diagnostics"]
                .as_array()
                .expect("mermaid diagnostics")
                .len(),
            0
        );
        assert!(mermaid["svg"]
            .as_str()
            .expect("mermaid svg")
            .contains("User"));

        let picouml = render_result(
            "@startpicouml\nAlice => Bob : request\n@endpicouml\n",
            Some(FrontendSelection::Picouml),
        );
        assert_eq!(
            picouml["diagnostics"]
                .as_array()
                .expect("picouml diagnostics")
                .len(),
            0
        );
        assert!(picouml["svg"]
            .as_str()
            .expect("picouml svg")
            .contains("request"));
    }
}
