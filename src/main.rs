mod cli;

use clap::{CommandFactory, Parser};
use cli::{
    Cli, CompatMode as CliCompatMode, DeterminismMode as CliDeterminismMode, DiagnosticsFormat,
    Dialect as CliDialect, DumpKind, LintReportFormat, OutputFormat,
};
use glob::glob;
use puml::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl,
    ParticipantRole as AstParticipantRole, Statement, StatementKind,
};
use puml::layout;
use puml::model::{
    Participant, ParticipantRole as ModelParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, TimelineDocument, VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use puml::scene::LayoutOptions;
use puml::source::Span;
use puml::{
    extract_markdown_diagrams, normalize_family, render, CompatMode, DeterminismMode, Diagnostic,
    DiagnosticJson, DiagramInput, FrontendSelection, NormalizedDocument, ParsePipelineOptions,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

static WARNING_COUNT: AtomicUsize = AtomicUsize::new(0);
static QUIET: AtomicUsize = AtomicUsize::new(0);
static VERBOSE: AtomicUsize = AtomicUsize::new(0);

fn record_warning() {
    WARNING_COUNT.fetch_add(1, Ordering::Relaxed);
}

fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed) != 0
}

fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed) != 0
}

fn warn_eprintln(msg: &str) {
    if !is_quiet() {
        eprintln!("{msg}");
    }
}

const EXIT_OK: u8 = 0;
const EXIT_VALIDATION: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INTERNAL: u8 = 3;

#[derive(Debug, Serialize)]
struct MultiSvgOut {
    name: String,
    svg: String,
}

#[derive(Debug, Serialize)]
struct SceneDump {
    size: SceneSize,
    lanes: Vec<SceneLane>,
    rows: Vec<SceneRow>,
}

#[derive(Debug, Serialize)]
struct SceneSize {
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
struct SceneLane {
    id: String,
    display: String,
    role: String,
    x: i32,
}

#[derive(Debug, Serialize)]
struct SceneRow {
    y: i32,
    event: Value,
}

#[derive(Debug, Clone)]
struct InputDiagram {
    source: String,
    source_span: Option<Span>,
    frontend_hint: Option<FrontendSelection>,
    output_name_hint: Option<String>,
}

#[derive(Debug, Clone)]
struct RenderedOutput {
    name_hint: Option<String>,
    svg: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticsPayload {
    schema: &'static str,
    schema_version: u32,
    diagnostics: Vec<DiagnosticJson>,
}

const DIAGNOSTICS_SCHEMA: &str = "puml.diagnostics";
const DIAGNOSTICS_SCHEMA_VERSION: u32 = 1;
const SUPPORTED_MARKDOWN_FENCES: &str =
    "puml, pumlx, picouml, plantuml, uml, puml-sequence, uml-sequence, mermaid";
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

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = if err.use_stderr() {
                EXIT_VALIDATION
            } else {
                EXIT_OK
            };
            let _ = err.print();
            return ExitCode::from(code);
        }
    };

    QUIET.store(cli.quiet as usize, Ordering::Relaxed);
    VERBOSE.store(cli.verbose as usize, Ordering::Relaxed);
    let track_duration = cli.duration;
    let fail_on_warn = cli.fail_on_warn;
    let started = Instant::now();

    let result = run(cli);

    if track_duration {
        let elapsed = started.elapsed();
        eprintln!("elapsed: {:.3}ms", elapsed.as_secs_f64() * 1_000.0);
    }

    match result {
        Ok(()) => {
            if fail_on_warn && WARNING_COUNT.load(Ordering::Relaxed) > 0 {
                eprintln!(
                    "[E_WARNINGS_PRESENT] --fail-on-warn: {} warning(s) emitted",
                    WARNING_COUNT.load(Ordering::Relaxed)
                );
                return ExitCode::from(EXIT_VALIDATION);
            }
            ExitCode::from(EXIT_OK)
        }
        Err((code, msg)) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            ExitCode::from(code)
        }
    }
}

