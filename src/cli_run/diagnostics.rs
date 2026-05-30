use super::EXIT_VALIDATION;
use crate::cli::{ColorChoice as CliColorChoice, DiagnosticsFormat};
use puml::diagnostic::{diagnostic_message_and_code, normalized_warnings};
use puml::source::Span;
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
pub(super) struct DiagnosticOutput {
    pub(super) format: DiagnosticsFormat,
    pub(super) color_enabled: bool,
}

/// Diagnostic hint codes that trigger an actionable suggestion in human output.
const HINT_URL_DISABLED: &[&str] = &["E_INCLUDE_URL_DISABLED", "E_IMPORT_URL_DISABLED"];

/// Return an optional hint string for a diagnostic message.
/// Hints are shown only in human format, appended after the main diagnostic.
pub(super) fn hint_for_diagnostic(message: &str) -> Option<&'static str> {
    // URL include rejection → suggest --allow-url-includes
    for code in HINT_URL_DISABLED {
        if message.contains(code) {
            return Some("hint: rerun with --allow-url-includes to permit URL includes");
        }
    }
    // Multiple diagrams or pages on stdin → suggest --multi
    if message.contains("multiple diagrams detected")
        || message.contains("multiple pages detected")
        || message.contains("rerun with --multi")
    {
        return Some("hint: rerun with --multi to allow multiple @startuml blocks or pages");
    }
    None
}

/// Return an optional hint string for a plain (non-Diagnostic) error message.
/// Used for higher-level error strings that are not wrapped in a Diagnostic.
pub(super) fn hint_for_error_message(message: &str) -> Option<&'static str> {
    // No markdown fences found → suggest wrapping in a fence block
    if message.contains("no supported markdown diagram fences found") {
        return Some(
            "hint: wrap diagram source in a fenced code block, e.g. ```puml ... ``` (or plantuml, uml, mermaid)"
        );
    }
    hint_for_diagnostic(message)
}

pub(super) fn should_color_human_diagnostics(choice: CliColorChoice) -> bool {
    match choice {
        CliColorChoice::Always => true,
        CliColorChoice::Never => false,
        CliColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal()
        }
    }
}

pub(super) fn diag_err_with_source_label(
    source: &str,
    d: Diagnostic,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) -> (u8, String) {
    match output.format {
        DiagnosticsFormat::Human => {
            let mut rendered =
                render_human_diagnostic_label(&d, source, output.color_enabled, file_label);
            if let Some(hint) = hint_for_diagnostic(&d.message) {
                let hint_line = if output.color_enabled {
                    ansi(hint, "2")
                } else {
                    hint.to_string()
                };
                rendered.push('\n');
                rendered.push_str(&hint_line);
            }
            (EXIT_VALIDATION, rendered)
        }
        DiagnosticsFormat::Json => (EXIT_VALIDATION, diagnostics_json_payload(vec![d], source)),
        DiagnosticsFormat::Stdrpt => (EXIT_VALIDATION, diagnostic_stdrpt(&d, source)),
    }
}

/// Emit a hint line to stderr in human-format output.
/// Only emits when the output format is Human and there is a hint for the given message.
pub(super) fn emit_hint_for_message(message: &str, output: DiagnosticOutput) {
    if output.format != DiagnosticsFormat::Human {
        return;
    }
    if let Some(hint) = hint_for_diagnostic(message) {
        let hint_line = if output.color_enabled {
            ansi(hint, "2")
        } else {
            hint.to_string()
        };
        eprintln!("{hint_line}");
    }
}

pub(super) fn diagnostic_stdrpt(d: &Diagnostic, source: &str) -> String {
    let json = d.to_json_with_source(source);
    let code = json.code.unwrap_or_default();
    let location = match (json.line, json.column) {
        (Some(line), Some(col)) => {
            let file = json.file.unwrap_or_else(|| "-".to_string());
            format!("{file}:{line}:{col}")
        }
        _ => "-".to_string(),
    };
    format!("{}\t{}\t{}\t{}", json.severity, code, location, d.message)
}

pub(super) fn render_human_diagnostic(d: &Diagnostic, source: &str, color_enabled: bool) -> String {
    render_human_diagnostic_label(d, source, color_enabled, None)
}

