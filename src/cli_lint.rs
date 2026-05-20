use crate::cli::{Cli, DiagnosticsFormat, LintReportFormat};
use crate::cli_diagnostics::{
    diagnostic_stdrpt, diagnostics_json_payload_precomputed, emit_warnings_for_model,
    normalized_warnings, render_human_diagnostic, should_color_human_diagnostics, DiagnosticOutput,
};
use crate::cli_input::{
    frontend_hint_for_path, map_diagnostic_span, should_extract_markdown, split_diagrams,
};
use crate::{parse_for_cli, EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION};
use glob::glob;
use puml::{normalize_family, Diagnostic, DiagnosticJson};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const LINT_REPORT_SCHEMA: &str = "puml.lint_report";
const LINT_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct LintReportPayload {
    schema: &'static str,
    schema_version: u32,
    summary: LintSummary,
    files: Vec<LintFileResult>,
}

#[derive(Debug, Serialize)]
struct LintSummary {
    total_files: usize,
    passed_files: usize,
    failed_files: usize,
    total_diagrams: usize,
    passed_diagrams: usize,
    failed_diagrams: usize,
    warning_count: usize,
    error_count: usize,
}

#[derive(Debug, Serialize)]
struct LintFileResult {
    path: String,
    diagrams: usize,
    failed_diagrams: usize,
    warnings: usize,
    errors: usize,
    passed: bool,
}

#[derive(Debug, Default)]
struct LintFileAccumulator {
    diagrams: usize,
    failed_diagrams: usize,
    warnings: usize,
    errors: usize,
}

pub(crate) fn is_lint_mode_enabled(cli: &Cli) -> bool {
    !cli.lint_input.is_empty() || !cli.lint_glob.is_empty()
}

pub(crate) fn run_lint_mode(cli: &Cli) -> Result<(), (u8, String)> {
    if !cli.check {
        return Err((EXIT_VALIDATION, "lint mode requires --check".to_string()));
    }
    let diagnostics_output = DiagnosticOutput {
        format: cli.diagnostics,
        color_enabled: should_color_human_diagnostics(cli.color),
    };

    let inject_vars: BTreeMap<String, String> = cli.defines.iter().cloned().collect();
    let lint_paths = collect_lint_inputs(&cli.lint_input, &cli.lint_glob)?;
    if lint_paths.is_empty() {
        return Err((
            EXIT_VALIDATION,
            "lint mode resolved no input files".to_string(),
        ));
    }

    let mut files = Vec::new();
    let mut lint_json_diagnostics = Vec::new();

    for path in lint_paths {
        let mut acc = LintFileAccumulator::default();
        let path_display = path.display().to_string();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) => {
                acc.errors += 1;
                eprintln!("{}: failed to read: {e}", path.display());
                files.push(LintFileResult {
                    path: path_display,
                    diagrams: acc.diagrams,
                    failed_diagrams: acc.failed_diagrams,
                    warnings: acc.warnings,
                    errors: acc.errors,
                    passed: false,
                });
                continue;
            }
        };

        let include_root = cli
            .include_root
            .clone()
            .or_else(|| path.parent().map(|d| d.to_path_buf()));
        let from_markdown = should_extract_markdown(cli.from_markdown, Some(path.as_path()));
        let markdown_name_prefix = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_string());

        let file_frontend_hint = frontend_hint_for_path(Some(path.as_path()));
        let diagrams = match split_diagrams(
            &raw,
            from_markdown,
            markdown_name_prefix.as_deref(),
            file_frontend_hint,
        ) {
            Ok(diagrams) => diagrams,
            Err(d) => {
                acc.errors += 1;
                emit_lint_diagnostic(
                    &path,
                    &raw,
                    d,
                    diagnostics_output,
                    &mut lint_json_diagnostics,
                );
                files.push(LintFileResult {
                    path: path_display,
                    diagrams: acc.diagrams,
                    failed_diagrams: acc.failed_diagrams,
                    warnings: acc.warnings,
                    errors: acc.errors,
                    passed: false,
                });
                continue;
            }
        };

        if diagrams.is_empty() {
            acc.errors += 1;
            emit_lint_diagnostic(
                &path,
                &raw,
                Diagnostic::error("no diagram content provided"),
                diagnostics_output,
                &mut lint_json_diagnostics,
            );
            files.push(LintFileResult {
                path: path_display,
                diagrams: acc.diagrams,
                failed_diagrams: acc.failed_diagrams,
                warnings: acc.warnings,
                errors: acc.errors,
                passed: false,
            });
            continue;
        }

        for source in &diagrams {
            acc.diagrams += 1;
            let doc = match parse_for_cli(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                cli.determinism,
                source.frontend_hint,
                cli.allow_url_includes,
                inject_vars.clone(),
            ) {
                Ok(doc) => doc,
                Err(d) => {
                    acc.errors += 1;
                    acc.failed_diagrams += 1;
                    emit_lint_diagnostic(
                        &path,
                        &raw,
                        map_diagnostic_span(d, source.source_span),
                        diagnostics_output,
                        &mut lint_json_diagnostics,
                    );
                    continue;
                }
            };

            let model = match normalize_family(doc) {
                Ok(model) => model,
                Err(d) => {
                    acc.errors += 1;
                    acc.failed_diagrams += 1;
                    emit_lint_diagnostic(
                        &path,
                        &raw,
                        map_diagnostic_span(d, source.source_span),
                        diagnostics_output,
                        &mut lint_json_diagnostics,
                    );
                    continue;
                }
            };

            acc.warnings += normalized_warnings(&model).len();
            match diagnostics_output.format {
                DiagnosticsFormat::Human => {
                    emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output)
                }
                DiagnosticsFormat::Json => {
                    for warning in normalized_warnings(&model) {
                        let warning = map_diagnostic_span(warning.clone(), source.source_span);
                        let mut json = warning.to_json_with_source(&raw);
                        json.file = Some(path_display.clone());
                        lint_json_diagnostics.push(json);
                    }
                }
                DiagnosticsFormat::Stdrpt => {
                    emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output)
                }
            }
        }

        files.push(LintFileResult {
            path: path_display,
            diagrams: acc.diagrams,
            failed_diagrams: acc.failed_diagrams,
            warnings: acc.warnings,
            errors: acc.errors,
            passed: acc.errors == 0,
        });
    }

    let summary = LintSummary {
        total_files: files.len(),
        passed_files: files.iter().filter(|f| f.passed).count(),
        failed_files: files.iter().filter(|f| !f.passed).count(),
        total_diagrams: files.iter().map(|f| f.diagrams).sum(),
        passed_diagrams: files
            .iter()
            .map(|f| f.diagrams.saturating_sub(f.failed_diagrams))
            .sum(),
        failed_diagrams: files.iter().map(|f| f.failed_diagrams).sum(),
        warning_count: files.iter().map(|f| f.warnings).sum(),
        error_count: files.iter().map(|f| f.errors).sum(),
    };

    emit_lint_report(cli.lint_report, &summary, &files)?;
    if diagnostics_output.format == DiagnosticsFormat::Json && !lint_json_diagnostics.is_empty() {
        eprintln!(
            "{}",
            diagnostics_json_payload_precomputed(lint_json_diagnostics)
        );
    }

    if summary.failed_files > 0 {
        return Err((EXIT_VALIDATION, String::new()));
    }

    Ok(())
}

