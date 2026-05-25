use puml::{
    normalize_family, normalized_model_summary_to_json, normalized_scene_summary_to_json,
    parse_with_pipeline_options, render_svg_pages_from_model, Document, FrontendSelection,
    ParsePipelineOptions,
};
use puml::{
    output::{render_output_bytes, render_svg_export_content, OutputFormat, RenderedOutput},
    render::render_text_pages,
};
use serde_json::{json, Value};

pub fn render_result(src: &str, frontend: Option<FrontendSelection>) -> Value {
    match lsp_parse_with_frontend(src, frontend).and_then(normalize_family) {
        Ok(model) => {
            let pages = render_svg_pages_from_model(&model);
            let scene = normalized_scene_summary_to_json(&model);
            let (width, height) = scene_dimensions(&scene);
            json!({
                "schema": "puml.renderSvg",
                "schemaVersion": 1,
                "svg": pages.first().cloned().unwrap_or_default(),
                "svgs": pages,
                "width": width,
                "height": height,
                "model": normalized_model_summary_to_json(&model),
                "scene": scene,
                "diagnostics": []
            })
        }
        Err(d) => json!({
            "schema": "puml.renderSvg",
            "schemaVersion": 1,
            "svg": "",
            "svgs": [],
            "width": 0,
            "height": 0,
            "diagnostics": [d.to_json_with_source(src)]
        }),
    }
}

pub fn render_scene_result(src: &str, frontend: Option<FrontendSelection>) -> Value {
    match lsp_parse_with_frontend(src, frontend).and_then(normalize_family) {
        Ok(model) => json!({
            "schema": "puml.renderScene",
            "schemaVersion": 1,
            "model": normalized_model_summary_to_json(&model),
            "scene": normalized_scene_summary_to_json(&model),
            "diagnostics": []
        }),
        Err(d) => json!({
            "schema": "puml.renderScene",
            "schemaVersion": 1,
            "model": null,
            "scene": null,
            "diagnostics": [d.to_json_with_source(src)]
        }),
    }
}

pub fn export_result(
    src: &str,
    frontend: Option<FrontendSelection>,
    format: OutputFormat,
) -> Value {
    match lsp_parse_with_frontend(src, frontend).and_then(normalize_family) {
        Ok(model) => {
            let rendered = match format.text_mode() {
                Some(mode) => render_text_pages(&model, mode),
                None => render_svg_pages_from_model(&model)
                    .into_iter()
                    .map(|svg| render_svg_export_content(&svg, format))
                    .collect(),
            };
            let pages = rendered
                .iter()
                .enumerate()
                .map(|(idx, content)| export_page(format, idx, content))
                .collect::<Result<Vec<_>, _>>();

            match pages {
                Ok(pages) => {
                    let first = pages.first().cloned().unwrap_or_else(
                        || json!({"name": format!("diagram-1.{}", format.extension())}),
                    );
                    json!({
                        "schema": "puml.export",
                        "schemaVersion": 1,
                        "format": format.extension(),
                        "mediaType": media_type(format),
                        "encoding": if format.is_binary() { "base64" } else { "utf-8" },
                        "content": first.get("content").cloned().unwrap_or(Value::Null),
                        "contentBase64": first.get("contentBase64").cloned().unwrap_or(Value::Null),
                        "pages": pages,
                        "model": normalized_model_summary_to_json(&model),
                        "scene": normalized_scene_summary_to_json(&model),
                        "diagnostics": []
                    })
                }
                Err(message) => json!({
                    "schema": "puml.export",
                    "schemaVersion": 1,
                    "format": format.extension(),
                    "mediaType": media_type(format),
                    "encoding": if format.is_binary() { "base64" } else { "utf-8" },
                    "content": null,
                    "contentBase64": null,
                    "pages": [],
                    "diagnostics": [{
                        "code": "E_EXPORT_FAILED",
                        "severity": "error",
                        "message": message
                    }]
                }),
            }
        }
        Err(d) => json!({
            "schema": "puml.export",
            "schemaVersion": 1,
            "format": format.extension(),
            "mediaType": media_type(format),
            "encoding": if format.is_binary() { "base64" } else { "utf-8" },
            "content": null,
            "contentBase64": null,
            "pages": [],
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

pub fn output_format_from_hint(raw: &str) -> Option<OutputFormat> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "svg" => Some(OutputFormat::Svg),
        "html" | "htm" => Some(OutputFormat::Html),
        "png" => Some(OutputFormat::Png),
        "jpg" | "jpeg" => Some(OutputFormat::Jpg),
        "webp" => Some(OutputFormat::Webp),
        "pdf" => Some(OutputFormat::Pdf),
        "txt" => Some(OutputFormat::Txt),
        "atxt" => Some(OutputFormat::Atxt),
        "utxt" => Some(OutputFormat::Utxt),
        _ => None,
    }
}

fn scene_dimensions(scene: &Value) -> (i64, i64) {
    if let Some(first_page) = scene.pointer("/pages/0") {
        return (
            numeric_dimension(
                first_page
                    .pointer("/size/width")
                    .or_else(|| first_page.pointer("/viewport/width")),
            ),
            numeric_dimension(
                first_page
                    .pointer("/size/height")
                    .or_else(|| first_page.pointer("/viewport/height")),
            ),
        );
    }
    (
        numeric_dimension(scene.pointer("/viewport/width")),
        numeric_dimension(scene.pointer("/viewport/height")),
    )
}

fn numeric_dimension(value: Option<&Value>) -> i64 {
    value
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|number| number.round() as i64))
        })
        .unwrap_or(0)
}

fn export_page(format: OutputFormat, idx: usize, content: &str) -> Result<Value, String> {
    let name = format!("diagram-{}.{}", idx + 1, format.extension());
    if !format.is_binary() {
        return Ok(json!({"name": name, "content": content}));
    }
    let output = RenderedOutput {
        name_hint: Some(name.clone()),
        content: content.to_string(),
    };
    let rendered = render_output_bytes(&output, format, 96.0).map_err(|err| err.to_string())?;
    Ok(json!({
        "name": rendered.name_hint.unwrap_or(name),
        "contentBase64": base64_encode(&rendered.bytes)
    }))
}

fn media_type(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Svg => "image/svg+xml",
        OutputFormat::Html => "text/html",
        OutputFormat::Png => "image/png",
        OutputFormat::Jpg => "image/jpeg",
        OutputFormat::Webp => "image/webp",
        OutputFormat::Pdf => "application/pdf",
        OutputFormat::Txt | OutputFormat::Atxt | OutputFormat::Utxt => "text/plain",
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
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
