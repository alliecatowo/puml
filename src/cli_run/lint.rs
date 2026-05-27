mod report;
use super::{
    diagnostics::{
        diagnostic_stdrpt, diagnostics_json_payload_precomputed, emit_warnings_for_model,
        map_diagnostic_span, render_human_diagnostic, should_color_human_diagnostics,
        DiagnosticOutput,
    },
    input::{frontend_hint_for_path, should_extract_markdown, split_diagrams},
    pipeline::parse_for_cli,
    EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION, SUPPORTED_MARKDOWN_FENCES,
};
use crate::cli::{
    Cli, CompatMode as CliCompatMode, DiagnosticsFormat, Dialect as CliDialect, LintArgs,
    LintFormat,
};
use glob::glob;
use puml::diagnostic::{normalized_warnings, offset_to_line_col};
use puml::{normalize_family, Diagnostic, DiagnosticJson};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize)]
struct LintSubcommandOutput {
    file: String,
    diagnostics: Vec<LintDiagnosticEntry>,
    summary: LintSubcommandSummary,
}

#[derive(Debug, Serialize)]
struct LintDiagnosticEntry {
    severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    message: String,
    span: LintSpan,
}

#[derive(Debug, Serialize)]
struct LintSpan {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

#[derive(Debug, Serialize)]
struct LintSubcommandSummary {
    errors: usize,
    warnings: usize,
}

#[derive(Debug, Clone)]
pub(super) struct LintSubcommandContext {
    pub(super) include_root: Option<PathBuf>,
    pub(super) dialect: CliDialect,
    pub(super) compat: CliCompatMode,
    pub(super) from_markdown: bool,
    pub(super) allow_url_includes: bool,
    pub(super) inject_vars: BTreeMap<String, String>,
}

pub(super) fn run_lint_subcommand(
    args: LintArgs,
    context: LintSubcommandContext,
) -> Result<(), (u8, String)> {
    let path = &args.file;
    let is_stdin = path == std::path::Path::new("-");

    let (file_label, raw, input_path) = if is_stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
        ("<stdin>".to_string(), buf, None)
    } else {
        let content = fs::read_to_string(path)
            .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", path.display())))?;
        (path.display().to_string(), content, Some(path.as_path()))
    };

    let include_root = context
        .include_root
        .clone()
        .or_else(|| input_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
    let from_markdown = should_extract_markdown(context.from_markdown, input_path);
    let markdown_name_prefix = input_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string());
    let file_frontend_hint = frontend_hint_for_path(input_path);

    // Collect all diagnostics (both parse errors and normalisation warnings).
    let mut diag_entries: Vec<LintDiagnosticEntry> = Vec::new();
    let mut error_count: usize = 0;
    let mut warning_count: usize = 0;

    let diagrams = match split_diagrams(
        &raw,
        from_markdown,
        markdown_name_prefix.as_deref(),
        file_frontend_hint,
    ) {
        Ok(diagrams) => diagrams,
        Err(d) => {
            error_count += 1;
            diag_entries.push(diagnostic_to_entry(&d, &raw));
            Vec::new()
        }
    };
    if diagrams.is_empty() && error_count == 0 {
        error_count += 1;
        let message = if from_markdown {
            format!(
                "no supported markdown diagram fences found; expected one of: {SUPPORTED_MARKDOWN_FENCES}"
            )
        } else {
            "no diagram content provided".to_string()
        };
        diag_entries.push(diagnostic_to_entry(&Diagnostic::error(message), &raw));
    }

    for source in &diagrams {
        let parse_result = parse_for_cli(
            &source.source,
            include_root.clone(),
            context.dialect,
            context.compat,
            source.frontend_hint,
            context.allow_url_includes,
            context.inject_vars.clone(),
        );

        match parse_result {
            Err(d) => {
                error_count += 1;
                let d = map_diagnostic_span(d, source.source_span);
                diag_entries.push(diagnostic_to_entry(&d, &raw));
            }
            Ok(doc) => match puml::normalize_family(doc) {
                Err(d) => {
                    error_count += 1;
                    let d = map_diagnostic_span(d, source.source_span);
                    diag_entries.push(diagnostic_to_entry(&d, &raw));
                }
                Ok(model) => {
                    let warnings = normalized_warnings(&model);
                    for w in warnings {
                        warning_count += 1;
                        let warning = map_diagnostic_span(w.clone(), source.source_span);
                        diag_entries.push(diagnostic_to_entry(&warning, &raw));
                    }
                }
            },
        }
    }