fn collect_lint_inputs(
    lint_input: &[PathBuf],
    lint_glob: &[String],
) -> Result<Vec<PathBuf>, (u8, String)> {
    let mut ordered = BTreeSet::new();

    for path in lint_input {
        ordered.insert(path.clone());
    }

    for pattern in lint_glob {
        let matches = glob(pattern).map_err(|e| {
            (
                EXIT_VALIDATION,
                format!("invalid lint glob '{pattern}': {e}"),
            )
        })?;
        for candidate in matches {
            let candidate = candidate.map_err(|e| {
                (
                    EXIT_IO,
                    format!("failed to expand lint glob '{pattern}': {e}"),
                )
            })?;
            ordered.insert(candidate);
        }
    }

    Ok(ordered.into_iter().collect())
}

fn emit_lint_diagnostic(
    path: &Path,
    source: &str,
    d: Diagnostic,
    output: DiagnosticOutput,
    lint_json_diagnostics: &mut Vec<DiagnosticJson>,
) {
    match output.format {
        DiagnosticsFormat::Human => {
            eprintln!("--> {}", path.display());
            eprintln!(
                "{}",
                render_human_diagnostic(&d, source, output.color_enabled)
            );
        }
        DiagnosticsFormat::Json => {
            let mut json = d.to_json_with_source(source);
            json.file = Some(path.display().to_string());
            lint_json_diagnostics.push(json);
        }
        DiagnosticsFormat::Stdrpt => eprintln!("{}", diagnostic_stdrpt(&d, source)),
    }
}

fn emit_lint_report(
    fmt: LintReportFormat,
    summary: &LintSummary,
    files: &[LintFileResult],
) -> Result<(), (u8, String)> {
    match fmt {
        LintReportFormat::Human => {
            println!(
                "lint summary: files={} passed={} failed={} diagrams={} passed_diagrams={} failed_diagrams={} warnings={} errors={}",
                summary.total_files,
                summary.passed_files,
                summary.failed_files,
                summary.total_diagrams,
                summary.passed_diagrams,
                summary.failed_diagrams,
                summary.warning_count,
                summary.error_count
            );
            for file in files.iter().filter(|f| !f.passed) {
                println!(
                    " - FAIL {} (diagrams={}, failed_diagrams={}, warnings={}, errors={})",
                    file.path, file.diagrams, file.failed_diagrams, file.warnings, file.errors
                );
            }
            Ok(())
        }
        LintReportFormat::Json => {
            let payload = LintReportPayload {
                schema: LINT_REPORT_SCHEMA,
                schema_version: LINT_REPORT_SCHEMA_VERSION,
                summary: LintSummary {
                    total_files: summary.total_files,
                    passed_files: summary.passed_files,
                    failed_files: summary.failed_files,
                    total_diagrams: summary.total_diagrams,
                    passed_diagrams: summary.passed_diagrams,
                    failed_diagrams: summary.failed_diagrams,
                    warning_count: summary.warning_count,
                    error_count: summary.error_count,
                },
                files: files
                    .iter()
                    .map(|f| LintFileResult {
                        path: f.path.clone(),
                        diagrams: f.diagrams,
                        failed_diagrams: f.failed_diagrams,
                        warnings: f.warnings,
                        errors: f.errors,
                        passed: f.passed,
                    })
                    .collect(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).map_err(|e| {
                    (
                        EXIT_INTERNAL,
                        format!("failed to serialize lint report: {e}"),
                    )
                })?
            );
            Ok(())
        }
    }
}
