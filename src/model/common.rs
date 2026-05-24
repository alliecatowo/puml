/// How to scale (or fix the size of) the output SVG.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleSpec {
    /// Multiply both width and height by this factor (e.g. `scale 1.5`).
    Factor(f64),
    /// Scale proportionally to this output width (e.g. `scale 200 width`).
    Width(u32),
    /// Scale proportionally to this output height (e.g. `scale 200 height`).
    Height(u32),
    /// Render to exactly this pixel size, preserving aspect via viewBox
    /// (e.g. `scale 800*600`).
    Fixed { width: u32, height: u32 },
    /// Cap the larger dimension at this pixel size (e.g. `scale max 800`).
    Max(u32),
    /// Cap the output width, preserving aspect (e.g. `scale max 800 width`).
    MaxWidth(u32),
    /// Cap the output height, preserving aspect (e.g. `scale max 600 height`).
    MaxHeight(u32),
    /// Fit output within this box, preserving aspect (e.g. `scale max 800*600`).
    MaxFixed { width: u32, height: u32 },
}

/// Horizontal positioning of the legend box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendHAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical positioning of the legend box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendVAlign {
    #[default]
    Bottom,
    Top,
}

/// Horizontal alignment for common header/footer metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetadataHAlign {
    #[default]
    Left,
    Center,
    Right,
}