fn render_human_diagnostic_label(
    d: &Diagnostic,
    source: &str,
    color_enabled: bool,
    file_label: Option<&str>,
) -> String {
    if let Some(label) = file_label {
        return render_human_diagnostic_with_file_label(d, source, color_enabled, label);
    }

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

fn render_human_diagnostic_with_file_label(
    d: &Diagnostic,
    source: &str,
    color_enabled: bool,
    file_label: &str,
) -> String {
    let (message, code) = diagnostic_message_and_code(&d.message);
    let severity = match d.severity {
        puml::diagnostic::Severity::Error => "error",
        puml::diagnostic::Severity::Warning => "warning",
    };
    let severity_with_code = code
        .map(|code| format!("{severity}[{code}]"))
        .unwrap_or_else(|| severity.to_string());
    let location = d
        .line_col(source)
        .map(|(line, col)| format!("{file_label}:{line}:{col}"))
        .unwrap_or_else(|| file_label.to_string());
    let header = format!("{location}: {severity_with_code}: {message}");
    let header = if color_enabled {
        match d.severity {
            puml::diagnostic::Severity::Error => ansi(&header, "1;31"),
            puml::diagnostic::Severity::Warning => ansi(&header, "1;33"),
        }
    } else {
        header
    };

    let Some(span) = d.span else {
        return header;
    };
    let caret = puml::diagnostic::render_caret_line(source, span);
    let mut out = header;
    for line in caret.lines() {
        out.push('\n');
        if color_enabled && line.trim_start().starts_with('^') {
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

pub(super) fn diag_err_mapped_label(
    raw_source: &str,
    mapping: Option<Span>,
    d: Diagnostic,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source_label(raw_source, mapped, output, file_label)
}

pub(super) fn emit_warnings_for_model(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<Span>,
    output: DiagnosticOutput,
) {
    emit_warnings_for_model_label(model, source, mapping, output, None);
}

pub(super) fn emit_diagnostics_label(
    diagnostics: &[Diagnostic],
    source: &str,
    mapping: Option<Span>,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) {
    for diagnostic in diagnostics {
        let diagnostic = map_diagnostic_span(diagnostic.clone(), mapping);
        match output.format {
            DiagnosticsFormat::Human => eprintln!(
                "{}",
                render_human_diagnostic_label(
                    &diagnostic,
                    source,
                    output.color_enabled,
                    file_label
                )
            ),
            DiagnosticsFormat::Json => {
                eprintln!("{}", diagnostics_json_payload(vec![diagnostic], source));
            }
            DiagnosticsFormat::Stdrpt => eprintln!("{}", diagnostic_stdrpt(&diagnostic, source)),
        }
    }
}

pub(super) fn emit_warnings_for_model_label(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<Span>,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) {
    for warning in normalized_warnings(model) {
        let warning = map_diagnostic_span(warning.clone(), mapping);
        match output.format {
            DiagnosticsFormat::Human => eprintln!(
                "{}",
                render_human_diagnostic_label(&warning, source, output.color_enabled, file_label)
            ),
            DiagnosticsFormat::Json => {
                eprintln!("{}", diagnostics_json_payload(vec![warning], source));
            }
            DiagnosticsFormat::Stdrpt => eprintln!("{}", diagnostic_stdrpt(&warning, source)),
        }
    }
}
pub(super) fn map_diagnostic_span(mut d: Diagnostic, mapping: Option<Span>) -> Diagnostic {
    if let (Some(span), Some(base)) = (d.span, mapping) {
        d.span = Some(Span::new(base.start + span.start, base.start + span.end));
    }
    d
}

fn diagnostics_json_payload(diags: Vec<Diagnostic>, source: &str) -> String {
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

pub(super) fn diagnostics_json_payload_precomputed(diags: Vec<DiagnosticJson>) -> String {
    let payload = DiagnosticsPayload {
        schema: DIAGNOSTICS_SCHEMA,
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        diagnostics: diags,
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"schema\":\"puml.diagnostics\",\"schema_version\":1,\"diagnostics\":[{\"code\":null,\"severity\":\"error\",\"message\":\"failed to serialize diagnostics\",\"span\":null,\"line\":null,\"column\":null,\"snippet\":null,\"caret\":null}]}".to_string()
    })
}