fn run(cli: Cli) -> Result<(), (u8, String)> {
    if !cli.charset.is_empty() && !cli.charset.eq_ignore_ascii_case("UTF-8") && !cli.charset.eq_ignore_ascii_case("utf8") {
        return Err((
            EXIT_VALIDATION,
            format!(
                "[E_CHARSET_UNSUPPORTED] only UTF-8 is supported (got `{}`)",
                cli.charset
            ),
        ));
    }

    if matches!(cli.format, OutputFormat::Png) {
        return Err((
            EXIT_VALIDATION,
            "[E_FORMAT_PNG_UNSUPPORTED] only SVG output is supported; rerun with --format svg"
                .to_string(),
        ));
    }

    if cli.dump_capabilities {
        println!(
            "{}",
            serde_json::to_string_pretty(&lsp_capabilities_manifest()).map_err(|e| (
                EXIT_INTERNAL,
                format!("failed to serialize capability manifest: {e}")
            ))?
        );
        return Ok(());
    }

    if let Some(path) = &cli.check_fixture {
        let src = fs::read_to_string(path).map_err(|e| {
            (
                EXIT_IO,
                format!("failed to read fixture '{}': {e}", path.display()),
            )
        })?;
        let include_root = path.parent().map(|p| p.to_path_buf());
        let doc = parse_for_cli(
            &src,
            include_root,
            cli.dialect,
            cli.compat,
            cli.determinism,
            None,
        )
        .map_err(|d| diag_err_with_source(&src, d, cli.diagnostics))?;
        let model =
            normalize_family(doc).map_err(|d| diag_err_with_source(&src, d, cli.diagnostics))?;
        emit_warnings_for_model(&model, &src, None, cli.diagnostics);
        return Ok(());
    }

    if is_lint_mode_enabled(&cli) {
        return run_lint_mode(&cli);
    }

    let (_input_name, raw, input_path) = read_input(cli.input.as_deref())?;
    let include_root = cli
        .include_root
        .clone()
        .or_else(|| input_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
    let from_markdown = should_extract_markdown(cli.from_markdown, input_path);
    let markdown_name_prefix = input_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string());
    let diagrams = split_diagrams(&raw, from_markdown, markdown_name_prefix.as_deref())
        .map_err(|d| diag_err_with_source(&raw, d, cli.diagnostics))?;

    if diagrams.is_empty() {
        if from_markdown {
            return Err((
                EXIT_VALIDATION,
                format!(
                    "no supported markdown diagram fences found; expected one of: {SUPPORTED_MARKDOWN_FENCES}"
                ),
            ));
        }
        return Err((EXIT_VALIDATION, "no diagram content provided".to_string()));
    }

    if input_path.is_none() && diagrams.len() > 1 && !cli.multi {
        return Err((
            EXIT_VALIDATION,
            "multiple diagrams detected; rerun with --multi".to_string(),
        ));
    }

    if cli.check {
        for source in &diagrams {
            let parse_start = Instant::now();
            let doc = parse_for_cli(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                cli.determinism,
                source.frontend_hint,
            )
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
            if is_verbose() {
                warn_eprintln(&format!(
                    "[verbose] parse: {:.3}ms",
                    parse_start.elapsed().as_secs_f64() * 1_000.0
                ));
            }
            let normalize_start = Instant::now();
            let model = normalize_family(doc)
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
            if is_verbose() {
                warn_eprintln(&format!(
                    "[verbose] normalize: {:.3}ms",
                    normalize_start.elapsed().as_secs_f64() * 1_000.0
                ));
            }
            emit_warnings_for_model(&model, &raw, source.source_span, cli.diagnostics);
        }
        return Ok(());
    }

    if let Some(dump_kind) = cli.dump {
        let values = diagrams
            .iter()
            .map(|source| match dump_kind {
                DumpKind::Ast => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    Ok(ast_to_json(&doc))
                }
                DumpKind::Model => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, cli.diagnostics)
                    })?;
                    emit_warnings_for_model(&model, &raw, source.source_span, cli.diagnostics);
                    Ok(normalized_model_to_json(&model))
                }
                DumpKind::Scene => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, cli.diagnostics)
                    })?;
                    emit_warnings_for_model(&model, &raw, source.source_span, cli.diagnostics);
                    Ok(normalized_scene_to_json(&model))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        if values.len() == 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&values[0]).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize dump output: {e}")
                ))?
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&values).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize dump output: {e}")
                ))?
            );
        }
        return Ok(());
    }

    let outputs = diagrams.iter().try_fold(Vec::new(), |mut all, source| {
        let parse_start = Instant::now();
        let doc = parse_for_cli(
            &source.source,
            include_root.clone(),
            cli.dialect,
            cli.compat,
            cli.determinism,
            source.frontend_hint,
        )
        .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
        if is_verbose() {
            warn_eprintln(&format!(
                "[verbose] parse: {:.3}ms",
                parse_start.elapsed().as_secs_f64() * 1_000.0
            ));
        }
        let normalize_start = Instant::now();
        let model = normalize_family(doc)
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
        if is_verbose() {
            warn_eprintln(&format!(
                "[verbose] normalize: {:.3}ms",
                normalize_start.elapsed().as_secs_f64() * 1_000.0
            ));
        }
        emit_warnings_for_model(&model, &raw, source.source_span, cli.diagnostics);
        let render_start = Instant::now();
        let pages = render_pages_from_model(&model);
        if is_verbose() {
            warn_eprintln(&format!(
                "[verbose] render: {:.3}ms",
                render_start.elapsed().as_secs_f64() * 1_000.0
            ));
        }
        let page_count = pages.len();
        for (page_idx, svg) in pages.into_iter().enumerate() {
            let name_hint = source.output_name_hint.as_ref().map(|base| {
                if page_count == 1 {
                    format!("{base}.svg")
                } else {
                    format!("{base}-{}.svg", page_idx + 1)
                }
            });
            all.push(RenderedOutput { name_hint, svg });
        }
        Ok::<_, (u8, String)>(all)
    })?;

    if input_path.is_none() && outputs.len() > 1 && !cli.multi {
        return Err((
            EXIT_VALIDATION,
            "multiple pages detected; rerun with --multi".to_string(),
        ));
    }

    if input_path.is_none() && outputs.len() > 1 {
        let payload = outputs
            .iter()
            .enumerate()
            .map(|(idx, out)| MultiSvgOut {
                name: out
                    .name_hint
                    .clone()
                    .unwrap_or_else(|| format!("diagram-{}.svg", idx + 1)),
                svg: out.svg.clone(),
            })
            .collect::<Vec<_>>();

        let json = serde_json::to_string_pretty(&payload).map_err(|e| {
            (
                EXIT_INTERNAL,
                format!("failed to serialize multi output: {e}"),
            )
        })?;
        println!("{json}");
        return Ok(());
    }

    if let Some(path) = cli.output {
        let svgs = outputs
            .iter()
            .map(|out| out.svg.clone())
            .collect::<Vec<_>>();
        write_output_files(&path, &svgs)?;
        return Ok(());
    }

    if let Some(input) = input_path {
        if from_markdown {
            write_markdown_output_files(input, &outputs)?;
        } else {
            let default_base = default_output_base(input)?;
            let svgs = outputs
                .iter()
                .map(|out| out.svg.clone())
                .collect::<Vec<_>>();
            write_output_files(&default_base, &svgs)?;
        }
        return Ok(());
    }

    if outputs.len() == 1 {
        println!("{}", outputs[0].svg);
        return Ok(());
    }

    Err((EXIT_INTERNAL, "unexpected stdin output mode".to_string()))
}

fn is_lint_mode_enabled(cli: &Cli) -> bool {
    !cli.lint_input.is_empty() || !cli.lint_glob.is_empty()
}

