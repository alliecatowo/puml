use super::protocol::notif;
use puml::diagnostic::Severity;
use puml::language_service::diagnostics_with_options;
use puml::ParsePipelineOptions;
use serde_json::{json, Value};
use std::io::{self, Write};

pub fn pub_diag(w: &mut impl Write, uri: &str, ver: i64, src: &str) -> io::Result<()> {
    let report = diagnostics_with_options(src, &ParsePipelineOptions::default());
    let ds = report
        .diagnostics
        .iter()
        .map(language_diagnostic_to_lsp_value)
        .collect::<Vec<_>>();
    notif(
        w,
        "textDocument/publishDiagnostics",
        json!({"uri":uri,"version":ver,"diagnostics":ds}),
    )
}

pub fn language_diagnostic_to_lsp_value(
    diagnostic: &puml::language_service::LanguageDiagnostic,
) -> Value {
    let range = diagnostic
        .range
        .map(|range| {
            json!({
                "start": {
                    "line": range.start.line.saturating_sub(1),
                    "character": range.start.column.saturating_sub(1)
                },
                "end": {
                    "line": range.end.line.saturating_sub(1),
                    "character": range.end.column.saturating_sub(1)
                }
            })
        })
        .unwrap_or_else(
            || json!({"start":{"line":0,"character":0},"end":{"line":0,"character":1}}),
        );
    json!({
        "range": range,
        "severity":lsp_severity(diagnostic.severity),
        "source":"puml",
        "code":diagnostic.code.clone(),
        "message":diagnostic.message.clone()
    })
}

fn lsp_severity(severity: Severity) -> i32 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_diagnostics_includes_diagnostic_code_when_present() {
        let mut out = Vec::new();
        let src = "@startuml\nA ->\n@enduml\n";
        pub_diag(&mut out, "file:///a.puml", 3, src).expect("publish diagnostics");

        let raw = String::from_utf8(out).expect("utf8");
        let payload = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("lsp frame");
        let msg: Value = serde_json::from_str(payload).expect("json frame");
        let first = msg["params"]["diagnostics"][0].clone();
        assert_eq!(first["source"], "puml");
        assert_eq!(first["severity"], 1);
        assert_eq!(first["code"], "E_ARROW_INVALID");
    }

    #[test]
    fn publish_diagnostics_does_not_fetch_url_includes() {
        let mut out = Vec::new();
        let src = "@startuml\n!include https://example.com/remote.puml\n@enduml\n";
        pub_diag(&mut out, "file:///a.puml", 3, src).expect("publish diagnostics");

        let raw = String::from_utf8(out).expect("utf8");
        let payload = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("lsp frame");
        let msg: Value = serde_json::from_str(payload).expect("json frame");
        let first = msg["params"]["diagnostics"][0].clone();
        assert_eq!(first["source"], "puml");
        assert_eq!(first["severity"], 1);
        assert_eq!(first["code"], "E_INCLUDE_URL_DISABLED");
    }
}
