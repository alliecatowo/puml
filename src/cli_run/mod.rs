mod diagnostics;
mod encodesprite;
mod format;
mod input;
mod lint;
mod output;
mod pipeline;
mod render;

use crate::cli::{Cli, Command as CliCommand, DiagnosticsFormat, DumpKind};
use crate::cli_dump::{normalized_model_to_json, normalized_scene_to_json};
use crate::cli_dump_ast::ast_to_json;
use crate::{cli_count, cli_env, cli_hash, cli_stats, cli_watch};
use diagnostics::{
    diag_err_mapped_label, diag_err_with_source_label, emit_diagnostics_label,
    emit_warnings_for_model_label, should_color_human_diagnostics, DiagnosticOutput,
};
use encodesprite::run_encodesprite;
use format::run_format_command;
use input::{frontend_hint_for_path, read_input, should_extract_markdown, split_diagrams};
use lint::{is_lint_mode_enabled, run_lint_mode, run_lint_subcommand, LintSubcommandContext};
use pipeline::{
    normalize_for_cli, parse_for_cli, parse_for_cli_with_diagnostics, preprocess_for_cli,
};
use puml::diagnostic::normalized_warnings;
use puml::extract_metadata;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

const EXIT_VALIDATION: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INTERNAL: u8 = 3;
const SUPPORTED_OUTPUT_FORMATS: &str = "svg, html, png, jpg, webp, pdf, txt, atxt, utxt";
const SUPPORTED_MARKDOWN_FENCES: &str =
    "puml, pumlx, picouml, plantuml, uml, puml-sequence, uml-sequence, mermaid";

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

    if let Some(format) = &cli.unsupported_output_format {
        return Err((
            EXIT_IO,
            format!(
                "[E_OUTPUT_FORMAT_UNSUPPORTED] unsupported output format `{format}`; supported formats: {SUPPORTED_OUTPUT_FORMATS}"
            ),
        ));
    }
    if cli.extract {
        return Err((
            EXIT_IO,
            "[E_FLAG_UNSUPPORTED] --extract is parsed for PlantUML CLI parity but is not implemented; render multi-diagram inputs normally or use --from-markdown for fenced extraction"
                .to_string(),
        ));
    }
    if let Some(pattern) = &cli.pattern {
        return Err((
            EXIT_IO,
            format!(
                "[E_FLAG_UNSUPPORTED] --pattern is parsed for PlantUML CLI parity but is not implemented; received pattern `{pattern}`"
            ),
        ));
    }

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
        let parse_result = parse_for_cli_with_diagnostics(
            &src,
            include_root.clone(),
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
        let model = normalize_for_cli(parse_result.document, include_root).map_err(|d| {
            diag_err_with_source_label(&src, d, diagnostics_output, Some(&fixture_label))
        })?;
        emit_diagnostics_label(
            &parse_result.diagnostics,
            &src,
            None,
            diagnostics_output,
            Some(&fixture_label),
        );
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
                let model = normalize_for_cli(doc, include_root.clone()).map_err(|d| {
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
            let parse_result = parse_for_cli_with_diagnostics(
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
            let model =
                normalize_for_cli(parse_result.document, include_root.clone()).map_err(|d| {
                    diag_err_mapped_label(
                        &raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label.as_deref(),
                    )
                })?;
            had_warnings |= !parse_result.diagnostics.is_empty();
            let warnings = normalized_warnings(&model);
            had_warnings |= !warnings.is_empty();
            if !cli.quiet {
                emit_diagnostics_label(
                    &parse_result.diagnostics,
                    &raw,
                    source.source_span,
                    diagnostics_output,
                    input_label.as_deref(),
                );
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
                    let model = normalize_for_cli(doc, include_root.clone()).map_err(|d| {
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
                    let model = normalize_for_cli(doc, include_root.clone()).map_err(|d| {
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

    render::run_render_mode(
        &cli,
        &diagrams,
        include_root,
        inject_vars,
        diagnostics_output,
        &raw,
        input_label.as_deref(),
        input_path,
        from_markdown,
    )
}

fn lsp_capabilities_manifest() -> Value {
    puml::lsp_capabilities()
}
