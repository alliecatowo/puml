use super::*;

pub(super) fn render_nwdiag_node_label(out: &mut String, x: i32, y: i32, width: i32, label: &str) {
    let extra_attrs =
        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\"";
    if label_contains_inline_sprite(label) {
        out.push_str(&super::super::svg::creole_text(
            x + 10,
            y + 18,
            "font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\"",
            label,
            "#0f172a",
        ));
    } else {
        out.push_str(&super::super::svg::creole_text(
            x + (width / 2),
            y + 18,
            extra_attrs,
            label,
            "#0f172a",
        ));
    }
}

pub(super) fn nwdiag_stroke_dash(style: &str) -> &'static str {
    if style
        .split([',', ' '])
        .any(|part| part.eq_ignore_ascii_case("dotted"))
    {
        " stroke-dasharray=\"1 3\""
    } else if style
        .split([',', ' '])
        .any(|part| part.eq_ignore_ascii_case("dashed"))
    {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}
