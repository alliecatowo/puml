mod cli;
mod cli_count;
mod cli_env;
mod cli_hash;
mod cli_stats;
mod cli_watch;

use clap::{CommandFactory, FromArgMatches};
use cli::{
    Cli, ColorChoice as CliColorChoice, Command as CliCommand, CompatMode as CliCompatMode,
    DeterminismMode as CliDeterminismMode, DiagnosticsFormat, Dialect as CliDialect, DumpKind,
    FormatArgs, LintArgs, LintFormat, LintReportFormat, OutputFormat,
};
use glob::glob;
use image::ImageEncoder;
use puml::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl,
    ParticipantRole as AstParticipantRole, Statement, StatementKind,
};
use puml::model::{
    Participant, ParticipantRole as ModelParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, StateDocument, TimelineDocument, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
use puml::source::Span;
use puml::{
    extract_markdown_diagrams, extract_metadata, normalize_family,
    preprocess_with_pipeline_options, render, render_svg_pages_from_model, specialized, CompatMode,
    DeterminismMode, Diagnostic, DiagnosticJson, DiagramInput, FrontendSelection,
    NormalizedDocument, ParsePipelineOptions, TextOutputMode,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const EXIT_OK: u8 = 0;
const EXIT_VALIDATION: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INTERNAL: u8 = 3;

#[derive(Debug, Serialize)]
struct MultiSvgOut {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
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
    content: String,
}

#[derive(Debug, Clone)]
struct RenderedBinaryOutput {
    name_hint: Option<String>,
    bytes: Vec<u8>,
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

#[derive(Debug, Clone, Copy)]
struct DiagnosticOutput {
    format: DiagnosticsFormat,
    color_enabled: bool,
}

fn main() -> ExitCode {
    let args = expand_plantuml_text_format_args(std::env::args_os());
    let clap_color = clap_color_choice_from_args(&args);
    let cli = match Cli::command()
        .color(clap_color)
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
    {
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

    match run(cli) {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err((code, msg)) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            ExitCode::from(code)
        }
    }
}

fn expand_plantuml_text_format_args<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut expanded = Vec::new();
    for arg in args {
        match arg.to_str() {
            Some("-txt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("txt"));
            }
            Some("-atxt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("atxt"));
            }
            Some("-utxt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("utxt"));
            }
            Some("-encodesprite") => expanded.push(OsString::from("--encodesprite")),
            _ => expanded.push(arg),
        }
    }
    expanded
}

fn clap_color_choice_from_args(args: &[OsString]) -> clap::ColorChoice {
    match color_choice_from_args(args).unwrap_or_else(default_color_choice_from_env) {
        CliColorChoice::Always => clap::ColorChoice::Always,
        CliColorChoice::Never => clap::ColorChoice::Never,
        CliColorChoice::Auto => clap::ColorChoice::Auto,
    }
}

fn default_color_choice_from_env() -> CliColorChoice {
    if std::env::var_os("NO_COLOR").is_some() {
        CliColorChoice::Never
    } else {
        CliColorChoice::Auto
    }
}

fn color_choice_from_args(args: &[OsString]) -> Option<CliColorChoice> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(raw) = arg.to_str() else {
            continue;
        };
        if let Some(value) = raw.strip_prefix("--color=") {
            return parse_color_choice(value);
        }
        if raw == "--color" {
            return iter
                .next()
                .and_then(|value| value.to_str())
                .and_then(parse_color_choice);
        }
    }
    None
}

fn parse_color_choice(raw: &str) -> Option<CliColorChoice> {
    match raw {
        "auto" => Some(CliColorChoice::Auto),
        "always" => Some(CliColorChoice::Always),
        "never" => Some(CliColorChoice::Never),
        _ => None,
    }
}

fn should_color_human_diagnostics(choice: CliColorChoice) -> bool {
    match choice {
        CliColorChoice::Always => true,
        CliColorChoice::Never => false,
        CliColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal()
        }
    }
}

