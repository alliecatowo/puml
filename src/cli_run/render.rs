use super::{
    diagnostics::{
        diag_err_mapped_label, emit_diagnostics_label, emit_warnings_for_model_label,
        DiagnosticOutput,
    },
    input::InputDiagram,
    output::{
        default_output_base, output_err, render_pages_from_model, write_markdown_output_files,
        write_output_files, MultiSvgOut,
    },
    pipeline::{parse_for_cli_with_diagnostics, preprocess_for_cli},
    EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION,
};
use crate::cli::{Cli, OutputFormat};
use puml::output::{render_output_bytes, render_svg_export_content, RenderedOutput};
use puml::{normalize_family, specialized};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(super) fn run_render_mode(
    cli: &Cli,
    diagrams: &[InputDiagram],
    include_root: Option<PathBuf>,
    inject_vars: BTreeMap<String, String>,
    diagnostics_output: DiagnosticOutput,
    raw: &str,
    input_label: Option<&str>,
    input_path: Option<&Path>,
    from_markdown: bool,
) -> Result<(), (u8, String)> {
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
        let model = normalize_family(parse_result.document).map_err(|d| {
            diag_err_mapped_label(
                &raw,
                source.source_span,
                d,
                diagnostics_output,
                input_label.as_deref(),
            )
        })?;
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

    if let Some(path) = &cli.output {
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
