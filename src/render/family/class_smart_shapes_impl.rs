//! SVG shape renderers for individual DDD / architectural stereotypes (#1285).
//!
//! Each function draws one specialised shape into `out`.
//! Called exclusively from `class_smart_shapes.rs`.

use crate::render::svg::escape_text;

/// Helper: render the guillemet stereotype label(s) above the node name, and then
/// the node name itself, centred within a pre-drawn shape.
// All parameters are distinct scalar render properties; no logical grouping
// reduces the count without introducing an artificial struct — false positive.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_smart_shape_labels(
    out: &mut String,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    cx: i32,
    label_y: i32,
    name_y: i32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    if !hide_stereotype {
        out.push_str(&format!(
            "<text class=\"uml-stereotype\" x=\"{cx}\" y=\"{label_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
            ff = escape_text(font_family),
            fc = escape_text(font_color),
            lbl = escape_text(stereotype_label),
        ));
        for (i, extra) in extra_labels.iter().enumerate() {
            let extra_y = label_y + (i as i32 + 1) * 12;
            out.push_str(&format!(
                "<text class=\"uml-stereotype\" x=\"{cx}\" y=\"{extra_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
                ff = escape_text(font_family),
                fc = escape_text(font_color),
                lbl = escape_text(extra),
            ));
        }
    }
    out.push_str(&format!(
        "<text class=\"uml-node-name\" x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\">{name}</text>",
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(font_color),
        name = escape_text(display_name),
    ));
}

/// Render a flat-top hexagon node (used by `<<controller>>` and `<<value>>`).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_hexagon_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let indent = (w / 5).max(8);
    let cx = x + w / 2;
    let points = format!(
        "{},{} {},{} {},{} {},{} {},{} {},{}",
        x + indent,
        y,
        x + w - indent,
        y,
        x + w,
        y + h / 2,
        x + w - indent,
        y + h,
        x + indent,
        y + h,
        x,
        y + h / 2,
    );
    out.push_str(&format!(
        "<polygon class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" points=\"{pts}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        pts = points,
        sw = stroke_width,
    ));
    let header_h = (h * 3 / 10).max(22);
    let hy = y + header_h;
    let t_top = (hy - y) as f64 / (h as f64 / 2.0);
    let left_x_at_hy = if t_top <= 1.0 {
        x as f64 + indent as f64 * (1.0 - t_top)
    } else {
        let t_bot = t_top - 1.0;
        x as f64 + indent as f64 * t_bot
    };
    let right_x_at_hy = if t_top <= 1.0 {
        (x + w) as f64 - indent as f64 * (1.0 - t_top)
    } else {
        let t_bot = t_top - 1.0;
        (x + w) as f64 - indent as f64 * t_bot
    };
    let hx_l = left_x_at_hy.round() as i32;
    let hx_r = right_x_at_hy.round() as i32;
    let header_pts = format!(
        "{},{} {},{} {},{} {},{}",
        x + indent,
        y,
        x + w - indent,
        y,
        hx_r,
        hy,
        hx_l,
        hy,
    );
    out.push_str(&format!(
        "<polygon class=\"{css_class}-header\" points=\"{pts}\" fill=\"{hf}\" stroke=\"none\"/>",
        pts = header_pts,
        hf = header_fill,
    ));
    out.push_str(&format!(
        "<line x1=\"{hx_l}\" y1=\"{hy}\" x2=\"{hx_r}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
    ));
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a pill (heavily-rounded rectangle) node — used by `<<service>>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_pill_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let r = h / 2;
    let cx = x + w / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" ry=\"{r}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    let header_h = (h * 3 / 10).max(22);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" rx=\"{r}\" ry=\"{r}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    let sq_y = y + header_h - r;
    if sq_y > y {
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{sq_y}\" width=\"{w}\" height=\"{r}\" fill=\"{hf}\" stroke=\"none\"/>",
            hf = header_fill,
        ));
    }
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a cylinder node — used by `<<repository>>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_cylinder_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let ell_ry = (h / 8).max(6);
    let cx = x + w / 2;
    let cy_top = y + ell_ry;
    let cy_bot = y + h - ell_ry;
    let rx = w / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{cy_top}\" width=\"{w}\" height=\"{body_h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        body_h = cy_bot - cy_top,
        sw = stroke_width,
    ));
    out.push_str(&format!(
        "<ellipse cx=\"{cx}\" cy=\"{cy_bot}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        sw = stroke_width,
    ));
    out.push_str(&format!(
        "<ellipse class=\"{css_class}-header\" cx=\"{cx}\" cy=\"{cy_top}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{hf}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        hf = header_fill,
        sw = stroke_width,
    ));
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{cy_top}\" x2=\"{x}\" y2=\"{cy_bot}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        sw = stroke_width,
    ));
    out.push_str(&format!(
        "<line x1=\"{x2}\" y1=\"{cy_top}\" x2=\"{x2}\" y2=\"{cy_bot}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        x2 = x + w,
        sw = stroke_width,
    ));
    let label_y = cy_top - 4;
    let name_y = cy_top + ell_ry + 14;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a thick-border rounded rect node — used by `<<aggregate>>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_thick_rounded_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let rx = 4;
    let thick_sw = (stroke_width * 3.0).min(6.0);
    let cx = x + w / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = thick_sw,
    ));
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{hf}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        hf = header_fill,
        sw = thick_sw,
    ));
    let sq_y = y + header_h - rx;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{sq_y}\" width=\"{w}\" height=\"{rx}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a double-border rectangle node — used by `<<datatype>>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_double_border_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    let inset = 3_i32;
    out.push_str(&format!(
        "<rect class=\"{css_class}-inner\" x=\"{ix}\" y=\"{iy}\" width=\"{iw}\" height=\"{ih}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        ix = x + inset,
        iy = y + inset,
        iw = (w - inset * 2).max(1),
        ih = (h - inset * 2).max(1),
        sw = stroke_width,
    ));
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a rectangle with a corner U mark — used by `<<utility>>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_corner_u_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    let u_w = 10_i32;
    let u_h = 10_i32;
    let u_x = x + w - u_w - 4;
    let u_y = y + 4;
    let u_rx = u_w / 2;
    out.push_str(&format!(
        "<path class=\"{css_class}-corner-u\" d=\"M {ux},{uy} L {ux},{uy2} A {rx},{rx} 0 0 0 {ux2},{uy2} L {ux2},{uy}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ux = u_x,
        uy = u_y,
        uy2 = u_y + u_h,
        ux2 = u_x + u_w,
        rx = u_rx,
    ));
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}
