use crate::render::TextOutputMode;
use crate::render_core::{BackendFormat, RenderBackend, SvgBackend};

#[cfg(feature = "cli")]
use image::ImageEncoder as _;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum OutputFormat {
    Svg,
    Html,
    Png,
    Jpg,
    Webp,
    /// PDF output via SVG-to-PDF vector conversion.
    Pdf,
    Txt,
    Atxt,
    Utxt,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Html => "html",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Webp => "webp",
            Self::Pdf => "pdf",
            Self::Txt => "txt",
            Self::Atxt => "atxt",
            Self::Utxt => "utxt",
        }
    }

    pub fn uses_svg_renderer(self) -> bool {
        self.backend_format()
            .is_some_and(|format| SvgBackend.supports_format(format))
    }

    pub fn backend_format(self) -> Option<BackendFormat> {
        match self {
            Self::Svg => Some(BackendFormat::Svg),
            Self::Html => Some(BackendFormat::Html),
            Self::Png => Some(BackendFormat::Png),
            Self::Jpg => Some(BackendFormat::Jpg),
            Self::Webp => Some(BackendFormat::Webp),
            Self::Pdf => Some(BackendFormat::Pdf),
            Self::Txt | Self::Atxt | Self::Utxt => None,
        }
    }

    pub fn is_binary(self) -> bool {
        matches!(self, Self::Png | Self::Jpg | Self::Webp | Self::Pdf)
    }

    pub fn is_text(self) -> bool {
        self.text_mode().is_some()
    }

    pub fn text_mode(self) -> Option<TextOutputMode> {
        match self {
            Self::Svg | Self::Html | Self::Png | Self::Jpg | Self::Webp | Self::Pdf => None,
            Self::Txt => Some(TextOutputMode::Txt),
            Self::Atxt => Some(TextOutputMode::Atxt),
            Self::Utxt => Some(TextOutputMode::Utxt),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderedOutput {
    pub name_hint: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct RenderedBinaryOutput {
    pub name_hint: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputErrorKind {
    Validation,
    Io,
    Internal,
    Unsupported,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OutputError {
    kind: OutputErrorKind,
    message: String,
}

impl OutputError {
    pub fn new(kind: OutputErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> OutputErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for OutputError {}

pub fn render_svg_export_content(svg: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Html => svg_to_html_document(svg),
        _ => svg.to_string(),
    }
}

pub fn svg_to_html_document(svg: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>puml diagram</title>\n<style>html,body{{margin:0;min-height:100%;background:#fff;}}body{{display:flex;align-items:flex-start;justify-content:center;padding:16px;box-sizing:border-box;}}svg{{max-width:100%;height:auto;}}</style>\n</head>\n<body>\n{svg}\n</body>\n</html>"
    )
}

#[cfg(feature = "cli")]
pub fn render_output_bytes(
    output: &RenderedOutput,
    format: OutputFormat,
    dpi: f32,
) -> Result<RenderedBinaryOutput, OutputError> {
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

#[cfg(feature = "cli")]
pub fn svg_to_raster_bytes(
    svg: &str,
    format: OutputFormat,
    dpi: f32,
) -> Result<Vec<u8>, OutputError> {
    let raster = rasterize_svg(svg, dpi)?;
    match format {
        OutputFormat::Png => encode_png(&raster),
        OutputFormat::Jpg => encode_jpg(&raster),
        OutputFormat::Webp => encode_webp(&raster),
        _ => Err(OutputError::new(
            OutputErrorKind::Internal,
            format!(
                "format '{}' does not use SVG raster export",
                format.extension()
            ),
        )),
    }
}

#[cfg(feature = "cli")]
pub fn svg_to_pdf_bytes(svg: &str) -> Result<Vec<u8>, OutputError> {
    let mut opt = svg2pdf::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = svg2pdf::usvg::Tree::from_str(svg, &opt).map_err(|e| {
        OutputError::new(
            OutputErrorKind::Validation,
            format!("failed to parse rendered SVG for PDF output: {e}"),
        )
    })?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| {
        OutputError::new(
            OutputErrorKind::Internal,
            format!("failed to convert SVG to PDF: {e}"),
        )
    })
}

#[cfg(feature = "cli")]
struct RasterizedSvg {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[cfg(feature = "cli")]
fn rasterize_svg(svg: &str, dpi: f32) -> Result<RasterizedSvg, OutputError> {
    let mut opt = resvg::usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();
    fontdb.set_monospace_family("Liberation Mono");
    let tree = resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| {
        OutputError::new(
            OutputErrorKind::Validation,
            format!("failed to parse rendered SVG for PNG output: {e}"),
        )
    })?;

    let size = tree.size();
    let scale = dpi / 96.0;
    let width = (size.width() * scale).round() as u32;
    let height = (size.height() * scale).round() as u32;
    if width == 0 || height == 0 {
        return Err(OutputError::new(
            OutputErrorKind::Internal,
            "failed to rasterize PNG: computed zero-sized output",
        ));
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        OutputError::new(
            OutputErrorKind::Internal,
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

#[cfg(feature = "cli")]
fn encode_png(raster: &RasterizedSvg) -> Result<Vec<u8>, OutputError> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            &raster.rgba,
            raster.width,
            raster.height,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| OutputError::new(OutputErrorKind::Io, format!("failed to encode PNG: {e}")))?;
    Ok(png)
}

#[cfg(feature = "cli")]
fn encode_jpg(raster: &RasterizedSvg) -> Result<Vec<u8>, OutputError> {
    let rgb = rgba_to_rgb_over_white(&raster.rgba);
    let mut jpg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg, 90)
        .write_image(
            &rgb,
            raster.width,
            raster.height,
            image::ColorType::Rgb8.into(),
        )
        .map_err(|e| OutputError::new(OutputErrorKind::Io, format!("failed to encode JPG: {e}")))?;
    Ok(jpg)
}

#[cfg(feature = "cli")]
fn encode_webp(raster: &RasterizedSvg) -> Result<Vec<u8>, OutputError> {
    let mut webp = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
        .write_image(
            &raster.rgba,
            raster.width,
            raster.height,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| {
            OutputError::new(OutputErrorKind::Io, format!("failed to encode WebP: {e}"))
        })?;
    Ok(webp)
}

#[cfg(feature = "cli")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_metadata_matches_cli_extensions_and_modes() {
        assert_eq!(OutputFormat::Svg.extension(), "svg");
        assert_eq!(OutputFormat::Html.extension(), "html");
        assert_eq!(OutputFormat::Png.extension(), "png");
        assert_eq!(OutputFormat::Jpg.extension(), "jpg");
        assert_eq!(OutputFormat::Webp.extension(), "webp");
        assert_eq!(OutputFormat::Pdf.extension(), "pdf");
        assert_eq!(OutputFormat::Txt.extension(), "txt");
        assert_eq!(OutputFormat::Atxt.extension(), "atxt");
        assert_eq!(OutputFormat::Utxt.extension(), "utxt");

        assert!(OutputFormat::Png.uses_svg_renderer());
        assert_eq!(OutputFormat::Png.backend_format(), Some(BackendFormat::Png));
        assert_eq!(OutputFormat::Txt.backend_format(), None);
        assert!(OutputFormat::Pdf.is_binary());
        assert!(OutputFormat::Txt.is_text());
        assert_eq!(OutputFormat::Utxt.text_mode(), Some(TextOutputMode::Utxt));
    }

    #[test]
    fn svg_export_content_wraps_html_only() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>";
        assert_eq!(render_svg_export_content(svg, OutputFormat::Svg), svg);
        let html = render_svg_export_content(svg, OutputFormat::Html);
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains(svg));
    }

    #[cfg(feature = "cli")]
    #[test]
    fn raster_output_encodes_supported_image_formats() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">
  <rect width="2" height="2" fill="#ffffff"/>
  <rect width="1" height="1" fill="#000000"/>
</svg>"##;

        let png = svg_to_raster_bytes(svg, OutputFormat::Png, 96.0).expect("png bytes");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));

        let jpg = svg_to_raster_bytes(svg, OutputFormat::Jpg, 96.0).expect("jpg bytes");
        assert!(jpg.starts_with(&[0xff, 0xd8]));

        let webp = svg_to_raster_bytes(svg, OutputFormat::Webp, 96.0).expect("webp bytes");
        assert!(webp.starts_with(b"RIFF"));
        assert_eq!(&webp[8..12], b"WEBP");
    }

    #[cfg(feature = "cli")]
    #[test]
    fn raster_output_rejects_degenerate_dimensions() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="6">
  <rect x="0" y="0" width="8" height="6" fill="#fff"/>
</svg>"##;

        let err = svg_to_raster_bytes(svg, OutputFormat::Png, 0.0).expect_err("degenerate output");
        assert_eq!(err.kind(), OutputErrorKind::Internal);
        assert_eq!(
            err.message(),
            "failed to rasterize PNG: computed zero-sized output"
        );
    }
}
