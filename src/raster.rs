/// Convert an SVG string to PNG bytes via resvg + tiny-skia.
///
/// Returns the raw PNG byte payload on success, or a human-readable error
/// string on failure.  All intermediate errors are mapped to `String` so
/// callers do not need to depend on resvg/usvg error types directly.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg.as_bytes(), &opts)
        .map_err(|e| format!("failed to parse SVG for rasterization: {e}"))?;

    let pixmap_size = tree.size().to_int_size();
    let mut pixmap =
        tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).ok_or_else(|| {
            format!(
                "failed to allocate pixmap ({}x{})",
                pixmap_size.width(),
                pixmap_size.height()
            )
        })?;

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("failed to encode PNG: {e}"))
}
