use super::*;

mod chronology;
mod dates;
mod details;
mod gantt;
mod rows;
mod scale;
mod util;

use chronology::*;
use dates::*;
use details::*;
use gantt::*;
use rows::*;
use scale::*;
use util::*;

pub fn render_timeline_stub_svg(document: &TimelineDocument) -> String {
    render_timeline_svg(document)
}

/// Render Gantt/Chronology timelines as real SVGs:
///   - Gantt: horizontal task bars on a date axis, milestone diamonds,
///     dashed arrows for `requires`/start/etc. constraints between bars.
///   - Chronology: vertical timeline with event bullets along a date axis.
pub fn render_timeline_svg(document: &TimelineDocument) -> String {
    match document.kind {
        DiagramKind::Chronology => render_chronology_svg(document),
        _ => render_gantt_svg(document),
    }
}