fn run(mut cli: Cli) -> Result<(), (u8, String)> {
    let started = Instant::now();
    let lint_context = LintSubcommandContext {
        include_root: cli.include_root.clone(),
        dialect: cli.dialect,
        compat: cli.compat,
        determinism: cli.determinism,
        from_markdown: cli.from_markdown,
        allow_url_includes: cli.allow_url_includes,
        inject_vars: cli.defines.iter().cloned().collect(),
    };
    if let Some(command) = cli.command.take() {
        return match command {
            CliCommand::Count(args) => cli_count::run_count(&args)
                .map(|_| ())
                .map_err(|(code, msg)| (code as u8, msg)),
            CliCommand::Env(args) => cli_env::run_env(&args)
                .map(|_| ())
                .map_err(|msg| (EXIT_VALIDATION, msg)),
            CliCommand::Format(args) => run_format_command(args),
            CliCommand::Hash(args) => cli_hash::run_hash(&args)
                .map(|_| ())
                .map_err(|(code, msg)| (code as u8, msg)),
            CliCommand::Lint(args) => run_lint_subcommand(args, lint_context),
            CliCommand::Stats(args) => cli_stats::run_stats(&args)
                .map(|_| ())
                .map_err(|(code, msg)| (code as u8, msg)),
        };
    }

    if cli.watch {
        return cli_watch::run_watch(&cli)
            .map(|_code| ())
            .map_err(|msg| (EXIT_IO, msg));
    }

    if cli.stdrpt {
        cli.diagnostics = DiagnosticsFormat::Stdrpt;
    }
    let diagnostics_output = DiagnosticOutput {
        format: cli.diagnostics,
        color_enabled: should_color_human_diagnostics(cli.color),
    };
    // Collect -D KEY=VALUE pairs into a BTreeMap for deterministic ordering per CLAUDE.md sec 6.
    let inject_vars: BTreeMap<String, String> = cli.defines.iter().cloned().collect();

    if !cli.charset.eq_ignore_ascii_case("utf-8") {
        return Err((
            EXIT_VALIDATION,
            format!(
                "[E_CHARSET_UNSUPPORTED] unsupported charset `{}`",
                cli.charset
            ),
        ));
    }
    if !cli.encodesprite.is_empty() {
        return run_encodesprite(&cli.encodesprite);
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
            frontend_hint_for_path(Some(path.as_path())),
            cli.allow_url_includes,
            inject_vars.clone(),
        )
        .map_err(|d| diag_err_with_source(&src, d, diagnostics_output))?;
        let model =
            normalize_family(doc).map_err(|d| diag_err_with_source(&src, d, diagnostics_output))?;
        emit_warnings_for_model(&model, &src, None, diagnostics_output);
        return Ok(());
    }

    if is_lint_mode_enabled(&cli) {
        return run_lint_mode(&cli);
    }

    let input_arg = if cli.pipe { None } else { cli.input.as_deref() };
    let (_input_name, raw, input_path) = read_input(input_arg)?;
    let include_root = cli
        .include_root
        .clone()
        .or_else(|| input_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
    let from_markdown = should_extract_markdown(cli.from_markdown, input_path);
    let file_frontend_hint = frontend_hint_for_path(input_path);
    let markdown_name_prefix = input_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string());
    let diagrams = split_diagrams(
        &raw,
        from_markdown,
        markdown_name_prefix.as_deref(),
        file_frontend_hint,
    )
    .map_err(|d| diag_err_with_source(&raw, d, diagnostics_output))?;

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

    if input_path.is_none() && diagrams.len() > 1 && !cli.multi && !cli.metadata {
        return Err((
            EXIT_VALIDATION,
            "multiple diagrams detected; rerun with --multi".to_string(),
        ));
    }

    if cli.preproc {
        let values = diagrams
            .iter()
            .map(|source| {
                preprocess_for_cli(
                    &source.source,
                    include_root.clone(),
                    cli.dialect,
                    cli.compat,
                    cli.determinism,
                    source.frontend_hint,
                    cli.allow_url_includes,
                    inject_vars.clone(),
                )
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (idx, preprocessed) in values.iter().enumerate() {
            if idx > 0 {
                println!();
            }
            print!("{preprocessed}");
            if !preprocessed.ends_with('\n') {
                println!();
            }
        }
        if cli.duration {
            eprintln!("elapsed: {:?}", started.elapsed());
        }
        return Ok(());
    }

    if cli.metadata {
        let values = diagrams
            .iter()
            .map(|source| {
                let doc = parse_for_cli(
                    &source.source,
                    include_root.clone(),
                    cli.dialect,
                    cli.compat,
                    cli.determinism,
                    source.frontend_hint,
                    cli.allow_url_includes,
                    inject_vars.clone(),
                )
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
                let ast = doc.clone();
                let model = normalize_family(doc).map_err(|d| {
                    diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                })?;
                Ok(extract_metadata(&ast, &model))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if values.len() == 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&values[0]).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize metadata output: {e}")
                ))?
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&values).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize metadata output: {e}")
                ))?
            );
        }
        return Ok(());
    }

    if cli.check || cli.check_syntax {
        let mut had_warnings = false;
        for source in &diagrams {
            if cli.verbose {
                eprintln!("[verbose] parse");
            }
            let doc = parse_for_cli(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                cli.determinism,
                source.frontend_hint,
                cli.allow_url_includes,
                inject_vars.clone(),
            )
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
            let model = normalize_family(doc)
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
            let warnings = normalized_warnings(&model);
            had_warnings |= !warnings.is_empty();
            if !cli.quiet {
                emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output);
            }
        }
        if cli.duration {
            eprintln!("elapsed: {:?}", started.elapsed());
        }
        if cli.fail_on_warn && had_warnings {
            return Err((
                EXIT_VALIDATION,
                "[E_WARNINGS_PRESENT] warnings present".to_string(),
            ));
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
                        cli.allow_url_includes,
                        inject_vars.clone(),
                    )
                    .map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                    })?;
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
                        cli.allow_url_includes,
                        inject_vars.clone(),
                    )
                    .map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                    })?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                    })?;
                    emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output);
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
                        cli.allow_url_includes,
                        inject_vars.clone(),
                    )
                    .map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                    })?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, diagnostics_output)
                    })?;
                    emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output);
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
        // Short-circuit for specialized families (math, ditaa, etc.) after the
        // same preprocessor pass used by check/dump routes.
        // Text modes intentionally route through normalized models instead.
        if cli.format.uses_svg_renderer() && specialized::is_specialized_source(&source.source) {
            let preprocessed = preprocess_for_cli(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                cli.determinism,
                source.frontend_hint,
                cli.allow_url_includes,
                inject_vars.clone(),
            )
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
            let result = specialized::try_render_specialized(&preprocessed).ok_or_else(|| {
                (
                    EXIT_VALIDATION,
                    "[E_SPECIALIZED_PREPROC] preprocessed specialized source changed family"
                        .to_string(),
                )
            })?;
            let svg = result
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
            let name_hint = source
                .output_name_hint
                .as_ref()
                .map(|base| format!("{base}.{}", output_extension(cli.format)));
            all.push(RenderedOutput {
                name_hint,
                content: render_svg_export_content(&svg, cli.format),
            });
            return Ok(all);
        }
        let doc = parse_for_cli(
            &source.source,
            include_root.clone(),
            cli.dialect,
            cli.compat,
            cli.determinism,
            source.frontend_hint,
            cli.allow_url_includes,
            inject_vars.clone(),
        )
        .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
        let model = normalize_family(doc)
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
        emit_warnings_for_model(&model, &raw, source.source_span, diagnostics_output);
        let pages = render_pages_from_model(&model, cli.format);
        let page_count = pages.len();
        for (page_idx, content) in pages.into_iter().enumerate() {
            let name_hint = source.output_name_hint.as_ref().map(|base| {
                if page_count == 1 {
                    format!("{base}.{}", output_extension(cli.format))
                } else {
                    format!("{base}-{}.{}", page_idx + 1, output_extension(cli.format))
                }
            });
            all.push(RenderedOutput { name_hint, content });
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
        if cli.format.is_binary() {
            return Err((
                EXIT_VALIDATION,
                format!(
                    "multiple {} outputs on stdin are not supported; provide file input or --output",
                    output_extension(cli.format).to_uppercase()
                )
                    .to_string(),
            ));
        }
        let payload = outputs
            .iter()
            .enumerate()
            .map(|(idx, out)| MultiSvgOut {
                name: out.name_hint.clone().unwrap_or_else(|| {
                    format!("diagram-{}.{}", idx + 1, output_extension(cli.format))
                }),
                svg: if cli.format == OutputFormat::Svg {
                    Some(out.content.clone())
                } else {
                    None
                },
                html: if cli.format == OutputFormat::Html {
                    Some(out.content.clone())
                } else {
                    None
                },
                text: if cli.format.is_text() {
                    Some(out.content.clone())
                } else {
                    None
                },
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

    let binary_outputs = outputs
        .iter()
        .map(|out| render_output_bytes(out, cli.format, cli.dpi))
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(path) = cli.output {
        let payloads = binary_outputs
            .iter()
            .map(|out| out.bytes.clone())
            .collect::<Vec<_>>();
        write_output_files(&path, &payloads)?;
        return Ok(());
    }

    if let Some(input) = input_path {
        if from_markdown {
            write_markdown_output_files(input, &binary_outputs)?;
        } else {
            let default_base = default_output_base(input, cli.format)?;
            let payloads = binary_outputs
                .iter()
                .map(|out| out.bytes.clone())
                .collect::<Vec<_>>();
            write_output_files(&default_base, &payloads)?;
        }
        return Ok(());
    }

    if outputs.len() == 1 {
        match cli.format {
            OutputFormat::Svg | OutputFormat::Html => {
                println!("{}", outputs[0].content);
            }
            OutputFormat::Png | OutputFormat::Jpg | OutputFormat::Webp | OutputFormat::Pdf => {
                io::stdout()
                    .write_all(&binary_outputs[0].bytes)
                    .map_err(|e| {
                        (
                            EXIT_IO,
                            format!(
                                "failed to write {} to stdout: {e}",
                                output_extension(cli.format).to_uppercase()
                            ),
                        )
                    })?;
            }
            OutputFormat::Txt | OutputFormat::Atxt | OutputFormat::Utxt => {
                print!("{}", outputs[0].content);
            }
        }
        return Ok(());
    }

    Err((EXIT_INTERNAL, "unexpected stdin output mode".to_string()))
}

fn run_encodesprite(args: &[String]) -> Result<(), (u8, String)> {
    let [format, image_path] = args else {
        return Err((
            EXIT_VALIDATION,
            "encodesprite requires a format and image path".to_string(),
        ));
    };
    let (gray_levels, compressed) = parse_sprite_encode_format(format)?;
    let path = Path::new(image_path);
    let image = image::open(path)
        .map_err(|e| {
            (
                EXIT_IO,
                format!("failed to read image '{}': {e}", path.display()),
            )
        })?
        .to_rgba8();
    let width = image.width();
    let height = image.height();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        let luminance = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;
        let alpha = a as f32 / 255.0;
        let darkness = (255_u8.saturating_sub(luminance)) as f32 / 255.0;
        pixels.push(((darkness * alpha * 15.0).round() as u8).min(15));
    }
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sprite")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let encoded =
        puml::sprites::encode_pixels(&name, width, height, gray_levels, compressed, &pixels)
            .map_err(|d| (EXIT_VALIDATION, d.message))?;
    println!("{encoded}");
    Ok(())
}

fn parse_sprite_encode_format(raw: &str) -> Result<(u8, bool), (u8, String)> {
    let trimmed = raw.trim();
    let compressed = trimmed.ends_with('z') || trimmed.ends_with('Z');
    let level_text = if compressed {
        &trimmed[..trimmed.len().saturating_sub(1)]
    } else {
        trimmed
    };
    let gray_levels = level_text.parse::<u8>().map_err(|_| {
        (
            EXIT_VALIDATION,
            format!("invalid encodesprite format `{raw}`; expected 4, 8, 16, 4z, 8z, or 16z"),
        )
    })?;
    if matches!(gray_levels, 4 | 8 | 16) {
        Ok((gray_levels, compressed))
    } else {
        Err((
            EXIT_VALIDATION,
            format!("invalid encodesprite format `{raw}`; expected 4, 8, 16, 4z, 8z, or 16z"),
        ))
    }
}

fn run_format_command(args: FormatArgs) -> Result<(), (u8, String)> {
    let mut changed_paths = Vec::new();
    let mut writes = Vec::new();

    for path in &args.files {
        if path == Path::new("-") {
            return Err((
                EXIT_VALIDATION,
                "puml format requires file paths; stdin cannot be formatted in place".to_string(),
            ));
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", path.display())))?;
        let result = puml::formatter::format_source(&raw);
        if result.changed {
            changed_paths.push(path.clone());
            if args.diff {
                print!("{}", format_unified_diff(path, &raw, &result.formatted));
            }
            if !args.check && !args.diff {
                writes.push((path.clone(), result.formatted.into_bytes()));
            }
        }
    }

    if !writes.is_empty() {
        write_files_transactionally(writes)?;
    }

    if args.check && !changed_paths.is_empty() {
        let files = changed_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err((
            EXIT_VALIDATION,
            format!("formatting changes needed: {files}"),
        ));
    }

    Ok(())
}

// ── puml lint <file> ──────────────────────────────────────────────────────────
//
// Parse and normalize only; no rendering.  Exit codes:
//   0  no errors (warnings may still be emitted)
//   1  at least one diagnostic error
//   2  I/O failure (file unreadable)
//
// JSON output schema:
//   { "file": "...",
//     "diagnostics": [ { "severity", "code", "message",
//                         "span": { "start_line", "start_col",
//                                   "end_line",   "end_col"  } } ],
//     "summary": { "errors": N, "warnings": N } }

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
struct LintSubcommandContext {
    include_root: Option<PathBuf>,
    dialect: CliDialect,
    compat: CliCompatMode,
    determinism: CliDeterminismMode,
    from_markdown: bool,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
}

fn run_lint_subcommand(args: LintArgs, context: LintSubcommandContext) -> Result<(), (u8, String)> {
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
            context.determinism,
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
            let (sl, sc) = line_col_at(source, span.start);
            let (el, ec) = line_col_at(source, span.end);
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

fn line_col_at(source: &str, offset: usize) -> (usize, usize) {
    let off = offset.min(source.len());
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (idx, ch) in source.char_indices() {
        if idx >= off {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    let col = source[line_start..off].chars().count() + 1;
    (line, col)
}

fn format_unified_diff(path: &Path, old: &str, new: &str) -> String {
    let old_display = old.replace("\r\n", "\n").replace('\r', "\n");
    let path_display = path.display();
    let mut diff = format!("--- {path_display}\n+++ {path_display} (formatted)\n");

    if old_display == new {
        diff.push_str("@@ line endings @@\n");
        diff.push_str("-contains CRLF or CR line endings\n");
        diff.push_str("+uses LF line endings\n");
        return diff;
    }

    let old_lines = diff_lines(&old_display);
    let new_lines = diff_lines(new);
    diff.push_str(&format!(
        "@@ -1,{} +1,{} @@\n",
        old_lines.len(),
        new_lines.len()
    ));
    for line in old_lines {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in new_lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff
}

fn diff_lines(source: &str) -> Vec<&str> {
    let mut lines = source.split('\n').collect::<Vec<_>>();
    if source.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn is_lint_mode_enabled(cli: &Cli) -> bool {
    !cli.lint_input.is_empty() || !cli.lint_glob.is_empty()
}

fn run_lint_mode(cli: &Cli) -> Result<(), (u8, String)> {
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

fn lsp_capabilities_manifest() -> Value {
    puml::lsp_capabilities()
}

fn diag_err_with_source(source: &str, d: Diagnostic, output: DiagnosticOutput) -> (u8, String) {
    match output.format {
        DiagnosticsFormat::Human => (
            EXIT_VALIDATION,
            render_human_diagnostic(&d, source, output.color_enabled),
        ),
        DiagnosticsFormat::Json => (EXIT_VALIDATION, diagnostics_json_payload(vec![d], source)),
        DiagnosticsFormat::Stdrpt => (EXIT_VALIDATION, diagnostic_stdrpt(&d, source)),
    }
}

fn diagnostic_stdrpt(d: &Diagnostic, source: &str) -> String {
    let json = d.to_json_with_source(source);
    let code = json.code.unwrap_or_default();
    let location = match (json.line, json.column) {
        (Some(line), Some(col)) => format!("-:{line}:{col}"),
        _ => "-".to_string(),
    };
    format!("{}\t{}\t{}\t{}", json.severity, code, location, d.message)
}

fn render_human_diagnostic(d: &Diagnostic, source: &str, color_enabled: bool) -> String {
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

fn diag_err_mapped(
    raw_source: &str,
    mapping: Option<Span>,
    d: Diagnostic,
    output: DiagnosticOutput,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source(raw_source, mapped, output)
}

fn emit_warnings_for_model(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<Span>,
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

fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
        NormalizedDocument::FamilyPages(pages) => pages
            .iter()
            .find_map(|page| (!page.warnings.is_empty()).then_some(page.warnings.as_slice()))
            .unwrap_or(&[]),
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
    }
}

fn render_pages_from_model(model: &NormalizedDocument, format: OutputFormat) -> Vec<String> {
    match format.text_mode() {
        Some(mode) => render::render_text_pages(model, mode),
        None => render_svg_pages_from_model(model)
            .into_iter()
            .map(|svg| render_svg_export_content(&svg, format))
            .collect(),
    }
}

fn render_svg_export_content(svg: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Html => svg_to_html_document(svg),
        _ => svg.to_string(),
    }
}

fn svg_to_html_document(svg: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>puml diagram</title>\n<style>html,body{{margin:0;min-height:100%;background:#fff;}}body{{display:flex;align-items:flex-start;justify-content:center;padding:16px;box-sizing:border-box;}}svg{{max-width:100%;height:auto;}}</style>\n</head>\n<body>\n{svg}\n</body>\n</html>"
    )
}

// All CLI pipeline parameters are required at call sites; grouping them into a
// struct would not reduce complexity here — the lint is a false positive.
#[allow(clippy::too_many_arguments)]
fn parse_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
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
        allow_url_includes,
        inject_vars,
    };
    puml::parse_with_pipeline_options(source, &options)
}

// Same rationale as parse_for_cli above — all args are required.
#[allow(clippy::too_many_arguments)]
fn preprocess_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
) -> Result<String, Diagnostic> {
    let include_root = include_root.or_else(|| match cli_compat {
        CliCompatMode::Strict => None,
        CliCompatMode::Extended => std::env::current_dir().ok(),
    });
    let options = ParsePipelineOptions {
        frontend: map_frontend(cli_dialect, frontend_hint),
        compat: map_compat(cli_compat),
        determinism: map_determinism(cli_determinism),
        include_root,
        allow_url_includes,
        inject_vars,
    };
    preprocess_with_pipeline_options(source, &options)
}

fn map_frontend(
    dialect: CliDialect,
    frontend_hint: Option<FrontendSelection>,
) -> FrontendSelection {
    // Auto mode is the only mode that accepts routing hints from file
    // extensions (`.picouml`) or markdown fence tags (`picouml`, `mermaid`).
    // Explicit `--dialect` keeps user intent ahead of extension names.
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

fn frontend_hint_for_path(path: Option<&Path>) -> Option<FrontendSelection> {
    path.and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
            "picouml" => Some(FrontendSelection::Picouml),
            _ => None,
        })
}

fn split_diagrams(
    raw: &str,
    from_markdown: bool,
    markdown_name_prefix: Option<&str>,
    file_frontend_hint: Option<FrontendSelection>,
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
                    frontend_hint: file_frontend_hint,
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
        frontend_hint: file_frontend_hint,
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

fn default_output_base(input: &Path, format: OutputFormat) -> Result<PathBuf, (u8, String)> {
    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output name from '{}': invalid stem",
                input.display()
            ),
        )
    })?;
    Ok(input.with_file_name(format!("{stem}.{}", output_extension(format))))
}

fn write_markdown_output_files(
    input: &Path,
    outputs: &[RenderedBinaryOutput],
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
        files.push((path, out.bytes.clone()));
    }
    write_files_transactionally(files)
}