    match args.format {
        LintFormat::Human => {
            // Emit errors always; suppress warnings in --quiet mode.
            for entry in &diag_entries {
                if args.quiet && entry.severity == "warning" {
                    continue;
                }
                let severity_label = if entry.severity == "error" {
                    "error"
                } else {
                    "warning"
                };
                let span_hint = if entry.span.start_line > 0 {
                    format!(
                        " [{}:{}:{}]",
                        file_label, entry.span.start_line, entry.span.start_col
                    )
                } else {
                    format!(" [{}]", file_label)
                };
                eprintln!("{severity_label}{span_hint}: {}", entry.message);
            }
            if !args.quiet {
                println!(
                    "{}: {} error(s), {} warning(s)",
                    file_label, error_count, warning_count
                );
            }
        }
        LintFormat::Json => {
            // In quiet mode, suppress warning-only entries from JSON output.
            let output_diags: Vec<&LintDiagnosticEntry> = if args.quiet {
                diag_entries
                    .iter()
                    .filter(|e| e.severity == "error")
                    .collect()
            } else {
                diag_entries.iter().collect()
            };
            let output = LintSubcommandOutput {
                file: file_label,
                diagnostics: output_diags
                    .into_iter()
                    .map(|e| LintDiagnosticEntry {
                        severity: e.severity,
                        code: e.code.clone(),
                        message: e.message.clone(),
                        span: LintSpan {
                            start_line: e.span.start_line,
                            start_col: e.span.start_col,
                            end_line: e.span.end_line,
                            end_col: e.span.end_col,
                        },
                    })
                    .collect(),
                summary: LintSubcommandSummary {
                    errors: error_count,
                    warnings: warning_count,
                },
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&output).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize lint output: {e}")
                ))?
            );
        }
    }

    if error_count > 0 {
        Err((EXIT_VALIDATION, String::new()))
    } else {
        Ok(())
    }
}

fn diagnostic_to_entry(d: &puml::Diagnostic, source: &str) -> LintDiagnosticEntry {
    let json = d.to_json_with_source(source);
    let (start_line, start_col, end_line, end_col) = match d.span {
        Some(span) => {
            let (sl, sc) = offset_to_line_col(source, span.start);
            let (el, ec) = offset_to_line_col(source, span.end);
            (sl, sc, el, ec)
        }
        None => (0, 0, 0, 0),
    };
    LintDiagnosticEntry {
        severity: match d.severity {
            puml::diagnostic::Severity::Error => "error",
            puml::diagnostic::Severity::Warning => "warning",
        },
        code: json.code,
        message: d.message.clone(),
        span: LintSpan {
            start_line,
            start_col,
            end_line,
            end_col,
        },
    }
}
pub(super) fn is_lint_mode_enabled(cli: &Cli) -> bool {
    !cli.lint_input.is_empty() || !cli.lint_glob.is_empty()
}

pub(super) fn run_lint_mode(cli: &Cli) -> Result<(), (u8, String)> {
    if !cli.check && !cli.check_syntax {
        return Err((
            EXIT_VALIDATION,
            "lint mode requires --check or --check-syntax".to_string(),
        ));
    }
    let diagnostics_output = DiagnosticOutput {
        format: cli.diagnostics,
        color_enabled: should_color_human_diagnostics(cli.color),
    };

    let inject_vars: BTreeMap<String, String> = cli.defines.iter().cloned().collect();
    let lint_paths = collect_lint_inputs(&cli.lint_input, &cli.lint_glob, cli.pattern.as_deref())?;
    if lint_paths.is_empty() {
        return Err((
            EXIT_VALIDATION,
            "lint mode resolved no input files".to_string(),
        ));
    }
    if cli.verbose {
        eprintln!(
            "[verbose] linting {} file(s) with {} worker hint",
            lint_paths.len(),
            pluralize_threads(cli.threads)
        );
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

    report::emit_lint_report(cli.lint_report, &summary, &files)?;
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
    pattern: Option<&str>,
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

    let paths = ordered.into_iter().collect::<Vec<_>>();
    let Some(pattern) = pattern else {
        return Ok(paths);
    };
    let regex = Regex::new(pattern).map_err(|e| {
        (
            EXIT_VALIDATION,
            format!("invalid --pattern regex '{pattern}': {e}"),
        )
    })?;
    Ok(paths
        .into_iter()
        .filter(|path| regex.is_match(&path.to_string_lossy()))
        .collect())
}

fn pluralize_threads(threads: usize) -> String {
    if threads == 1 {
        "1 thread".to_string()
    } else {
        format!("{threads} threads")
    }
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
