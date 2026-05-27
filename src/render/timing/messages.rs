use super::model::timing_relation_endpoint;
use super::*;
use std::collections::BTreeMap;

pub(super) fn render_timing_relations(
    out: &mut String,
    doc: &FamilyDocument,
    signal_row_mid: &BTreeMap<String, i32>,
    axis_top: i32,
    chart_bottom: i32,
    time_to_x: &dyn Fn(i64) -> i32,
    style: &crate::theme::TimingStyle,
) {
    for relation in &doc.relations {
        let Some((from_signal, from_time)) = timing_relation_endpoint(&relation.from) else {
            continue;
        };
        let Some((to_signal, to_time)) = timing_relation_endpoint(&relation.to) else {
            continue;
        };
        let from_lookup = from_signal.to_ascii_lowercase();
        let to_lookup = to_signal.to_ascii_lowercase();
        let Some(&y1) = signal_row_mid.get(&from_lookup) else {
            continue;
        };
        let Some(&y2) = signal_row_mid.get(&to_lookup) else {
            continue;
        };
        let x1 = time_to_x(from_time);
        let x2 = time_to_x(to_time);
        let lane_inset = 16;
        let (y1, y2) = if y2 > y1 {
            (y1 + lane_inset, y2 - lane_inset)
        } else if y2 < y1 {
            (y1 - lane_inset, y2 + lane_inset)
        } else {
            (y1, y2)
        };
        let color = relation.line_color.as_deref().unwrap_or(&style.arrow_color);
        let dash = if relation.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line class=\"timing-message\" data-from=\"{}\" data-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1.6\"{dash}/>",
            escape_text(from_signal),
            escape_text(to_signal),
            escape_text(color)
        ));
        let head = if x2 >= x1 {
            format!("{},{} {},{} {},{}", x2, y2, x2 - 8, y2 - 5, x2 - 8, y2 + 5)
        } else {
            format!("{},{} {},{} {},{}", x2, y2, x2 + 8, y2 - 5, x2 + 8, y2 + 5)
        };
        out.push_str(&format!(
            "<polygon class=\"timing-message-head\" points=\"{head}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            escape_text(color),
            escape_text(color)
        ));
        if let Some(label) = relation.label.as_deref().filter(|label| !label.is_empty()) {
            let lx = (x1 + x2) / 2;
            // Nudge the label 10 px above the arrow midpoint so it does not sit on a lane
            // border stroke.  A semi-transparent background rect makes it legible even when
            // the arrow crosses a state-block edge.
            let ly = ((y1 + y2) / 2 - 10).clamp(axis_top + 12, chart_bottom - 6);
            let label_half_w = (label.len() as i32 * 6 / 2 + 4).max(12);
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"13\" fill=\"white\" fill-opacity=\"0.85\" rx=\"2\"/>",
                lx - label_half_w,
                ly - 10,
                label_half_w * 2
            ));
            out.push_str(&format!(
                "<text class=\"timing-message-label\" x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                escape_text(&style.font_color),
                escape_text(label)
            ));
        }
    }
}