fn write_output_files(base: &Path, payloads: &[Vec<u8>]) -> Result<(), (u8, String)> {
    if payloads.len() == 1 {
        return write_files_transactionally(vec![(base.to_path_buf(), payloads[0].clone())]);
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
    let mut files = Vec::with_capacity(payloads.len());

    for (idx, payload) in payloads.iter().enumerate() {
        let path = parent.join(format!("{stem}-{}.{}", idx + 1, ext));
        files.push((path, payload.clone()));
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

fn write_files_transactionally(files: Vec<(PathBuf, Vec<u8>)>) -> Result<(), (u8, String)> {
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

fn output_extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Svg => "svg",
        OutputFormat::Html => "html",
        OutputFormat::Png => "png",
        OutputFormat::Jpg => "jpg",
        OutputFormat::Webp => "webp",
        OutputFormat::Pdf => "pdf",
        OutputFormat::Txt => "txt",
        OutputFormat::Atxt => "atxt",
        OutputFormat::Utxt => "utxt",
    }
}

impl OutputFormat {
    fn uses_svg_renderer(self) -> bool {
        matches!(
            self,
            Self::Svg | Self::Html | Self::Png | Self::Jpg | Self::Webp | Self::Pdf
        )
    }

    fn is_binary(self) -> bool {
        matches!(self, Self::Png | Self::Jpg | Self::Webp | Self::Pdf)
    }

    fn is_text(self) -> bool {
        self.text_mode().is_some()
    }

    fn text_mode(self) -> Option<TextOutputMode> {
        match self {
            Self::Svg | Self::Html | Self::Png | Self::Jpg | Self::Webp | Self::Pdf => None,
            Self::Txt => Some(TextOutputMode::Txt),
            Self::Atxt => Some(TextOutputMode::Atxt),
            Self::Utxt => Some(TextOutputMode::Utxt),
        }
    }
}

fn render_output_bytes(
    output: &RenderedOutput,
    format: OutputFormat,
    dpi: f32,
) -> Result<RenderedBinaryOutput, (u8, String)> {
    let bytes = match format {
        OutputFormat::Svg
        | OutputFormat::Html
        | OutputFormat::Txt
        | OutputFormat::Atxt
        | OutputFormat::Utxt => output.content.as_bytes().to_vec(),
        OutputFormat::Png | OutputFormat::Jpg | OutputFormat::Webp => {
            svg_to_raster_bytes(&output.content, format, dpi)?
        }
        OutputFormat::Pdf => svg_to_pdf_bytes(&output.content)?,
    };
    Ok(RenderedBinaryOutput {
        name_hint: output.name_hint.clone(),
        bytes,
    })
}

struct RasterizedSvg {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn svg_to_raster_bytes(svg: &str, format: OutputFormat, dpi: f32) -> Result<Vec<u8>, (u8, String)> {
    let raster = rasterize_svg(svg, dpi)?;
    match format {
        OutputFormat::Png => encode_png(&raster),
        OutputFormat::Jpg => encode_jpg(&raster),
        OutputFormat::Webp => encode_webp(&raster),
        _ => Err((
            EXIT_INTERNAL,
            format!(
                "format '{}' does not use SVG raster export",
                output_extension(format)
            ),
        )),
    }
}

#[cfg(feature = "cli")]
fn svg_to_pdf_bytes(svg: &str) -> Result<Vec<u8>, (u8, String)> {
    let mut opt = svg2pdf::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = svg2pdf::usvg::Tree::from_str(svg, &opt).map_err(|e| {
        (
            EXIT_VALIDATION,
            format!("failed to parse rendered SVG for PDF output: {e}"),
        )
    })?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| (EXIT_INTERNAL, format!("failed to convert SVG to PDF: {e}")))
}

fn rasterize_svg(svg: &str, dpi: f32) -> Result<RasterizedSvg, (u8, String)> {
    let mut opt = resvg::usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();
    fontdb.set_monospace_family("Liberation Mono");
    let tree = resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| {
        (
            EXIT_VALIDATION,
            format!("failed to parse rendered SVG for PNG output: {e}"),
        )
    })?;

    let size = tree.size();
    let scale = dpi / 96.0;
    let width = (size.width() * scale).round() as u32;
    let height = (size.height() * scale).round() as u32;
    if width == 0 || height == 0 {
        return Err((
            EXIT_INTERNAL,
            "failed to rasterize PNG: computed zero-sized output".to_string(),
        ));
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        (
            EXIT_INTERNAL,
            format!("failed to allocate PNG surface {width}x{height}"),
        )
    })?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(RasterizedSvg {
        width,
        height,
        rgba: pixmap.data().to_vec(),
    })
}

