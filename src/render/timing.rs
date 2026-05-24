use super::*;

mod axes;
mod messages;
mod model;
mod rows;
mod svg_emit;

use axes::render_timing_axis;
use messages::render_timing_relations;
use model::{TimingLayout, TimingModel};
use rows::{render_timing_rows, signal_row_midpoints};
use svg_emit::{render_timing_footer_caption, render_timing_svg_header};

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    let default_timing_style;
    let style = match &doc.family_style {
        Some(crate::model::FamilyStyle::Timing(style)) => style,
        _ => {
            default_timing_style = crate::theme::TimingStyle::default();
            &default_timing_style
        }
    };

    let model = TimingModel::from_document(doc);
    let layout = TimingLayout::new(doc, &model, style);

    let mut out = render_timing_svg_header(doc, style, &layout);

    render_timing_axis(&mut out, &model, &layout, style);

    let signal_row_mid = signal_row_midpoints(&model.signals, &layout);
    render_timing_rows(&mut out, &model, &layout, style);

    render_timing_relations(
        &mut out,
        doc,
        &signal_row_mid,
        layout.axis_top,
        layout.signals_top + layout.rows_h(),
        &|time| layout.time_to_x(time),
        style,
    );

    render_timing_footer_caption(&mut out, doc, style, &layout);

    out.push_str("</svg>");
    out
}
