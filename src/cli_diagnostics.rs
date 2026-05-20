use crate::cli::DiagnosticsFormat;
use crate::{cli_input::map_diagnostic_span, EXIT_VALIDATION};
use puml::{Diagnostic, DiagnosticJson, NormalizedDocument};
use serde::Serialize;
use std::io::{self, IsTerminal};

const DIAGNOSTICS_SCHEMA: &str = "puml.diagnostics";
const DIAGNOSTICS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct DiagnosticsPayload {
    schema: &'static str,
    schema_version: u32,
    diagnostics: Vec<DiagnosticJson>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DiagnosticOutput {
    pub(crate) format: DiagnosticsFormat,
    pub(crate) color_enabled: bool,
}

pub(crate) fn should_color_human_diagnostics(choice: crate::cli::ColorChoice) -> bool {
    match choice {
        crate::cli::ColorChoice::Always => true,
        crate::cli::ColorChoice::Never => false,
        crate::cli::ColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal()
        }
    }
}

pub(crate) fn lsp_capabilities_manifest() -> serde_json::Value {
    puml::lsp_capabilities()
}

pub(crate) fn diag_err_with_source(
    source: &str,
    d: Diagnostic,
    output: DiagnosticOutput,
) -> (u8, String) {
    match output.format {
        DiagnosticsFormat::Human => (
            EXIT_VALIDATION,
            render_human_diagnostic(&d, source, output.color_enabled),
        ),
        DiagnosticsFormat::Json => (EXIT_VALIDATION, diagnostics_json_payload(vec![d], source)),
        DiagnosticsFormat::Stdrpt => (EXIT_VALIDATION, diagnostic_stdrpt(&d, source)),
    }
}

pub(crate) fn diagnostic_stdrpt(d: &Diagnostic, source: &str) -> String {
    let json = d.to_json_with_source(source);
    let code = json.code.unwrap_or_default();
    let location = match (json.line, json.column) {
        (Some(line), Some(col)) => format!("-:{line}:{col}"),
        _ => "-".to_string(),
    };
    format!("{}\t{}\t{}\t{}", json.severity, code, location, d.message)
}

pub(crate) fn render_human_diagnostic(d: &Diagnostic, source: &str, color_enabled: bool) -> String {
    let rendered = d.render_with_source(source);
    if !color_enabled {
        return rendered;
    }

    let (first, rest) = rendered.split_once('\n').unwrap_or((&rendered, ""));
    let header = match d.severity {
        puml::diagnostic::Severity::Error => ansi(first, "1;31"),
        puml::diagnostic::Severity::Warning => ansi(first, "1;33"),
    };
    if rest.is_empty() {
        return header;
    }

    let mut out = String::new();
    out.push_str(&header);
    for line in rest.lines() {
        out.push('\n');
        if line.trim_start().starts_with('^') {
            out.push_str(&ansi(line, "1;36"));
        } else {
            out.push_str(line);
        }
    }
    out
}

fn ansi(text: &str, code: &str) -> String {
    format!("\x1b[{code}m{text}\x1b[0m")
}

pub(crate) fn diag_err_mapped(
    raw_source: &str,
    mapping: Option<puml::source::Span>,
    d: Diagnostic,
    output: DiagnosticOutput,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source(raw_source, mapped, output)
}

pub(crate) fn emit_warnings_for_model(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<puml::source::Span>,
    output: DiagnosticOutput,
) {
    for warning in normalized_warnings(model) {
        let warning = map_diagnostic_span(warning.clone(), mapping);
        match output.format {
            DiagnosticsFormat::Human => eprintln!(
                "{}",
                render_human_diagnostic(&warning, source, output.color_enabled)
            ),
            DiagnosticsFormat::Json => {
                eprintln!("{}", diagnostics_json_payload(vec![warning], source));
            }
            DiagnosticsFormat::Stdrpt => eprintln!("{}", diagnostic_stdrpt(&warning, source)),
        }
    }
}

pub(crate) fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
        NormalizedDocument::Timeline(timeline) => &timeline.warnings,
        NormalizedDocument::State(state) => &state.warnings,
        NormalizedDocument::Json(doc) => &doc.warnings,
        NormalizedDocument::Yaml(doc) => &doc.warnings,
        NormalizedDocument::Nwdiag(doc) => &doc.warnings,
        NormalizedDocument::Archimate(doc) => &doc.warnings,
        NormalizedDocument::Regex(doc) => &doc.warnings,
        NormalizedDocument::Ebnf(doc) => &doc.warnings,
        NormalizedDocument::Math(doc) => &doc.warnings,
        NormalizedDocument::Sdl(doc) => &doc.warnings,
        NormalizedDocument::Ditaa(doc) => &doc.warnings,
        NormalizedDocument::Chart(doc) => &doc.warnings,
        NormalizedDocument::Chen(doc) => &doc.warnings,
    }
}

pub(crate) fn diagnostics_json_payload(diags: Vec<Diagnostic>, source: &str) -> String {
    let payload = DiagnosticsPayload {
        schema: DIAGNOSTICS_SCHEMA,
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        diagnostics: diags
            .iter()
            .map(|d| d.to_json_with_source(source))
            .collect::<Vec<_>>(),
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"schema\":\"puml.diagnostics\",\"schema_version\":1,\"diagnostics\":[{\"code\":null,\"severity\":\"error\",\"message\":\"failed to serialize diagnostics\",\"span\":null,\"line\":null,\"column\":null,\"snippet\":null,\"caret\":null}]}".to_string()
    })
}

pub(crate) fn diagnostics_json_payload_precomputed(diags: Vec<DiagnosticJson>) -> String {
    let payload = DiagnosticsPayload {
        schema: DIAGNOSTICS_SCHEMA,
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        diagnostics: diags,
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"schema\":\"puml.diagnostics\",\"schema_version\":1,\"diagnostics\":[{\"code\":null,\"severity\":\"error\",\"message\":\"failed to serialize diagnostics\",\"span\":null,\"line\":null,\"column\":null,\"snippet\":null,\"caret\":null}]}".to_string()
    })
}
