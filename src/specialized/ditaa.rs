// Family: @startditaa
//
// Ditaa is a raw ASCII-art renderer. Keep the grid model, detection passes, and
// SVG emission separate so the specialized raw-source path and normalized-model
// path do not need source reconstruction shims.

mod connectors;
mod detect;
mod emit;
mod model;

use super::shared::strip_block;
use crate::diagnostic::Diagnostic;

use connectors::detect_connectors;
use detect::detect_shapes;
use emit::emit_svg;
use model::{DitaaGrid, DitaaOptions};

/// Render a ditaa diagram directly from its body and optional title, without
/// reconstructing the `@startditaa...@endditaa` wrapper. Called from the model
/// render path (`render::specialized::ditaa`) so the LSP and CLI share the same
/// rendering logic without the two-hop reconstruct round-trip.
/// Ditaa options (scale, transparent, shadow, background) are not preserved in
/// the normalized model, so default options are used — matching the behavior of
/// the previous reconstruct shim which also lost those options.
pub(crate) fn render_ditaa_from_parts(
    body: &str,
    title: Option<&str>,
) -> Result<String, Diagnostic> {
    render_ditaa_inner(body, title, &DitaaOptions::default())
}

pub(super) fn render_ditaa(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startditaa", "@endditaa");
    let options = DitaaOptions::parse(source.lines().next().unwrap_or(""));
    render_ditaa_inner(body, title.as_deref(), &options)
}

fn render_ditaa_inner(
    body: &str,
    title: Option<&str>,
    options: &DitaaOptions,
) -> Result<String, Diagnostic> {
    if body.trim().is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa body is empty",
        ));
    }

    let grid = DitaaGrid::new(body, options.scale, title.is_some());
    if grid.rows == 0 {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa has no grid content",
        ));
    }

    let shapes = detect_shapes(&grid);
    let connectors = detect_connectors(&grid, &shapes);
    Ok(emit_svg(&grid, title, options, &shapes, &connectors))
}