fn run_lint_mode(cli: &Cli) -> Result<(), (u8, String)> {
    if !cli.check {
        return Err((EXIT_VALIDATION, "lint mode requires --check".to_string()));
    }

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

        let diagrams = match split_diagrams(&raw, from_markdown, markdown_name_prefix.as_deref()) {
            Ok(diagrams) => diagrams,
            Err(d) => {
                acc.errors += 1;
                emit_lint_diagnostic(&path, &raw, d, cli.diagnostics, &mut lint_json_diagnostics);
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
                cli.diagnostics,
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
            ) {
                Ok(doc) => doc,
                Err(d) => {
                    acc.errors += 1;
                    acc.failed_diagrams += 1;
                    emit_lint_diagnostic(
                        &path,
                        &raw,
                        map_diagnostic_span(d, source.source_span),
                        cli.diagnostics,
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
                        cli.diagnostics,
                        &mut lint_json_diagnostics,
                    );
                    continue;
                }
            };

            acc.warnings += normalized_warnings(&model).len();
            match cli.diagnostics {
                DiagnosticsFormat::Human => {
                    emit_warnings_for_model(&model, &raw, source.source_span, cli.diagnostics)
                }
                DiagnosticsFormat::Json => {
                    for warning in normalized_warnings(&model) {
                        let warning = map_diagnostic_span(warning.clone(), source.source_span);
                        let mut json = warning.to_json_with_source(&raw);
                        json.file = Some(path_display.clone());
                        lint_json_diagnostics.push(json);
                    }
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
    if cli.diagnostics == DiagnosticsFormat::Json && !lint_json_diagnostics.is_empty() {
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
    fmt: DiagnosticsFormat,
    lint_json_diagnostics: &mut Vec<DiagnosticJson>,
) {
    match fmt {
        DiagnosticsFormat::Human => {
            eprintln!("--> {}", path.display());
            eprintln!("{}", d.render_with_source(source));
        }
        DiagnosticsFormat::Json => {
            let mut json = d.to_json_with_source(source);
            json.file = Some(path.display().to_string());
            lint_json_diagnostics.push(json);
        }
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

fn lsp_capabilities_manifest() -> Value {
    json!({
      "server": "puml-lsp",
      "protocol": "3.17",
      "languageId": "puml",
      "extensions": [".puml", ".plantuml", ".iuml", ".pu"],
      "lifecycle": ["initialize", "initialized", "shutdown", "exit"],
      "textSync": ["didOpen", "didChange", "didSave", "didClose"],
      "languageFeatures": [
        "diagnostics", "completion", "hover", "definition", "references", "rename",
        "documentSymbols", "workspaceSymbols", "semanticTokens", "formatting", "codeActions",
        "foldingRanges", "selectionRanges", "documentLinks", "documentColor"
      ],
      "customRequests": [
        "puml.applyFormat", "puml.renderSvg"
      ]
    })
}

fn diag_err_with_source(source: &str, d: Diagnostic, fmt: DiagnosticsFormat) -> (u8, String) {
    match fmt {
        DiagnosticsFormat::Human => (EXIT_VALIDATION, d.render_with_source(source)),
        DiagnosticsFormat::Json => (EXIT_VALIDATION, diagnostics_json_payload(vec![d], source)),
    }
}

fn diag_err_mapped(
    raw_source: &str,
    mapping: Option<Span>,
    d: Diagnostic,
    fmt: DiagnosticsFormat,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source(raw_source, mapped, fmt)
}

fn emit_warnings_for_model(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<Span>,
    fmt: DiagnosticsFormat,
) {
    for warning in normalized_warnings(model) {
        record_warning();
        let warning = map_diagnostic_span(warning.clone(), mapping);
        if is_quiet() {
            continue;
        }
        match fmt {
            DiagnosticsFormat::Human => eprintln!("{}", warning.render_with_source(source)),
            DiagnosticsFormat::Json => {
                eprintln!("{}", diagnostics_json_payload(vec![warning], source));
            }
        }
    }
}

fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
        NormalizedDocument::Timeline(timeline) => &timeline.warnings,
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
    }
}

fn render_pages_from_model(model: &NormalizedDocument) -> Vec<String> {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            let scenes = layout::layout_pages(sequence, LayoutOptions::default());
            scenes.iter().map(render::render_svg).collect::<Vec<_>>()
        }
        NormalizedDocument::Family(family) => vec![match family.kind {
            DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase | DiagramKind::Salt => {
                render::render_class_svg(family)
            }
            DiagramKind::MindMap | DiagramKind::Wbs => render::render_family_tree_svg(family),
            DiagramKind::Component => render::render_component_svg(family),
            DiagramKind::Deployment => render::render_deployment_svg(family),
            DiagramKind::State => render::render_state_svg(family),
            DiagramKind::Activity => render::render_activity_svg(family),
            DiagramKind::Timing => render::render_timing_svg(family),
            _ => render::render_family_stub_svg(family),
        }],
        NormalizedDocument::Timeline(timeline) => vec![render::render_timeline_svg(timeline)],
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

fn parse_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
) -> Result<Document, Diagnostic> {
    let include_root = include_root.or_else(|| match cli_compat {
        CliCompatMode::Strict => None,
        CliCompatMode::Extended => std::env::current_dir().ok(),
    });
    let options = ParsePipelineOptions {
        frontend: map_frontend(cli_dialect, frontend_hint),
        compat: map_compat(cli_compat),
        determinism: map_determinism(cli_determinism),
        include_root,
    };
    puml::parse_with_pipeline_options(source, &options)
}

fn map_frontend(
    dialect: CliDialect,
    frontend_hint: Option<FrontendSelection>,
) -> FrontendSelection {
    if matches!(dialect, CliDialect::Auto) {
        if let Some(hint) = frontend_hint {
            return hint;
        }
    }

    match dialect {
        CliDialect::Auto => FrontendSelection::Auto,
        CliDialect::Plantuml => FrontendSelection::Plantuml,
        CliDialect::Mermaid => FrontendSelection::Mermaid,
        CliDialect::Picouml => FrontendSelection::Picouml,
    }
}

fn map_compat(mode: CliCompatMode) -> CompatMode {
    match mode {
        CliCompatMode::Strict => CompatMode::Strict,
        CliCompatMode::Extended => CompatMode::Extended,
    }
}

fn map_determinism(mode: CliDeterminismMode) -> DeterminismMode {
    match mode {
        CliDeterminismMode::Strict => DeterminismMode::Strict,
        CliDeterminismMode::Full => DeterminismMode::Full,
    }
}

fn read_input(path: Option<&Path>) -> Result<(String, String, Option<&Path>), (u8, String)> {
    match path {
        Some(p) if p != Path::new("-") => {
            let raw = fs::read_to_string(p)
                .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", p.display())))?;
            Ok((p.display().to_string(), raw, Some(p)))
        }
        _ => {
            if io::stdin().is_terminal() {
                let mut cmd = Cli::command();
                cmd.print_help().ok();
                println!();
                return Err((
                    EXIT_VALIDATION,
                    "no input provided on stdin (TTY detected); supply a file path or pipe input"
                        .to_string(),
                ));
            }
            let mut raw = String::new();
            io::stdin()
                .read_to_string(&mut raw)
                .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
            Ok(("stdin".to_string(), raw, None))
        }
    }
}

fn should_extract_markdown(from_markdown_flag: bool, input_path: Option<&Path>) -> bool {
    if from_markdown_flag {
        return true;
    }

    input_path
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

fn split_diagrams(
    raw: &str,
    from_markdown: bool,
    markdown_name_prefix: Option<&str>,
) -> Result<Vec<InputDiagram>, Diagnostic> {
    if from_markdown {
        let diagrams = extract_markdown_diagrams(raw)
            .into_iter()
            .enumerate()
            .map(
                |(
                    idx,
                    DiagramInput {
                        source,
                        span_in_input,
                        fence_frontend,
                    },
                )| InputDiagram {
                    source,
                    source_span: Some(span_in_input),
                    frontend_hint: Some(fence_frontend),
                    output_name_hint: Some(match markdown_name_prefix {
                        Some(prefix) => format!("{prefix}_snippet_{}", idx + 1),
                        None => format!("snippet-{}", idx + 1),
                    }),
                },
            )
            .collect::<Vec<_>>();
        return Ok(diagrams);
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut blocks = Vec::new();

    let has_startuml_marker = raw.lines().any(|line| {
        let marker = strip_inline_plantuml_comment(line).trim();
        matches_uml_marker(marker, "@startuml")
    });
    if has_startuml_marker {
        let mut current = Vec::new();
        let mut in_block = false;
        let mut block_start_line = 0usize;
        for (line_idx, line) in raw.lines().enumerate() {
            let marker = strip_inline_plantuml_comment(line).trim();
            if matches_uml_marker(marker, "@startuml") {
                if in_block {
                    return Err(Diagnostic::error(format!(
                        "unmatched @startuml/@enduml boundary: found @startuml at line {} before closing previous block started at line {}",
                        line_idx + 1,
                        block_start_line
                    )));
                }
                in_block = true;
                block_start_line = line_idx + 1;
                current.clear();
            }
            if matches_uml_marker(marker, "@enduml") && !in_block {
                return Err(Diagnostic::error(format!(
                    "unmatched @startuml/@enduml boundary: found @enduml at line {} without a preceding @startuml",
                    line_idx + 1
                )));
            }
            if in_block {
                current.push(line);
            }
            if in_block && matches_uml_marker(marker, "@enduml") {
                blocks.push(InputDiagram {
                    source: current.join("\n").trim().to_string(),
                    source_span: None,
                    frontend_hint: None,
                    output_name_hint: None,
                });
                current.clear();
                in_block = false;
            }
        }
        if in_block {
            return Err(Diagnostic::error(format!(
                "unmatched @startuml/@enduml boundary: @startuml at line {} is missing a closing @enduml",
                block_start_line
            )));
        }
        if !blocks.is_empty() {
            return Ok(blocks);
        }
    }

    Ok(vec![InputDiagram {
        source: trimmed.to_string(),
        source_span: None,
        frontend_hint: None,
        output_name_hint: None,
    }])
}

fn strip_inline_plantuml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '\'' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

fn matches_uml_marker(line: &str, marker: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with(marker) {
        return false;
    }
    let rest = &line[marker.len()..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn map_diagnostic_span(mut d: Diagnostic, mapping: Option<Span>) -> Diagnostic {
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

fn diagnostics_json_payload_precomputed(diags: Vec<DiagnosticJson>) -> String {
    let payload = DiagnosticsPayload {
        schema: DIAGNOSTICS_SCHEMA,
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        diagnostics: diags,
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"schema\":\"puml.diagnostics\",\"schema_version\":1,\"diagnostics\":[{\"code\":null,\"severity\":\"error\",\"message\":\"failed to serialize diagnostics\",\"span\":null,\"line\":null,\"column\":null,\"snippet\":null,\"caret\":null}]}".to_string()
    })
}

fn default_output_base(input: &Path) -> Result<PathBuf, (u8, String)> {
    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output name from '{}': invalid stem",
                input.display()
            ),
        )
    })?;
    Ok(input.with_file_name(format!("{stem}.svg")))
}

fn write_markdown_output_files(
    input: &Path,
    outputs: &[RenderedOutput],
) -> Result<(), (u8, String)> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::with_capacity(outputs.len());
    for (idx, out) in outputs.iter().enumerate() {
        let name = out.name_hint.as_ref().ok_or_else(|| {
            (
                EXIT_INTERNAL,
                format!("missing markdown output name for diagram {}", idx + 1),
            )
        })?;
        let path = parent.join(name);
        files.push((path, out.svg.clone()));
    }
    write_files_transactionally(files)
}

fn write_output_files(base: &Path, svgs: &[String]) -> Result<(), (u8, String)> {
    if svgs.len() == 1 {
        return write_files_transactionally(vec![(base.to_path_buf(), svgs[0].clone())]);
    }

    let stem = base.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output stem from '{}': invalid stem",
                base.display()
            ),
        )
    })?;
    let ext = base
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("svg");
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::with_capacity(svgs.len());

    for (idx, svg) in svgs.iter().enumerate() {
        let path = parent.join(format!("{stem}-{}.{}", idx + 1, ext));
        files.push((path, svg.clone()));
    }

    write_files_transactionally(files)
}

