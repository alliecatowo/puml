#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFormat {
    Svg,
    Html,
    Png,
    Jpg,
    Webp,
    Pdf,
}

impl BackendFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Html => "html",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Webp => "webp",
            Self::Pdf => "pdf",
        }
    }

    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Svg => "image/svg+xml",
            Self::Html => "text/html",
            Self::Png => "image/png",
            Self::Jpg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Pdf => "application/pdf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendCapability {
    VectorOutput,
    HtmlExport,
    RasterExport,
    PdfExport,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub primary_format: BackendFormat,
    pub export_formats: &'static [BackendFormat],
    pub capabilities: &'static [BackendCapability],
}

impl BackendDescriptor {
    pub fn supports_format(self, format: BackendFormat) -> bool {
        self.primary_format == format || self.export_formats.contains(&format)
    }

    pub fn has_capability(self, capability: BackendCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

pub trait RenderBackend {
    fn descriptor(&self) -> &'static BackendDescriptor;

    fn supports_format(&self, format: BackendFormat) -> bool {
        self.descriptor().supports_format(format)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SvgBackend;

pub static SVG_BACKEND_DESCRIPTOR: BackendDescriptor = BackendDescriptor {
    id: "svg",
    display_name: "SVG backend",
    primary_format: BackendFormat::Svg,
    export_formats: &[
        BackendFormat::Html,
        BackendFormat::Png,
        BackendFormat::Jpg,
        BackendFormat::Webp,
        BackendFormat::Pdf,
    ],
    capabilities: &[
        BackendCapability::VectorOutput,
        BackendCapability::HtmlExport,
        BackendCapability::RasterExport,
        BackendCapability::PdfExport,
        BackendCapability::Metadata,
    ],
};

impl RenderBackend for SvgBackend {
    fn descriptor(&self) -> &'static BackendDescriptor {
        &SVG_BACKEND_DESCRIPTOR
    }
}
