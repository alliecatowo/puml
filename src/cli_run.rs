use crate::cli::{
    Cli, ColorChoice as CliColorChoice, Command as CliCommand, CompatMode as CliCompatMode,
    DeterminismMode as CliDeterminismMode, DiagnosticsFormat, Dialect as CliDialect, DumpKind,
    FormatArgs, LintArgs, LintFormat, LintReportFormat, OutputFormat,
};
use crate::cli_dump::{normalized_model_to_json, normalized_scene_to_json};
use crate::cli_dump_ast::ast_to_json;
use crate::{cli_count, cli_env, cli_hash, cli_stats, cli_watch};
use glob::glob;
use puml::ast::Document;
use puml::diagnostic::{diagnostic_message_and_code, normalized_warnings, offset_to_line_col};
use puml::output::{
    render_output_bytes, render_svg_export_content, OutputError, OutputErrorKind,
    RenderedBinaryOutput, RenderedOutput,
};
use puml::source::Span;
use puml::{
    extract_markdown_diagrams, extract_metadata, normalize_family,
    preprocess_with_pipeline_options, render, render_svg_pages_from_model, specialized, CompatMode,
    DeterminismMode, Diagnostic, DiagnosticJson, DiagramInput, FrontendSelection,
    NormalizedDocument, ParsePipelineOptions,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

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

#[derive(Debug, Clone)]
struct InputDiagram {
    source: String,
    source_span: Option<Span>,
    frontend_hint: Option<FrontendSelection>,
    output_name_hint: Option<String>,
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

fn should_color_human_diagnostics(choice: CliColorChoice) -> bool {
    match choice {
        CliColorChoice::Always => true,
        CliColorChoice::Never => false,
        CliColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal()
        }
    }
}

pub(crate) fn run(mut cli: Cli) -> Result<(), (u8, String)> {
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
    if cli.stdlib {
        let root = puml::stdlib::resolve_local_stdlib_root(cli.include_root.as_deref())
            .map_err(|msg| (EXIT_IO, format!("[E_STDLIB_ROOT] {msg}")))?;
        let entries = puml::stdlib::inventory_from_root(&root)
            .map_err(|msg| (EXIT_IO, format!("[E_STDLIB_INVENTORY] {msg}")))?;
        print!("{}", puml::stdlib::format_stdlib_listing(&root, &entries));
        if cli.duration {
            eprintln!("elapsed: {:?}", started.elapsed());
        }
        return Ok(());
    }

    if let Some(path) = &cli.check_fixture {
        let fixture_label = path.display().to_string();
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
        .map_err(|d| {
            diag_err_with_source_label(&src, d, diagnostics_output, Some(&fixture_label))
        })?;
        let model = normalize_family(doc).map_err(|d| {
            diag_err_with_source_label(&src, d, diagnostics_output, Some(&fixture_label))
        })?;
        emit_warnings_for_model_label(&model, &src, None, diagnostics_output, Some(&fixture_label));
        return Ok(());
    }

    if is_lint_mode_enabled(&cli) {
        return run_lint_mode(&cli);
    }

    let input_arg = if cli.pipe { None } else { cli.input.as_deref() };
    let (_input_name, raw, input_path) = read_input(input_arg)?;
    let input_label = input_path.map(|path| path.display().to_string());
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
    .map_err(|d| diag_err_with_source_label(&raw, d, diagnostics_output, input_label.as_deref()))?;

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
                .map_err(|d| {
                    diag_err_mapped_label(
                        &raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label.as_deref(),
                    )
                })
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
                .map_err(|d| {
                    diag_err_mapped_label(
                        &raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label.as_deref(),
                    )
                })?;
                let ast = doc.clone();
                let model = normalize_family(doc).map_err(|d| {
                    diag_err_mapped_label(
                        &raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label.as_deref(),
                    )
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
            .map_err(|d| {
                diag_err_mapped_label(
                    &raw,
                    source.source_span,
                    d,
                    diagnostics_output,
                    input_label.as_deref(),
                )
            })?;
            let model = normalize_family(doc).map_err(|d| {
                diag_err_mapped_label(
                    &raw,
                    source.source_span,
                    d,
                    diagnostics_output,
                    input_label.as_deref(),
                )
            })?;
            let warnings = normalized_warnings(&model);
            had_warnings |= !warnings.is_empty();
            if !cli.quiet {
                emit_warnings_for_model_label(
                    &model,
                    &raw,
                    source.source_span,
                    diagnostics_output,
                    input_label.as_deref(),
                );
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
                        diag_err_mapped_label(
                            &raw,
                            source.source_span,
                            d,
                            diagnostics_output,
                            input_label.as_deref(),
                        )
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
                        diag_err_mapped_label(
                            &raw,
                            source.source_span,
                            d,
                            diagnostics_output,
                            input_label.as_deref(),
                        )
                    })?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped_label(
                            &raw,
                            source.source_span,
                            d,
                            diagnostics_output,
                            input_label.as_deref(),
                        )
                    })?;
                    emit_warnings_for_model_label(
                        &model,
                        &raw,
                        source.source_span,
                        diagnostics_output,
                        input_label.as_deref(),
                    );
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
                        diag_err_mapped_label(
                            &raw,
                            source.source_span,
                            d,
                            diagnostics_output,
                            input_label.as_deref(),
                        )
                    })?;
                    let model = normalize_family(doc).map_err(|d| {
                        diag_err_mapped_label(
                            &raw,
                            source.source_span,
                            d,
                            diagnostics_output,
                            input_label.as_deref(),
                        )
                    })?;
                    emit_warnings_for_model_label(
                        &model,
                        &raw,
                        source.source_span,
                        diagnostics_output,
                        input_label.as_deref(),
                    );
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
            .map_err(|d| {
                diag_err_mapped_label(
                    &raw,
                    source.source_span,
                    d,
                    diagnostics_output,
                    input_label.as_deref(),
                )
            })?;
            let result = specialized::try_render_specialized(&preprocessed).ok_or_else(|| {
                (
                    EXIT_VALIDATION,
                    "[E_SPECIALIZED_PREPROC] preprocessed specialized source changed family"
                        .to_string(),
                )
            })?;
            let svg = result.map_err(|d| {
                diag_err_mapped_label(
                    &raw,
                    source.source_span,
                    d,
                    diagnostics_output,
                    input_label.as_deref(),
                )
            })?;
            let name_hint = source
                .output_name_hint
                .as_ref()
                .map(|base| format!("{base}.{}", cli.format.extension()));
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
        .map_err(|d| {
            diag_err_mapped_label(
                &raw,
                source.source_span,
                d,
                diagnostics_output,
                input_label.as_deref(),
            )
        })?;
        let model = normalize_family(doc).map_err(|d| {
            diag_err_mapped_label(
                &raw,
                source.source_span,
                d,
                diagnostics_output,
                input_label.as_deref(),
            )
        })?;
        emit_warnings_for_model_label(
            &model,
            &raw,
            source.source_span,
            diagnostics_output,
            input_label.as_deref(),
        );
        let pages = render_pages_from_model(&model, cli.format);
        let page_count = pages.len();
        for (page_idx, content) in pages.into_iter().enumerate() {
            let name_hint = source.output_name_hint.as_ref().map(|base| {
                if page_count == 1 {
                    format!("{base}.{}", cli.format.extension())
                } else {
                    format!("{base}-{}.{}", page_idx + 1, cli.format.extension())
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
                    cli.format.extension().to_uppercase()
                )
                    .to_string(),
            ));
        }
        let payload = outputs
            .iter()
            .enumerate()
            .map(|(idx, out)| MultiSvgOut {
                name: out
                    .name_hint
                    .clone()
                    .unwrap_or_else(|| format!("diagram-{}.{}", idx + 1, cli.format.extension())),
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
        .collect::<Result<Vec<_>, _>>()
        .map_err(output_err)?;

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
                                cli.format.extension().to_uppercase()
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

fn diag_err_with_source_label(
    source: &str,
    d: Diagnostic,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) -> (u8, String) {
    match output.format {
        DiagnosticsFormat::Human => (
            EXIT_VALIDATION,
            render_human_diagnostic_label(&d, source, output.color_enabled, file_label),
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

fn diag_err_mapped_label(
    raw_source: &str,
    mapping: Option<Span>,
    d: Diagnostic,
    output: DiagnosticOutput,
    file_label: Option<&str>,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source_label(raw_source, mapped, output, file_label)
}

fn emit_warnings_for_model(
    model: &NormalizedDocument,
    source: &str,
    mapping: Option<Span>,
    output: DiagnosticOutput,
) {
    emit_warnings_for_model_label(model, source, mapping, output, None);
}

fn emit_warnings_for_model_label(
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

fn render_pages_from_model(model: &NormalizedDocument, format: OutputFormat) -> Vec<String> {
    match format.text_mode() {
        Some(mode) => render::render_text_pages(model, mode),
        None => render_svg_pages_from_model(model)
            .into_iter()
            .map(|svg| render_svg_export_content(&svg, format))
            .collect(),
    }
}

fn output_err(error: OutputError) -> (u8, String) {
    let code = match error.kind() {
        OutputErrorKind::Validation => EXIT_VALIDATION,
        OutputErrorKind::Io => EXIT_IO,
        OutputErrorKind::Internal | OutputErrorKind::Unsupported => EXIT_INTERNAL,
    };
    (code, error.message().to_string())
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
    Ok(input.with_file_name(format!("{stem}.{}", format.extension())))
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