#[derive(Debug)]
struct StagedWrite {
    target: PathBuf,
    staged: PathBuf,
    backup: Option<PathBuf>,
    published: bool,
}

fn write_files_transactionally(files: Vec<(PathBuf, String)>) -> Result<(), (u8, String)> {
    if files.is_empty() {
        return Ok(());
    }

    let pid = std::process::id();
    let mut staged_writes = Vec::with_capacity(files.len());

    for (idx, (target, contents)) in files.into_iter().enumerate() {
        if target.is_dir() {
            cleanup_staged_artifacts(&staged_writes);
            return Err((
                EXIT_IO,
                format!(
                    "failed to write '{}': target is a directory",
                    target.display()
                ),
            ));
        }
        let staged = staging_path_for(&target, "stage", pid, idx);
        fs::write(&staged, contents).map_err(|e| {
            cleanup_staged_artifacts(&staged_writes);
            (
                EXIT_IO,
                format!("failed to write '{}': {e}", target.display()),
            )
        })?;
        staged_writes.push(StagedWrite {
            target,
            staged,
            backup: None,
            published: false,
        });
    }

    let fail_after = transactional_write_fail_after();

    for idx in 0..staged_writes.len() {
        let target_display = staged_writes[idx].target.display().to_string();

        if staged_writes[idx].target.exists() {
            let backup = staging_path_for(&staged_writes[idx].target, "backup", pid, idx);
            if let Err(e) = fs::rename(&staged_writes[idx].target, &backup) {
                rollback_staged_writes(&mut staged_writes);
                return Err((
                    EXIT_IO,
                    format!("failed to prepare output '{target_display}': {e}"),
                ));
            }
            staged_writes[idx].backup = Some(backup);
        }

        if fail_after == Some(idx) {
            rollback_staged_writes(&mut staged_writes);
            return Err((
                EXIT_IO,
                format!("failed to write '{target_display}': simulated write failure"),
            ));
        }

        if let Err(e) = fs::rename(&staged_writes[idx].staged, &staged_writes[idx].target) {
            rollback_staged_writes(&mut staged_writes);
            return Err((EXIT_IO, format!("failed to write '{target_display}': {e}")));
        }

        staged_writes[idx].published = true;
    }

    for item in staged_writes {
        if let Some(backup) = item.backup {
            let _ = fs::remove_file(backup);
        }
    }

    Ok(())
}

