use super::{
    diagnostics::{
        diag_err_mapped_label, emit_diagnostics_label, emit_hint_for_message,
        emit_warnings_for_model_label, DiagnosticOutput,
    },
    input::InputDiagram,
    output::{
        default_output_base, output_err, render_pages_from_model, write_markdown_output_files,
        write_output_files, MultiSvgOut,
    },
    pipeline::{normalize_for_cli, parse_for_cli_with_diagnostics, preprocess_for_cli},
    EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION,
};
use crate::cli::{Cli, OutputFormat, StyleMode};
use puml::model::FamilyStyle;
use puml::output::{
    render_artifact_export_content, render_artifact_output_bytes, RenderedArtifactOutput,
};
use puml::specialized;
use puml::theme::StyleMode as ThemeStyleMode;
use puml::NormalizedDocument;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Apply the CLI `--style` mode to the normalized model's class/component style.
///
/// When `style == StyleMode::Plantuml`, the `ClassStyle.style_mode` on every
/// `Family` / `FamilyPages` variant is set to `ThemeStyleMode::Plantuml` so the
/// renderer suppresses PUML-enhanced chrome (yellow object header, type badges,
/// UML 2.x visibility glyphs). Layout coordinates are never touched.
fn apply_style_mode(model: &mut NormalizedDocument, style: StyleMode) {
    if style == StyleMode::Puml {
        return; // default — nothing to override
    }
    let theme_mode = ThemeStyleMode::Plantuml;
    match model {
        NormalizedDocument::Family(doc) => {
            if let Some(FamilyStyle::Class(ref mut cs)) = doc.family_style {
                cs.style_mode = theme_mode;
            } else if doc.family_style.is_none() {
                // Inject a default ClassStyle with the requested mode so the renderer
                // picks it up even when no skinparam block was present.
                let cs = puml::theme::ClassStyle {
                    style_mode: theme_mode,
                    ..Default::default()
                };
                doc.family_style = Some(FamilyStyle::Class(cs));
            }
        }
        NormalizedDocument::FamilyPages(pages) => {
            for doc in pages.iter_mut() {
                if let Some(FamilyStyle::Class(ref mut cs)) = doc.family_style {
                    cs.style_mode = theme_mode;
                } else if doc.family_style.is_none() {
                    let cs = puml::theme::ClassStyle {
                        style_mode: theme_mode,
                        ..Default::default()
                    };
                    doc.family_style = Some(FamilyStyle::Class(cs));
                }
            }
        }
        _ => {} // other families don't yet carry chrome that needs mode-gating
    }
}

// Rendering crosses the CLI/input/output boundary, so this helper keeps the
// distinct command state explicit instead of hiding it behind a transient bag.
#[allow(clippy::too_many_arguments)]
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
    if cli.verbose {
        eprintln!(
            "[verbose] rendering {} diagram source(s) as .{} with {}",
            diagrams.len(),
            cli.format.extension(),
            pluralize_threads(cli.threads)
        );
    }
    let n = diagrams.len();
    let emit_progress = !cli.quiet && (n > 1 || from_markdown || cli.verbose);
    let outputs = diagrams
        .iter()
        .enumerate()
        .try_fold(Vec::new(), |mut all, (idx, source)| {
            if emit_progress {
                let label = source
                    .output_name_hint
                    .as_deref()
                    .or(input_label)
                    .unwrap_or("<stdin>");
                eprintln!("[{}/{}] rendering {}...", idx + 1, n, label);
            }
            // Short-circuit for specialized families (math, ditaa, etc.) after the
            // same preprocessor pass used by check/dump routes.
            // Text modes intentionally route through normalized models instead.
            if cli.format.uses_svg_renderer() && specialized::is_specialized_source(&source.source)
            {
                let preprocessed = preprocess_for_cli(
                    &source.source,
                    include_root.clone(),
                    cli.dialect,
                    cli.compat,
                    source.frontend_hint,
                    cli.allow_url_includes,
                    inject_vars.clone(),
                )
                .map_err(|d| {
                    diag_err_mapped_label(
                        raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label,
                    )
                })?;
                let result =
                    specialized::try_render_specialized(&preprocessed).ok_or_else(|| {
                        (
                    EXIT_VALIDATION,
                    "[E_SPECIALIZED_PREPROC] preprocessed specialized source changed family"
                        .to_string(),
                )
                    })?;
                let svg = result.map_err(|d| {
                    diag_err_mapped_label(
                        raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label,
                    )
                })?;
                let name_hint = source
                    .output_name_hint
                    .as_ref()
                    .map(|base| format!("{base}.{}", cli.format.extension()));
                let artifact = puml::render::RenderArtifact::svg_only(svg);
                all.push(RenderedArtifactOutput {
                    name_hint,
                    content: render_artifact_export_content(&artifact, cli.format),
                    artifact: Some(puml::output::RenderArtifactOutputMetadata::from_artifact(
                        &artifact,
                    )),
                });
                return Ok(all);
            }
            let parse_result = parse_for_cli_with_diagnostics(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                source.frontend_hint,
                cli.allow_url_includes,
                inject_vars.clone(),
            )
            .map_err(|d| {
                diag_err_mapped_label(raw, source.source_span, d, diagnostics_output, input_label)
            })?;
            let mut model = normalize_for_cli(parse_result.document, include_root.clone())
                .map_err(|d| {
                    diag_err_mapped_label(
                        raw,
                        source.source_span,
                        d,
                        diagnostics_output,
                        input_label,
                    )
                })?;
            // Apply chrome style mode before rendering — only paint is affected,
            // layout coordinates are never mutated.
            apply_style_mode(&mut model, cli.style);
            emit_diagnostics_label(
                &parse_result.diagnostics,
                raw,
                source.source_span,
                diagnostics_output,
                input_label,
            );
            emit_warnings_for_model_label(
                &model,
                raw,
                source.source_span,
                diagnostics_output,
                input_label,
            );
            let pages = render_pages_from_model(&model, cli.format);
            let page_count = pages.len();
            for (page_idx, mut output) in pages.into_iter().enumerate() {
                output.name_hint = source.output_name_hint.as_ref().map(|base| {
                    if page_count == 1 {
                        format!("{base}.{}", cli.format.extension())
                    } else {
                        format!("{base}-{}.{}", page_idx + 1, cli.format.extension())
                    }
                });
                all.push(output);
            }
            Ok::<_, (u8, String)>(all)
        })?;
    if cli.verbose {
        eprintln!("[verbose] rendered {} output artifact(s)", outputs.len());
    }

    if input_path.is_none() && outputs.len() > 1 && !cli.multi {
        let msg = "multiple pages detected; rerun with --multi";
        emit_hint_for_message(msg, diagnostics_output);
        return Err((EXIT_VALIDATION, msg.to_string()));
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
        .map(|out| render_artifact_output_bytes(out, cli.format, cli.dpi))
        .collect::<Result<Vec<_>, _>>()
        .map_err(output_err)?;

    if let Some(path) = &cli.output {
        let payloads = binary_outputs
            .iter()
            .map(|out| out.bytes.clone())
            .collect::<Vec<_>>();
        write_output_files(path, &payloads)?;
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

fn pluralize_threads(threads: usize) -> String {
    if threads == 1 {
        "1 thread hint".to_string()
    } else {
        format!("{threads} thread hints")
    }
}
