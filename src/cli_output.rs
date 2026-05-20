use crate::cli::OutputFormat;
use crate::cli_input::missing_markdown_output_name;
use crate::{EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION};
use image::ImageEncoder;
use puml::{render, try_render_svg_pages_from_model, NormalizedDocument, TextOutputMode};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct RenderedOutput {
    pub(crate) name_hint: Option<String>,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedBinaryOutput {
    pub(crate) name_hint: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn render_pages_from_model(
    model: &NormalizedDocument,
    format: OutputFormat,
) -> Result<Vec<String>, puml::Diagnostic> {
    match format.text_mode() {
        Some(mode) => Ok(render::render_text_pages(model, mode)),
        None => Ok(try_render_svg_pages_from_model(model)?
            .into_iter()
            .map(|svg| render_svg_export_content(&svg, format))
            .collect()),
    }
}

pub(crate) fn render_svg_export_content(svg: &str, format: OutputFormat) -> String {
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

pub(crate) fn default_output_base(
    input: &Path,
    format: OutputFormat,
) -> Result<PathBuf, (u8, String)> {
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

pub(crate) fn write_markdown_output_files(
    input: &Path,
    outputs: &[RenderedBinaryOutput],
) -> Result<(), (u8, String)> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::with_capacity(outputs.len());
    for (idx, out) in outputs.iter().enumerate() {
        let name = out
            .name_hint
            .as_ref()
            .ok_or_else(|| missing_markdown_output_name(idx))?;
        let path = parent.join(name);
        files.push((path, out.bytes.clone()));
    }
    write_files_transactionally(files)
}

pub(crate) fn write_output_files(base: &Path, payloads: &[Vec<u8>]) -> Result<(), (u8, String)> {
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

pub(crate) fn write_files_transactionally(
    files: Vec<(PathBuf, Vec<u8>)>,
) -> Result<(), (u8, String)> {
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

pub(crate) fn output_extension(format: OutputFormat) -> &'static str {
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
    pub(crate) fn uses_svg_renderer(self) -> bool {
        matches!(
            self,
            Self::Svg | Self::Html | Self::Png | Self::Jpg | Self::Webp | Self::Pdf
        )
    }

    pub(crate) fn is_binary(self) -> bool {
        matches!(self, Self::Png | Self::Jpg | Self::Webp | Self::Pdf)
    }

    pub(crate) fn is_text(self) -> bool {
        self.text_mode().is_some()
    }

    pub(crate) fn text_mode(self) -> Option<TextOutputMode> {
        match self {
            Self::Svg | Self::Html | Self::Png | Self::Jpg | Self::Webp | Self::Pdf => None,
            Self::Txt => Some(TextOutputMode::Txt),
            Self::Atxt => Some(TextOutputMode::Atxt),
            Self::Utxt => Some(TextOutputMode::Utxt),
        }
    }
}

pub(crate) fn render_output_bytes(
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