fn staging_path_for(target: &Path, kind: &str, pid: u32, idx: usize) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("output");
    let base = format!(".{name}.puml.{kind}.{pid}.{idx}");
    for attempt in 0..32 {
        let candidate = parent.join(format!("{base}.{attempt}.tmp"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{base}.overflow.tmp"))
}

fn rollback_staged_writes(staged_writes: &mut [StagedWrite]) {
    for item in staged_writes.iter_mut().rev() {
        if item.published {
            let _ = fs::remove_file(&item.target);
            if let Some(backup) = item.backup.take() {
                let _ = fs::rename(&backup, &item.target);
            }
        } else {
            let _ = fs::remove_file(&item.staged);
            if let Some(backup) = item.backup.take() {
                let _ = fs::rename(&backup, &item.target);
            }
        }
    }
}

fn cleanup_staged_artifacts(staged_writes: &[StagedWrite]) {
    for item in staged_writes {
        let _ = fs::remove_file(&item.staged);
    }
}

fn transactional_write_fail_after() -> Option<usize> {
    std::env::var("PUML_FAIL_OUTPUT_AFTER")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
}

fn ast_to_json(doc: &Document) -> Value {
    json!({
        "kind": match doc.kind {
            DiagramKind::Sequence => "Sequence",
            DiagramKind::Class => "Class",
            DiagramKind::Object => "Object",
            DiagramKind::UseCase => "UseCase",
            DiagramKind::Salt => "Salt",
            DiagramKind::MindMap => "MindMap",
            DiagramKind::Wbs => "Wbs",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            DiagramKind::Component => "Component",
            DiagramKind::Deployment => "Deployment",
            DiagramKind::State => "State",
            DiagramKind::Activity => "Activity",
            DiagramKind::Timing => "Timing",
            DiagramKind::Json => "Json",
            DiagramKind::Yaml => "Yaml",
            DiagramKind::Nwdiag => "Nwdiag",
            DiagramKind::Archimate => "Archimate",
            DiagramKind::Regex => "Regex",
            DiagramKind::Ebnf => "Ebnf",
            DiagramKind::Math => "Math",
            DiagramKind::Sdl => "Sdl",
            DiagramKind::Ditaa => "Ditaa",
            DiagramKind::Chart => "Chart",            DiagramKind::Unknown => "Unknown",
        },
        "statements": doc.statements.iter().map(statement_to_json).collect::<Vec<_>>()
    })
}

fn statement_to_json(s: &Statement) -> Value {
    json!({
        "span": {"start": s.span.start, "end": s.span.end},
        "kind": statement_kind_to_json(&s.kind)
    })
}

fn statement_kind_to_json(kind: &StatementKind) -> Value {
    match kind {
        StatementKind::Participant(p) => json!({"Participant": participant_decl_to_json(p)}),
        StatementKind::Message(m) => json!({"Message": message_to_json(m)}),
        StatementKind::ClassDecl(v) => {
            json!({"ClassDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::ObjectDecl(v) => {
            json!({"ObjectDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::UseCaseDecl(v) => {
            json!({"UseCaseDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::FamilyRelation(v) => {
            json!({"FamilyRelation": {"from": v.from, "to": v.to, "arrow": v.arrow, "label": v.label}})
        }
        StatementKind::GanttTaskDecl { name } => json!({"GanttTaskDecl": {"name": name}}),
        StatementKind::GanttMilestoneDecl { name } => {
            json!({"GanttMilestoneDecl": {"name": name}})
        }
        StatementKind::GanttConstraint {
            subject,
            kind,
            target,
        } => {
            json!({"GanttConstraint": {"subject": subject, "kind": kind, "target": target}})
        }
        StatementKind::ChronologyHappensOn { subject, when } => {
            json!({"ChronologyHappensOn": {"subject": subject, "when": when}})
        }
        StatementKind::Note(n) => json!({"Note": note_to_json(n)}),
        StatementKind::Group(g) => json!({"Group": group_to_json(g)}),
        StatementKind::Title(v) => json!({"Title": v}),
        StatementKind::Header(v) => json!({"Header": v}),
        StatementKind::Footer(v) => json!({"Footer": v}),
        StatementKind::Caption(v) => json!({"Caption": v}),
        StatementKind::Legend(v) => json!({"Legend": v}),
        StatementKind::Theme(v) => json!({"Theme": v}),
        StatementKind::Pragma(v) => json!({"Pragma": v}),
        StatementKind::SkinParam { key, value } => {
            json!({"SkinParam": {"key": key, "value": value}})
        }
        StatementKind::Footbox(v) => json!({"Footbox": v}),
        StatementKind::Delay(v) => json!({"Delay": v}),
        StatementKind::Divider(v) => json!({"Divider": v}),
        StatementKind::Separator(v) => json!({"Separator": v}),
        StatementKind::Spacer => json!("Spacer"),
        StatementKind::NewPage(v) => json!({"NewPage": v}),
        StatementKind::IgnoreNewPage => json!("IgnoreNewPage"),
        StatementKind::Autonumber(v) => json!({"Autonumber": v}),
        StatementKind::Activate(v) => json!({"Activate": v}),
        StatementKind::Deactivate(v) => json!({"Deactivate": v}),
        StatementKind::Destroy(v) => json!({"Destroy": v}),
        StatementKind::Create(v) => json!({"Create": v}),
        StatementKind::Return(v) => json!({"Return": v}),
        StatementKind::Include(v) => json!({"Include": v}),
        StatementKind::Define { name, value } => json!({"Define": {"name": name, "value": value}}),
        StatementKind::Undef(v) => json!({"Undef": v}),
        StatementKind::RawBlockContent(v) => json!({"RawBlockContent": v}),
        StatementKind::RawBody(v) => json!({"RawBody": v}),
        StatementKind::Unknown(v) => json!({"Unknown": v}),
        StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
        } => json!({
            "ComponentDecl": {
                "kind": format!("{:?}", kind),
                "name": name,
                "alias": alias,
                "label": label,
            }
        }),
        StatementKind::StateDecl {
            name,
            alias,
            label,
        } => json!({
            "StateDecl": {"name": name, "alias": alias, "label": label}
        }),
        StatementKind::ActivityStep(step) => json!({
            "ActivityStep": {"kind": format!("{:?}", step.kind), "label": step.label}
        }),
        StatementKind::TimingDecl { kind, name, label } => json!({
            "TimingDecl": {"kind": format!("{:?}", kind), "name": name, "label": label}
        }),
        StatementKind::TimingEvent {
            time,
            signal,
            state,
            note,
        } => json!({
            "TimingEvent": {"time": time, "signal": signal, "state": state, "note": note}
        }),
    }
}

fn participant_decl_to_json(p: &ParticipantDecl) -> Value {
    json!({
        "role": ast_role_to_str(p.role),
        "name": p.name,
        "alias": p.alias,
        "display": p.display
    })
}

fn message_to_json(m: &Message) -> Value {
    let mut message = json!({"from": m.from, "to": m.to, "arrow": m.arrow, "label": m.label});
    if let Some(ep) = m.from_virtual {
        message["from_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    if let Some(ep) = m.to_virtual {
        message["to_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    message
}

fn note_to_json(n: &Note) -> Value {
    json!({"position": n.position, "target": n.target, "text": n.text})
}

fn group_to_json(g: &Group) -> Value {
    json!({"kind": g.kind, "label": g.label})
}

fn ast_role_to_str(role: AstParticipantRole) -> &'static str {
    match role {
        AstParticipantRole::Participant => "Participant",
        AstParticipantRole::Actor => "Actor",
        AstParticipantRole::Boundary => "Boundary",
        AstParticipantRole::Control => "Control",
        AstParticipantRole::Entity => "Entity",
        AstParticipantRole::Database => "Database",
        AstParticipantRole::Collections => "Collections",
        AstParticipantRole::Queue => "Queue",
    }
}

fn normalized_model_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => model_to_json(sequence),
        NormalizedDocument::Family(family) => family_model_to_json(family),
        NormalizedDocument::Timeline(timeline) => timeline_model_to_json(timeline),
        NormalizedDocument::Json(doc) => json!({
            "kind": "Json",
            "title": doc.title,
            "raw": doc.raw,
            "nodes": doc.nodes.iter().map(|n| json!({"depth": n.depth, "label": n.label})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Yaml(doc) => json!({
            "kind": "Yaml",
            "title": doc.title,
            "raw": doc.raw,
            "nodes": doc.nodes.iter().map(|n| json!({"depth": n.depth, "label": n.label})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Nwdiag(doc) => json!({
            "kind": "Nwdiag",
            "title": doc.title,
            "networks": doc.networks.iter().map(|n| json!({
                "name": n.name,
                "address": n.address,
                "nodes": n.nodes.iter().map(|nd| json!({"name": nd.name, "address": nd.address})).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Archimate(doc) => json!({
            "kind": "Archimate",
            "title": doc.title,
            "elements": doc.elements.iter().map(|e| json!({"name": e.name, "alias": e.alias, "layer": e.layer})).collect::<Vec<_>>(),
            "relations": doc.relations.iter().map(|r| json!({"from": r.from, "to": r.to, "kind": r.kind, "label": r.label})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Regex(doc) => json!({
            "kind": "Regex",
            "title": doc.title,
            "patterns": doc.patterns.iter().map(|p| json!({"source": p.source})).collect::<Vec<_>>(),
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
        NormalizedDocument::Ebnf(doc) => json!({
            "kind": "Ebnf",
            "title": doc.title,
            "rules": doc.rules.iter().map(|r| json!({"name": r.name, "body": r.body})).collect::<Vec<_>>(),
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
        NormalizedDocument::Math(doc) => json!({
            "kind": "Math",
            "title": doc.title,
            "body": doc.body,
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
        NormalizedDocument::Sdl(doc) => json!({
            "kind": "Sdl",
            "title": doc.title,
            "states": doc.states.iter().map(|s| json!({"name": s.name, "kind": match s.kind {
                puml::model::SdlStateKind::Start => "Start",
                puml::model::SdlStateKind::Stop => "Stop",
                puml::model::SdlStateKind::State => "State",
            }})).collect::<Vec<_>>(),
            "transitions": doc.transitions.iter().map(|t| json!({"from": t.from, "to": t.to, "signal": t.signal})).collect::<Vec<_>>(),
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
        NormalizedDocument::Ditaa(doc) => json!({
            "kind": "Ditaa",
            "title": doc.title,
            "body": doc.body,
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
        NormalizedDocument::Chart(doc) => json!({
            "kind": "Chart",
            "title": doc.title,
            "subtype": match doc.subtype {
                puml::model::ChartSubtype::Bar => "Bar",
                puml::model::ChartSubtype::Line => "Line",
                puml::model::ChartSubtype::Pie => "Pie",
            },
            "data": doc.data.iter().map(|p| json!({"label": p.label, "value": p.value})).collect::<Vec<_>>(),
            "warnings": doc.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        }),
    }
}

fn model_to_json(model: &SequenceDocument) -> Value {
    json!({
        "participants": model.participants.iter().map(model_participant_to_json).collect::<Vec<_>>(),
        "events": model.events.iter().map(model_event_to_json).collect::<Vec<_>>(),
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "skinparams": model.skinparams,
        "footbox_visible": model.footbox_visible,
        "style": {
            "arrow_color": model.style.arrow_color,
            "lifeline_border_color": model.style.lifeline_border_color,
            "participant_background_color": model.style.participant_background_color,
            "participant_border_color": model.style.participant_border_color,
            "note_background_color": model.style.note_background_color,
            "note_border_color": model.style.note_border_color,
            "group_background_color": model.style.group_background_color,
            "group_border_color": model.style.group_border_color
        }
    })
}

fn family_model_to_json(model: &puml::FamilyDocument) -> Value {
    json!({
        "kind": match model.kind {
            DiagramKind::Class => "Class",
            DiagramKind::Object => "Object",
            DiagramKind::UseCase => "UseCase",
            DiagramKind::Salt => "Salt",
            DiagramKind::MindMap => "MindMap",
            DiagramKind::Wbs => "Wbs",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            DiagramKind::Component => "Component",
            DiagramKind::Deployment => "Deployment",
            DiagramKind::State => "State",
            DiagramKind::Activity => "Activity",
            DiagramKind::Timing => "Timing",
            DiagramKind::Json => "Json",
            DiagramKind::Yaml => "Yaml",
            DiagramKind::Nwdiag => "Nwdiag",
            DiagramKind::Archimate => "Archimate",
            DiagramKind::Sequence => "Sequence",
            DiagramKind::Regex => "Regex",
            DiagramKind::Ebnf => "Ebnf",
            DiagramKind::Math => "Math",
            DiagramKind::Sdl => "Sdl",
            DiagramKind::Ditaa => "Ditaa",
            DiagramKind::Chart => "Chart",
            DiagramKind::Unknown => "Unknown",
        },
        "nodes": model
            .nodes
            .iter()
            .map(|n| {
                json!({
                    "kind": format!("{:?}", n.kind),
                    "name": n.name,
                    "alias": n.alias
                })
            })
            .collect::<Vec<_>>(),
        "relations": model
            .relations
            .iter()
            .map(|r| {
                json!({
                    "from": r.from,
                    "to": r.to,
                    "arrow": r.arrow,
                    "label": r.label
                })
            })
            .collect::<Vec<_>>(),
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn timeline_model_to_json(model: &TimelineDocument) -> Value {
    json!({
        "kind": match model.kind {
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            DiagramKind::Sequence => "Sequence",
            DiagramKind::Class => "Class",
            DiagramKind::Object => "Object",
            DiagramKind::UseCase => "UseCase",
            DiagramKind::Salt => "Salt",
            DiagramKind::MindMap => "MindMap",
            DiagramKind::Wbs => "Wbs",
            DiagramKind::Component => "Component",
            DiagramKind::Deployment => "Deployment",
            DiagramKind::State => "State",
            DiagramKind::Activity => "Activity",
            DiagramKind::Timing => "Timing",
            DiagramKind::Json => "Json",
            DiagramKind::Yaml => "Yaml",
            DiagramKind::Nwdiag => "Nwdiag",
            DiagramKind::Archimate => "Archimate",
            DiagramKind::Regex => "Regex",
            DiagramKind::Ebnf => "Ebnf",
            DiagramKind::Math => "Math",
            DiagramKind::Sdl => "Sdl",
            DiagramKind::Ditaa => "Ditaa",
            DiagramKind::Chart => "Chart",            DiagramKind::Unknown => "Unknown",
        },
        "tasks": model.tasks.iter().map(|t| json!({"name": t.name})).collect::<Vec<_>>(),
        "milestones": model.milestones.iter().map(|m| json!({"name": m.name})).collect::<Vec<_>>(),
        "constraints": model.constraints.iter().map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target})).collect::<Vec<_>>(),
        "chronology_events": model.chronology_events.iter().map(|e| json!({"subject": e.subject, "when": e.when})).collect::<Vec<_>>(),
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn model_participant_to_json(p: &Participant) -> Value {
    json!({
        "id": p.id,
        "display": p.display,
        "role": model_role_to_str(p.role),
        "explicit": p.explicit
    })
}

fn model_role_to_str(role: ModelParticipantRole) -> &'static str {
    match role {
        ModelParticipantRole::Participant => "Participant",
        ModelParticipantRole::Actor => "Actor",
        ModelParticipantRole::Boundary => "Boundary",
        ModelParticipantRole::Control => "Control",
        ModelParticipantRole::Entity => "Entity",
        ModelParticipantRole::Database => "Database",
        ModelParticipantRole::Collections => "Collections",
        ModelParticipantRole::Queue => "Queue",
    }
}

fn model_event_to_json(e: &SequenceEvent) -> Value {
    json!({
        "span": {"start": e.span.start, "end": e.span.end},
        "kind": model_event_kind_to_json(&e.kind)
    })
}

fn model_event_kind_to_json(kind: &SequenceEventKind) -> Value {
    match kind {
        SequenceEventKind::Message {
            from,
            to,
            arrow,
            label,
            from_virtual,
            to_virtual,
        } => {
            let mut message = json!({"from": from, "to": to, "arrow": arrow, "label": label});
            if let Some(ep) = from_virtual {
                message["from_virtual"] = virtual_endpoint_to_json(*ep);
            }
            if let Some(ep) = to_virtual {
                message["to_virtual"] = virtual_endpoint_to_json(*ep);
            }
            json!({"Message": message})
        }
        SequenceEventKind::Note {
            position,
            target,
            text,
        } => {
            json!({"Note": {"position": position, "target": target, "text": text}})
        }
        SequenceEventKind::GroupStart { kind, label } => {
            json!({"GroupStart": {"kind": kind, "label": label}})
        }
        SequenceEventKind::GroupEnd => json!("GroupEnd"),
        SequenceEventKind::Delay(v) => json!({"Delay": v}),
        SequenceEventKind::Divider(v) => json!({"Divider": v}),
        SequenceEventKind::Separator(v) => json!({"Separator": v}),
        SequenceEventKind::Spacer => json!("Spacer"),
        SequenceEventKind::NewPage(v) => json!({"NewPage": v}),
        SequenceEventKind::Autonumber(v) => json!({"Autonumber": v}),
        SequenceEventKind::Activate(v) => json!({"Activate": v}),
        SequenceEventKind::Deactivate(v) => json!({"Deactivate": v}),
        SequenceEventKind::Destroy(v) => json!({"Destroy": v}),
        SequenceEventKind::Create(v) => json!({"Create": v}),
        SequenceEventKind::Return { label, from, to } => {
            json!({"Return": {"label": label, "from": from, "to": to}})
        }
        SequenceEventKind::IncludePlaceholder(v) => json!({"IncludePlaceholder": v}),
        SequenceEventKind::DefinePlaceholder { name, value } => {
            json!({"DefinePlaceholder": {"name": name, "value": value}})
        }
        SequenceEventKind::UndefPlaceholder(v) => json!({"UndefPlaceholder": v}),
    }
}

fn virtual_endpoint_to_json(ep: VirtualEndpoint) -> Value {
    json!({
        "side": match ep.side {
            VirtualEndpointSide::Left => "left",
            VirtualEndpointSide::Right => "right",
        },
        "kind": match ep.kind {
            VirtualEndpointKind::Plain => "plain",
            VirtualEndpointKind::Circle => "circle",
            VirtualEndpointKind::Cross => "cross",
            VirtualEndpointKind::Filled => "filled",
        }
    })
}

fn scene_to_json(model: &SequenceDocument) -> Value {
    let lane_spacing = 140;
    let lane_start = 100;
    let row_spacing = 40;
    let row_start = 120;
    let width = 200 + (model.participants.len() as i32 * lane_spacing);
    let height = 120 + (model.events.len() as i32 * row_spacing);

    let lanes = model
        .participants
        .iter()
        .enumerate()
        .map(|(idx, p)| SceneLane {
            id: p.id.clone(),
            display: p.display.clone(),
            role: model_role_to_str(p.role).to_string(),
            x: lane_start + (idx as i32 * lane_spacing),
        })
        .collect::<Vec<_>>();

    let rows = model
        .events
        .iter()
        .enumerate()
        .map(|(idx, e)| SceneRow {
            y: row_start + (idx as i32 * row_spacing),
            event: model_event_to_json(e),
        })
        .collect::<Vec<_>>();

    let scene = SceneDump {
        size: SceneSize { width, height },
        lanes,
        rows,
    };
    serde_json::to_value(scene).unwrap_or_else(|_| json!({"error": "scene serialization failed"}))
}

fn normalized_scene_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => scene_to_json(sequence),
        NormalizedDocument::Family(family) => {
            let svg = render::render_family_stub_svg(family);
            json!({
                "kind": "FamilyStub",
                "family": match family.kind {
                    DiagramKind::Class => "Class",
                    DiagramKind::Object => "Object",
                    DiagramKind::UseCase => "UseCase",
                    DiagramKind::Salt => "Salt",
                    DiagramKind::MindMap => "MindMap",
                    DiagramKind::Wbs => "Wbs",
                    DiagramKind::Gantt => "Gantt",
                    DiagramKind::Chronology => "Chronology",
                    DiagramKind::Component => "Component",
                    DiagramKind::Deployment => "Deployment",
                    DiagramKind::State => "State",
                    DiagramKind::Activity => "Activity",
                    DiagramKind::Timing => "Timing",
                    DiagramKind::Json => "Json",
                    DiagramKind::Yaml => "Yaml",
                    DiagramKind::Nwdiag => "Nwdiag",
                    DiagramKind::Archimate => "Archimate",
                    DiagramKind::Sequence => "Sequence",
                    DiagramKind::Regex => "Regex",
                    DiagramKind::Ebnf => "Ebnf",
                    DiagramKind::Math => "Math",
                    DiagramKind::Sdl => "Sdl",
                    DiagramKind::Ditaa => "Ditaa",
                    DiagramKind::Chart => "Chart",
                    DiagramKind::Unknown => "Unknown",
                },
                "nodes": family
                    .nodes
                    .iter()
                    .map(|n| {
                        json!({
                            "kind": format!("{:?}", n.kind),
                            "name": n.name,
                            "alias": n.alias
                        })
                    })
                    .collect::<Vec<_>>(),
                "relations": family
                    .relations
                    .iter()
                    .map(|r| {
                        json!({
                            "from": r.from,
                            "to": r.to,
                            "arrow": r.arrow,
                            "label": r.label
                        })
                    })
                    .collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::Timeline(timeline) => {
            json!({
                "kind": "TimelineBaseline",
                "family": match timeline.kind {
                    DiagramKind::Gantt => "Gantt",
                    DiagramKind::Chronology => "Chronology",
                    DiagramKind::Sequence => "Sequence",
                    DiagramKind::Class => "Class",
                    DiagramKind::Object => "Object",
                    DiagramKind::UseCase => "UseCase",
                    DiagramKind::Salt => "Salt",
                    DiagramKind::MindMap => "MindMap",
                    DiagramKind::Wbs => "Wbs",
                    DiagramKind::Component => "Component",
                    DiagramKind::Deployment => "Deployment",
                    DiagramKind::State => "State",
                    DiagramKind::Activity => "Activity",
                    DiagramKind::Timing => "Timing",
                    DiagramKind::Json => "Json",
                    DiagramKind::Yaml => "Yaml",
                    DiagramKind::Nwdiag => "Nwdiag",
                    DiagramKind::Archimate => "Archimate",
                    DiagramKind::Regex => "Regex",
                    DiagramKind::Ebnf => "Ebnf",
                    DiagramKind::Math => "Math",
                    DiagramKind::Sdl => "Sdl",
                    DiagramKind::Ditaa => "Ditaa",
                    DiagramKind::Chart => "Chart",
                    DiagramKind::Unknown => "Unknown",
                },
                "tasks": timeline.tasks.iter().map(|t| json!({"name": t.name})).collect::<Vec<_>>(),
                "milestones": timeline.milestones.iter().map(|m| json!({"name": m.name})).collect::<Vec<_>>(),
                "constraints": timeline.constraints.iter().map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target})).collect::<Vec<_>>(),
                "chronology_events": timeline.chronology_events.iter().map(|e| json!({"subject": e.subject, "when": e.when})).collect::<Vec<_>>(),
            })
        }
        NormalizedDocument::Json(doc) => {
            let svg = render::render_json_svg(doc);
            json!({
                "kind": "Json",
                "title": doc.title,
                "nodes": doc.nodes.iter().map(|n| json!({"depth": n.depth, "label": n.label})).collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::Yaml(doc) => {
            let svg = render::render_yaml_svg(doc);
            json!({
                "kind": "Yaml",
                "title": doc.title,
                "nodes": doc.nodes.iter().map(|n| json!({"depth": n.depth, "label": n.label})).collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::Nwdiag(doc) => {
            let svg = render::render_nwdiag_svg(doc);
            json!({
                "kind": "Nwdiag",
                "title": doc.title,
                "networks": doc.networks.iter().map(|n| json!({
                    "name": n.name,
                    "address": n.address,
                    "nodes": n.nodes.iter().map(|nd| json!({"name": nd.name, "address": nd.address})).collect::<Vec<_>>()
                })).collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::Archimate(doc) => {
            let svg = render::render_archimate_svg(doc);
            json!({
                "kind": "Archimate",
                "title": doc.title,
                "elements": doc.elements.iter().map(|e| json!({"name": e.name, "alias": e.alias, "layer": e.layer})).collect::<Vec<_>>(),
                "relations": doc.relations.iter().map(|r| json!({"from": r.from, "to": r.to, "kind": r.kind, "label": r.label})).collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::Regex(doc) => json!({
            "kind": "RegexScene",
            "svg_preview": render::render_regex_svg(doc),
            "patterns": doc.patterns.iter().map(|p| json!({"source": p.source})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Ebnf(doc) => json!({
            "kind": "EbnfScene",
            "svg_preview": render::render_ebnf_svg(doc),
            "rules": doc.rules.iter().map(|r| json!({"name": r.name, "body": r.body})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Math(doc) => json!({
            "kind": "MathScene",
            "svg_preview": render::render_math_svg(doc),
            "body": doc.body,
        }),
        NormalizedDocument::Sdl(doc) => json!({
            "kind": "SdlScene",
            "svg_preview": render::render_sdl_svg(doc),
            "states": doc.states.iter().map(|s| json!({"name": s.name})).collect::<Vec<_>>(),
            "transitions": doc.transitions.iter().map(|t| json!({"from": t.from, "to": t.to, "signal": t.signal})).collect::<Vec<_>>(),
        }),
        NormalizedDocument::Ditaa(doc) => json!({
            "kind": "DitaaScene",
            "svg_preview": render::render_ditaa_svg(doc),
            "body": doc.body,
        }),
        NormalizedDocument::Chart(doc) => json!({
            "kind": "ChartScene",
            "svg_preview": render::render_chart_svg(doc),
            "data": doc.data.iter().map(|p| json!({"label": p.label, "value": p.value})).collect::<Vec<_>>(),
        }),
    }
}
