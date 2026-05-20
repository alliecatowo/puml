mod cli;
mod cli_diagnostics;
mod cli_dump;
mod cli_format;
mod cli_input;
mod cli_lint;
mod cli_output;

use clap::{CommandFactory, FromArgMatches};
use cli::{
    Cli, ColorChoice as CliColorChoice, Command as CliCommand, CompatMode as CliCompatMode,
    DeterminismMode as CliDeterminismMode, DiagnosticsFormat, Dialect as CliDialect, DumpKind,
    OutputFormat,
};
use cli_diagnostics::{
    diag_err_mapped, diag_err_with_source, emit_warnings_for_model, lsp_capabilities_manifest,
    normalized_warnings, should_color_human_diagnostics, DiagnosticOutput,
};
use cli_dump::{ast_to_json, normalized_model_to_json, normalized_scene_to_json};
use cli_format::run_format_command;
use cli_input::{
    frontend_hint_for_path, read_input, should_extract_markdown, split_diagrams,
    SUPPORTED_MARKDOWN_FENCES,
};
use cli_lint::{is_lint_mode_enabled, run_lint_mode};
use cli_output::{
    default_output_base, output_extension, render_output_bytes, render_pages_from_model,
    render_svg_export_content, write_markdown_output_files, write_output_files, RenderedOutput,
};
use puml::ast::Document;
use puml::{
    extract_metadata, normalize_family, preprocess_with_pipeline_options, render, specialized,
    CompatMode, DeterminismMode, Diagnostic, FrontendSelection, ParsePipelineOptions,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
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
            _ => expanded.push(arg),
        }
    }
    expanded
}

fn clap_color_choice_from_args(args: &[OsString]) -> clap::ColorChoice {
    match color_choice_from_args(args) {
        CliColorChoice::Always => clap::ColorChoice::Always,
        CliColorChoice::Never => clap::ColorChoice::Never,
        CliColorChoice::Auto => clap::ColorChoice::Auto,
    }
}

fn color_choice_from_args(args: &[OsString]) -> CliColorChoice {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(raw) = arg.to_str() else {
            continue;
        };
        if let Some(value) = raw.strip_prefix("--color=") {
            return parse_color_choice(value).unwrap_or(CliColorChoice::Auto);
        }
        if raw == "--color" {
            return iter
                .next()
                .and_then(|value| value.to_str())
                .and_then(parse_color_choice)
                .unwrap_or(CliColorChoice::Auto);
        }
    }
    CliColorChoice::Auto
}

fn parse_color_choice(raw: &str) -> Option<CliColorChoice> {
    match raw {
        "auto" => Some(CliColorChoice::Auto),
        "always" => Some(CliColorChoice::Always),
        "never" => Some(CliColorChoice::Never),
        _ => None,
    }
}

fn run(mut cli: Cli) -> Result<(), (u8, String)> {
    let started = Instant::now();
    if let Some(command) = cli.command.take() {
        return match command {
            CliCommand::Format(args) => run_format_command(args),
        };
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

    let (_input_name, raw, input_path) = read_input(cli.input.as_deref())?;
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

    if cli.check {
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
            let svg = render::validate_svg(svg, render::RenderProfile::Legacy)
                .map(render::ValidatedSvg::into_string)
                .map_err(|err| {
                    diag_err_mapped(
                        &raw,
                        source.source_span,
                        Diagnostic::error_code("E_RENDER_CONTRACT", err.to_string()),
                        diagnostics_output,
                    )
                })?;
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
        let pages = render_pages_from_model(&model, cli.format)
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, diagnostics_output))?;
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
