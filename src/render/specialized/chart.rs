use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

use super::*;

pub fn render_chart_svg(document: &ChartDocument) -> String {
    render_chart_artifact(document).svg
}

/// Render a chart into a typed [`RenderArtifact`].
///
/// The SVG is byte-identical to what `render_chart_svg` previously produced.
/// In addition, a [`RenderScene`] is built from the *actual* drawn geometry
/// so the scene stays consistent with the SVG output (no scene/SVG drift).
/// Each drawn element (plot area, bars, points, legend swatches, pie bounding
/// box) becomes a `SceneNode` at its exact SVG coordinates.
pub fn render_chart_artifact(document: &ChartDocument) -> RenderArtifact {
    let title_lines: Vec<&str> = document
        .title
        .as_deref()
        .map(|title| title.lines().collect())
        .unwrap_or_default();
    let title_px = title_lines
        .iter()
        .map(|line| estimate_text_width(line, 16))
        .max()
        .unwrap_or(0);
    let width = 780.max(title_px + 80);
    let height = 420;
    let style = &document.style;
    let series = effective_chart_series(document);
    let categories = effective_chart_categories(document, &series);
    let type_name = chart_subtype_name(document.subtype);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-chart-type=\"{type_name}\" data-chart-horizontal=\"{}\" data-chart-stacked=\"{}\">",
        document.horizontal,
        document.stacked,
        w = width,
        h = height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-chart-style=\"{} {} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.series_color),
        escape_text(&style.bar_color),
        escape_text(&style.line_color),
        escape_text(&style.pie_border_color),
        escape_text(&style.font_color)
    ));
    let mut y = 28;
    if !title_lines.is_empty() {
        for line in &title_lines {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
                escape_text(line)
            ));
            y += 22;
        }
    }
    // Suppress visible type-name label (#488) — it leaks into the axis title slot.
    // The type is already encoded in the SVG root attribute data-chart-type.
    out.push_str(&format!(
        "<metadata data-chart-subtype-label=\"{}\"/>",
        escape_text(type_name)
    ));
    if !document.palette.is_empty() {
        out.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_text(&document.palette.join(" "))
        ));
    }
    if !series.is_empty() {
        let names = series
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join("|");
        out.push_str(&format!(
            "<metadata data-chart-series=\"{}\"/>",
            escape_text(&names)
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-label-mode=\"{}\"/>",
        chart_label_mode_name(document.label_mode)
    ));
    let legend_visible = chart_legend_visible(document, &series);
    let legend_left = legend_visible && document.legend.h_align == crate::model::LegendHAlign::Left;
    let legend_right =
        legend_visible && document.legend.h_align == crate::model::LegendHAlign::Right;
    let legend_bottom =
        legend_visible && document.legend.v_align == crate::model::LegendVAlign::Bottom;
    let plot_top =
        y + if legend_visible && document.legend.v_align == crate::model::LegendVAlign::Top {
            54
        } else {
            16
        };
    let plot_bottom = height - if legend_bottom { 122 } else { 74 };
    let plot_left = if legend_left { 218 } else { 78 };
    let plot_right = width - if legend_right { 178 } else { 40 };
    let plot = ChartPlotArea {
        left: plot_left,
        top: plot_top,
        right: plot_right,
        bottom: plot_bottom,
    };
    match document.subtype {
        ChartSubtype::Bar if document.horizontal => {
            render_chart_horizontal_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Bar => {
            render_chart_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Line => {
            render_chart_line(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Area => {
            render_chart_area(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Scatter => {
            render_chart_scatter(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Pie => {
            let points = effective_chart_points(document, &series, &categories);
            render_chart_pie(
                document,
                &mut out,
                &points,
                width / 2,
                (plot_top + plot_bottom) / 2,
                style,
            )
        }
    }
    render_chart_annotations(&mut out, document, plot);
    render_chart_caption(&mut out, document, width, height);
    render_chart_legend(&mut out, document, &series, plot);
    out.push_str("</svg>");

    let scene = build_chart_scene(
        document,
        &series,
        &categories,
        plot,
        width,
        height,
        legend_visible,
    );
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from Chart's laid-out geometry.
///
/// Scene nodes are derived from the *same* coordinate calculations the SVG
/// renderer uses, so the scene never diverges from the drawn output:
/// - `"plot_area"` — the axis-bounded drawing rectangle
/// - `"bar::{series_idx}::{cat_idx}"` — each bar rect (bar/horizontal-bar charts)
/// - `"point::{series_idx}::{cat_idx}"` — each data point (line/area/scatter charts)
/// - `"pie_area"` — bounding box of the pie disc (pie charts)
/// - `"legend::{idx}"` — each legend swatch rect (when legend is visible)
fn build_chart_scene(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    width: i32,
    height: i32,
    legend_visible: bool,
) -> RenderScene {
    let viewport = Rect::new(0.0, 0.0, width as f64, height as f64);
    let mut scene = RenderScene::new(viewport);

    // --- plot area ---
    let plot_w = (plot.right - plot.left).max(0);
    let plot_h = (plot.bottom - plot.top).max(0);
    add_scene_rect(
        &mut scene,
        "plot_area",
        "plot_area",
        plot.left as f64,
        plot.top as f64,
        plot_w as f64,
        plot_h as f64,
    );

    match document.subtype {
        ChartSubtype::Bar if document.horizontal => {
            build_horizontal_bar_nodes(&mut scene, document, series, categories, plot);
        }
        ChartSubtype::Bar => {
            build_bar_nodes(&mut scene, document, series, categories, plot);
        }
        ChartSubtype::Line | ChartSubtype::Area | ChartSubtype::Scatter => {
            build_point_nodes(&mut scene, document, series, categories, plot);
        }
        ChartSubtype::Pie => {
            build_pie_node(&mut scene, document, series, categories, plot, width);
        }
    }

    // --- legend swatches ---
    if legend_visible {
        build_legend_nodes(&mut scene, document, series, plot);
    }

    scene
}

/// Add bar rect nodes for a vertical bar chart — same geometry as
/// `render_chart_bars` in `svg.rs`.
fn build_bar_nodes(
    scene: &mut RenderScene,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
) {
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.right - plot.left).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_w = ((band - 8) / group_count).max(4);

    for (cat_idx, _category) in categories.iter().enumerate() {
        let band_x = plot.left + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let bx = band_x
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_w
                };
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let y1 = chart_y_for_value(from, min_value, max_value, plot);
            let y2 = chart_y_for_value(to, min_value, max_value, plot);
            let by = y1.min(y2);
            let bh = (y1 - y2).abs().max(1);
            let id = format!("bar::{series_idx}::{cat_idx}");
            add_scene_rect(
                scene,
                &id,
                &id,
                bx as f64,
                by as f64,
                bar_w as f64,
                bh as f64,
            );
        }
    }
}

/// Add bar rect nodes for a horizontal bar chart — same geometry as
/// `render_chart_horizontal_bars` in `svg.rs`.
fn build_horizontal_bar_nodes(
    scene: &mut RenderScene,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
) {
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.bottom - plot.top).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_h = ((band - 8) / group_count).max(4);

    for (cat_idx, _category) in categories.iter().enumerate() {
        let band_y = plot.top + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let x1 = chart_x_for_value(from, min_value, max_value, plot);
            let x2 = chart_x_for_value(to, min_value, max_value, plot);
            let bx = x1.min(x2);
            let bw = (x1 - x2).abs().max(1);
            let by = band_y
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_h
                };
            let id = format!("bar::{series_idx}::{cat_idx}");
            add_scene_rect(
                scene,
                &id,
                &id,
                bx as f64,
                by as f64,
                bw as f64,
                bar_h as f64,
            );
        }
    }
}

/// Add point nodes (6×6 rect centred on the data point) for line, area, and
/// scatter charts — same coordinates as the `<circle>` elements in `svg.rs`.
fn build_point_nodes(
    scene: &mut RenderScene,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
) {
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);

    for (series_idx, item) in series.iter().enumerate() {
        for (cat_idx, _category) in categories.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let px = plot.left + ((cat_idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            // Represent the point as a 6×6 rect centred on (px, py) — matches
            // the r=3 circle the SVG draws.
            let id = format!("point::{series_idx}::{cat_idx}");
            add_scene_rect(scene, &id, &id, (px - 3) as f64, (py - 3) as f64, 6.0, 6.0);
        }
    }
}

/// Add a `"pie_area"` node covering the bounding box of the pie disc.
fn build_pie_node(
    scene: &mut RenderScene,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    width: i32,
) {
    let cx = width / 2;
    let cy = (plot.top + plot.bottom) / 2;
    let radius = 120_i32;
    // Bounding box of the pie circle.
    add_scene_rect(
        scene,
        "pie_area",
        effective_chart_categories(document, series)
            .iter()
            .chain(categories.iter())
            .next()
            .map(|s| s.as_str())
            .unwrap_or("pie"),
        (cx - radius) as f64,
        (cy - radius) as f64,
        (radius * 2) as f64,
        (radius * 2) as f64,
    );
}

/// Add legend swatch nodes — same coordinates as the `<rect class="chart-legend-swatch">`
/// elements in `parts.rs`.
fn build_legend_nodes(
    scene: &mut RenderScene,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    plot: ChartPlotArea,
) {
    // Replicate the legend-item list that `render_chart_legend` builds.
    let pie_points;
    let legend_items: Vec<&str> = if document.subtype == ChartSubtype::Pie {
        let cats = effective_chart_categories(document, series);
        pie_points = effective_chart_points(document, series, &cats);
        pie_points.iter().map(|p| p.label.as_str()).collect()
    } else {
        series.iter().map(|s| s.name.as_str()).collect()
    };
    if legend_items.is_empty() {
        return;
    }
    let x = match document.legend.h_align {
        crate::model::LegendHAlign::Left => 24,
        crate::model::LegendHAlign::Center => ((plot.left + plot.right) / 2) - 66,
        crate::model::LegendHAlign::Right => plot.right + 20,
    };
    let y = match document.legend.v_align {
        crate::model::LegendVAlign::Top => (plot.top - 44).max(44),
        crate::model::LegendVAlign::Bottom => plot.bottom + 46,
    };
    for (idx, name) in legend_items.iter().enumerate() {
        let cy = y + 18 + (idx as i32) * 18;
        // Swatch: x+8, cy-9, 10×10 — matches `render_chart_legend` in parts.rs.
        let id = format!("legend::{idx}");
        let label_id = format!("legend::{idx}::label");
        let swatch_x = (x + 8) as f64;
        let swatch_y = (cy - 9) as f64;
        let label = LabelBox {
            id: label_id,
            text: name.to_string(),
            bounds: Rect::new(swatch_x, swatch_y, 10.0, 10.0),
            owner_id: Some(id.clone()),
            role: LabelRole::Other,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id,
                bounds: Rect::new(swatch_x, swatch_y, 10.0, 10.0),
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }
}

/// Helper: add a single `SceneNode` with a plain rect and no label.
fn add_scene_rect(
    scene: &mut RenderScene,
    id: &str,
    label_text: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let bounds = Rect::new(x, y, w, h);
    let label = LabelBox {
        id: format!("{id}::label"),
        text: label_text.to_string(),
        bounds,
        owner_id: Some(id.to_string()),
        role: LabelRole::Node,
    };
    scene.add_node(SceneNode {
        id: id.to_string(),
        node_box: NodeBox {
            id: id.to_string(),
            bounds,
            ports: Vec::new(),
            labels: vec![label],
        },
    });
}

mod layout;
mod model;
mod parts;
mod svg;

use layout::*;
use model::*;
use parts::*;
use svg::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{normalize_family, parser, NormalizedDocument};

    fn parse_chart(src: &str) -> ChartDocument {
        let parsed = parser::parse(src).expect("parse failed");
        let NormalizedDocument::Chart(doc) = normalize_family(parsed).expect("normalize failed")
        else {
            panic!("expected Chart NormalizedDocument");
        };
        doc
    }

    #[test]
    fn bar_chart_scene_has_nodes_and_valid_geometry() {
        let src = "@startchart\nbar chart\n  Q1 : 42\n  Q2 : 58\n  Q3 : 73\n@endchart";
        let doc = parse_chart(src);
        let artifact = render_chart_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");
        // plot_area + 3 bars
        assert!(
            scene.nodes.len() >= 4,
            "expected >= 4 scene nodes, got {}",
            scene.nodes.len()
        );
        assert!(
            scene.nodes.contains_key("plot_area"),
            "missing plot_area node"
        );
        assert!(
            scene.nodes.contains_key("bar::0::0"),
            "missing bar::0::0 node"
        );
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "geometry validation failed: {issues:?}");
    }

    #[test]
    fn line_chart_scene_has_point_nodes() {
        let src = "@startchart\nline chart\n  Jan : 10\n  Feb : 20\n  Mar : 30\n@endchart";
        let doc = parse_chart(src);
        let artifact = render_chart_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");
        assert!(scene.nodes.contains_key("plot_area"), "missing plot_area");
        assert!(
            scene.nodes.contains_key("point::0::0"),
            "missing point::0::0"
        );
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "geometry issues: {issues:?}");
    }

    #[test]
    fn pie_chart_scene_has_pie_area_node() {
        let src = "@startchart\npie chart\n  Alpha : 30\n  Beta : 70\n@endchart";
        let doc = parse_chart(src);
        let artifact = render_chart_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");
        assert!(
            scene.nodes.contains_key("pie_area"),
            "missing pie_area node"
        );
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "geometry issues: {issues:?}");
    }

    #[test]
    fn svg_is_byte_identical_after_migration() {
        // Confirm that render_chart_svg still returns the same SVG as
        // render_chart_artifact(...).svg — both paths must be identical.
        let src = "@startchart\nbar chart\n  A : 10\n  B : 20\n@endchart";
        let doc = parse_chart(src);
        let svg_direct = render_chart_svg(&doc);
        let artifact = render_chart_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_chart_svg and render_chart_artifact must produce identical SVG"
        );
    }
}