fn encode_png(raster: &RasterizedSvg) -> Result<Vec<u8>, (u8, String)> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            &raster.rgba,
            raster.width,
            raster.height,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| (EXIT_IO, format!("failed to encode PNG: {e}")))?;
    Ok(png)
}

fn encode_jpg(raster: &RasterizedSvg) -> Result<Vec<u8>, (u8, String)> {
    let rgb = rgba_to_rgb_over_white(&raster.rgba);
    let mut jpg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg, 90)
        .write_image(
            &rgb,
            raster.width,
            raster.height,
            image::ColorType::Rgb8.into(),
        )
        .map_err(|e| (EXIT_IO, format!("failed to encode JPG: {e}")))?;
    Ok(jpg)
}

fn encode_webp(raster: &RasterizedSvg) -> Result<Vec<u8>, (u8, String)> {
    let mut webp = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
        .write_image(
            &raster.rgba,
            raster.width,
            raster.height,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| (EXIT_IO, format!("failed to encode WebP: {e}")))?;
    Ok(webp)
}

fn rgba_to_rgb_over_white(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for pixel in rgba.chunks_exact(4) {
        let alpha = pixel[3] as u16;
        for channel in &pixel[..3] {
            let value = ((*channel as u16 * alpha) + (255 * (255 - alpha)) + 127) / 255;
            rgb.push(value as u8);
        }
    }
    rgb
}

fn ast_to_json(doc: &Document) -> Value {
    json!({
        "kind": match doc.kind {
            DiagramKind::Sequence => "Sequence",
            DiagramKind::Class => "Class",
            DiagramKind::Object => "Object",
            DiagramKind::UseCase => "UseCase",
            DiagramKind::MindMap => "MindMap",
            DiagramKind::Wbs => "Wbs",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            DiagramKind::Component => "Component",
            DiagramKind::Deployment => "Deployment",
            DiagramKind::State => "State",
            DiagramKind::Activity => "Activity",
            DiagramKind::Timing => "Timing",
            DiagramKind::Salt => "Salt",
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
        StatementKind::StateDecl(v) => {
            json!({"StateDecl": {"name": v.name, "alias": v.alias, "stereotype": v.stereotype}})
        }
        StatementKind::StateTransition(v) => {
            json!({"StateTransition": {"from": v.from, "to": v.to, "label": v.label}})
        }
        StatementKind::StateInternalAction(v) => {
            json!({"StateInternalAction": {"state": v.state, "kind": v.kind, "action": v.action}})
        }
        StatementKind::StateRegionDivider => json!("StateRegionDivider"),
        StatementKind::StateHistory { deep } => json!({"StateHistory": {"deep": deep}}),
        StatementKind::GanttTaskDecl {
            name,
            alias,
            resources,
            ..
        } => json!({"GanttTaskDecl": {"name": name, "alias": alias, "resources": resources}}),
        StatementKind::GanttCompound {
            name,
            alias,
            resources,
            clauses,
            after_previous,
        } => {
            json!({"GanttCompound": {"name": name, "alias": alias, "resources": resources, "clauses": clauses, "after_previous": after_previous}})
        }
        StatementKind::GanttMilestoneDecl { name, happens_on } => {
            json!({"GanttMilestoneDecl": {"name": name, "happens_on": happens_on}})
        }
        StatementKind::GanttConstraint {
            subject,
            kind,
            target,
        } => {
            json!({"GanttConstraint": {"subject": subject, "kind": kind, "target": target}})
        }
        StatementKind::GanttCalendarClosed { day } => {
            json!({"GanttCalendarClosed": {"day": day}})
        }
        StatementKind::GanttCalendarOpen { day } => {
            json!({"GanttCalendarOpen": {"day": day}})
        }
        StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        } => json!({
            "GanttCalendarClosedDateRange": {
                "start_date": start_date,
                "end_date": end_date
            }
        }),
        StatementKind::GanttCalendarOpenDateRange {
            start_date,
            end_date,
        } => json!({
            "GanttCalendarOpenDateRange": {
                "start_date": start_date,
                "end_date": end_date
            }
        }),
        StatementKind::GanttNamedDate { date, label } => {
            json!({"GanttNamedDate": {"date": date, "label": label}})
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
        StatementKind::Spacer(pixels) => json!({"Spacer": pixels}),
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
        StatementKind::SpriteDef(sprite) => json!({
            "SpriteDef": {
                "name": sprite.name,
                "width": sprite.width,
                "height": sprite.height,
                "gray_levels": sprite.gray_levels
            }
        }),
        StatementKind::ListSprites => json!("ListSprites"),
        StatementKind::Unknown(v) => json!({"Unknown": v}),
        StatementKind::JsonProjection { alias, body } => json!({
            "JsonProjection": {"alias": alias, "body": body}
        }),
        StatementKind::YamlProjection { alias, body } => json!({
            "YamlProjection": {"alias": alias, "body": body}
        }),
        other => json!({"Other": format!("{other:?}")}),
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
                puml::ast::VirtualEndpointKind::Short => "short",
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
                puml::ast::VirtualEndpointKind::Short => "short",
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
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(family_model_to_json).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => timeline_model_to_json(timeline),
        NormalizedDocument::State(state) => state_model_to_json(state),
        NormalizedDocument::Json(doc) => json!({"kind": "Json", "warnings": doc.warnings.len()}),
        NormalizedDocument::Yaml(doc) => json!({"kind": "Yaml", "warnings": doc.warnings.len()}),
        NormalizedDocument::Nwdiag(doc) => {
            json!({"kind": "Nwdiag", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Archimate(doc) => {
            json!({"kind": "Archimate", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Regex(doc) => json!({"kind": "Regex", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ebnf(doc) => json!({"kind": "Ebnf", "warnings": doc.warnings.len()}),
        NormalizedDocument::Math(doc) => json!({"kind": "Math", "warnings": doc.warnings.len()}),
        NormalizedDocument::Sdl(doc) => json!({"kind": "Sdl", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ditaa(doc) => json!({"kind": "Ditaa", "warnings": doc.warnings.len()}),
        NormalizedDocument::Chart(doc) => json!({"kind": "Chart", "warnings": doc.warnings.len()}),
    }
}

fn state_model_to_json(model: &StateDocument) -> Value {
    json!({
        "kind": "State",
        "nodes": model.nodes.iter().map(|n| json!({
            "name": n.name,
            "display": n.display,
            "kind": match n.kind {
                puml::model::StateNodeKind::Normal => "Normal",
                puml::model::StateNodeKind::StartEnd => "StartEnd",
                puml::model::StateNodeKind::HistoryShallow => "HistoryShallow",
                puml::model::StateNodeKind::HistoryDeep => "HistoryDeep",
                puml::model::StateNodeKind::Fork => "Fork",
                puml::model::StateNodeKind::Join => "Join",
                puml::model::StateNodeKind::Choice => "Choice",
                puml::model::StateNodeKind::End => "End",
                puml::model::StateNodeKind::EntryPoint => "EntryPoint",
                puml::model::StateNodeKind::ExitPoint => "ExitPoint",
                puml::model::StateNodeKind::InputPin => "InputPin",
                puml::model::StateNodeKind::OutputPin => "OutputPin",
                puml::model::StateNodeKind::ExpansionInput => "ExpansionInput",
                puml::model::StateNodeKind::ExpansionOutput => "ExpansionOutput",
                puml::model::StateNodeKind::Note => "Note",
                puml::model::StateNodeKind::JsonProjection => "JsonProjection",
            },
            "style": {
                "fill_color": n.style.fill_color,
                "border_color": n.style.border_color,
                "border_dashed": n.style.border_dashed,
                "border_thickness": n.style.border_thickness,
                "text_color": n.style.text_color,
            },
            "internal_actions": n.internal_actions.iter().map(|a| json!({
                "kind": a.kind,
                "action": a.action
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "transitions": model.transitions.iter().map(|t| json!({
            "from": t.from,
            "to": t.to,
            "label": t.label
        })).collect::<Vec<_>>(),
        "title": model.title,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn model_to_json(model: &SequenceDocument) -> Value {
    json!({
        "participants": model.participants.iter().map(model_participant_to_json).collect::<Vec<_>>(),
        "events": model.events.iter().map(model_event_to_json).collect::<Vec<_>>(),
        "teoz": model.teoz,
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "skinparams": model.skinparams,
        "style": {
            "arrow_color": model.style.arrow_color,
            "lifeline_border_color": model.style.lifeline_border_color,
            "participant_background_color": model.style.participant_background_color,
            "participant_border_color": model.style.participant_border_color,
            "note_background_color": model.style.note_background_color,
            "note_border_color": model.style.note_border_color,
            "group_background_color": model.style.group_background_color,
            "group_border_color": model.style.group_border_color
        },
        "footbox_visible": model.footbox_visible
    })
}

fn family_model_to_json(model: &puml::FamilyDocument) -> Value {
    json!({
        "kind": format!("{:?}", model.kind),
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
            DiagramKind::Salt => "Salt",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            _ => "Timeline",
        },
        "tasks": model
            .tasks
            .iter()
            .map(|t| json!({"name": t.name, "start_day": t.start_day, "workload_days": t.workload_days, "duration_days": t.duration_days, "resources": t.resources, "resource_allocations": t.resource_allocations.iter().map(|r| json!({"name": r.name, "load_percent": r.load_percent})).collect::<Vec<_>>()}))
            .collect::<Vec<_>>(),
        "milestones": model.milestones.iter().map(|m| json!({"name": m.name, "happens_on": m.happens_on})).collect::<Vec<_>>(),
        "separators": model.separators.iter().map(|s| json!({"label": s.label, "target": s.target})).collect::<Vec<_>>(),
        "constraints": model
            .constraints
            .iter()
            .map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target}))
            .collect::<Vec<_>>(),
        "closed_weekdays": model.closed_weekdays,
        "closed_ranges": model
            .closed_ranges
            .iter()
            .map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day}))
            .collect::<Vec<_>>(),
        "open_ranges": model
            .open_ranges
            .iter()
            .map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day}))
            .collect::<Vec<_>>(),
        "named_dates": model
            .named_dates
            .iter()
            .map(|n| json!({"date": n.date, "label": n.label, "day": n.day}))
            .collect::<Vec<_>>(),
        "chronology_events": model
            .chronology_events
            .iter()
            .map(|e| json!({"subject": e.subject, "when": e.when}))
            .collect::<Vec<_>>(),
        "project_start": model.project_start,
        "project_start_day": model.project_start_day,
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
            style: _,
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
            kind,
            position,
            target,
            text,
            ..
        } => {
            json!({"Note": {"kind": format!("{:?}", kind), "position": position, "target": target, "text": text}})
        }
        SequenceEventKind::GroupStart { kind, label } => {
            json!({"GroupStart": {"kind": kind, "label": label}})
        }
        SequenceEventKind::GroupEnd => json!("GroupEnd"),
        SequenceEventKind::Delay(v) => json!({"Delay": v}),
        SequenceEventKind::Divider(v) => json!({"Divider": v}),
        SequenceEventKind::Separator(v) => json!({"Separator": v}),
        SequenceEventKind::Spacer(pixels) => json!({"Spacer": pixels}),
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
            VirtualEndpointKind::Short => "short",
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
                "family": format!("{:?}", family.kind),
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
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(|family| {
                let svg = render::render_family_stub_svg(family);
                json!({
                    "kind": "FamilyStub",
                    "family": format!("{:?}", family.kind),
                    "nodes": family.nodes.iter().map(|n| json!({
                        "kind": format!("{:?}", n.kind),
                        "name": n.name,
                        "alias": n.alias
                    })).collect::<Vec<_>>(),
                    "relations": family.relations.iter().map(|r| json!({
                        "from": r.from,
                        "to": r.to,
                        "arrow": r.arrow,
                        "label": r.label
                    })).collect::<Vec<_>>(),
                    "svg_preview": svg
                })
            }).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => {
            json!({
                "kind": "TimelineScene",
                "family": match timeline.kind {
                    DiagramKind::Salt => "Salt",
                    DiagramKind::Gantt => "Gantt",
                    DiagramKind::Chronology => "Chronology",
                    _ => "Timeline",
                },
                "tasks": timeline
                    .tasks
                    .iter()
                    .map(|t| json!({"name": t.name, "start_day": t.start_day, "workload_days": t.workload_days, "duration_days": t.duration_days, "resources": t.resources, "resource_allocations": t.resource_allocations.iter().map(|r| json!({"name": r.name, "load_percent": r.load_percent})).collect::<Vec<_>>()}))
                    .collect::<Vec<_>>(),
                "milestones": timeline.milestones.iter().map(|m| json!({"name": m.name, "happens_on": m.happens_on})).collect::<Vec<_>>(),
                "separators": timeline.separators.iter().map(|s| json!({"label": s.label, "target": s.target})).collect::<Vec<_>>(),
                "constraints": timeline.constraints.iter().map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target})).collect::<Vec<_>>(),
                "closed_weekdays": timeline.closed_weekdays,
                "closed_ranges": timeline.closed_ranges.iter().map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day})).collect::<Vec<_>>(),
                "open_ranges": timeline.open_ranges.iter().map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day})).collect::<Vec<_>>(),
                "chronology_events": timeline.chronology_events.iter().map(|e| json!({"subject": e.subject, "when": e.when})).collect::<Vec<_>>(),
                "project_start": timeline.project_start,
                "project_start_day": timeline.project_start_day,
                "title": timeline.title,
                "header": timeline.header,
                "footer": timeline.footer,
                "caption": timeline.caption,
                "legend": timeline.legend,
                "svg_preview": render::render_timeline_svg(timeline),
                "warnings": timeline.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
            })
        }
        NormalizedDocument::State(state) => {
            let svg = render::render_state_svg(state);
            json!({
                "kind": "StateDiagram",
                "nodes": state.nodes.len(),
                "transitions": state.transitions.len(),
                "svg_preview": svg
            })
        }
        other => normalized_model_to_json(other),
    }
}
