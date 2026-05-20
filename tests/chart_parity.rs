mod svg_test_helpers;

use puml::parser::{parse_with_options, ParseOptions};
use puml::{render_source_to_svg_for_family, DiagramFamily, NormalizedDocument};
use svg_test_helpers::{attr, bounds, has_class, SvgDoc};

#[test]
fn chart_axes_named_series_arrays_and_legend_render() {
    let src = "@startchart
h-axis [Q1,Q2]
v-axis \"Revenue\" 0 --> 100
bar \"Sales\" [45,62] #3498db
bar \"Costs\" [20,28] #e67e22
legend right
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("chart parity slice should render");
    let doc = SvgDoc::parse(&svg);
    assert_eq!(doc.root_attr("data-chart-type"), Some("bar"));
    assert_eq!(doc.root_attr("data-chart-horizontal"), Some("false"));

    assert!(!doc.texts_containing("Revenue").is_empty());
    for text in ["Q1", "Q2", "Sales", "Costs"] {
        assert!(
            !doc.texts_containing(text).is_empty(),
            "expected visible chart text containing {text:?}"
        );
    }

    let sales_bar = doc.first_with_attr("rect", "fill", "#3498db");
    let costs_bar = doc.first_with_attr("rect", "fill", "#e67e22");
    let sales_bounds = bounds(sales_bar);
    let costs_bounds = bounds(costs_bar);
    assert!(sales_bounds.width > 0.0 && sales_bounds.height > 0.0);
    assert!(costs_bounds.width > 0.0 && costs_bounds.height > 0.0);
    assert!(
        sales_bounds.x < costs_bounds.x || sales_bounds.y < costs_bounds.y,
        "series bars should occupy distinct chart positions"
    );

    let legend = doc.first_with_attr("g", "data-chart-legend", "right");
    assert!(doc
        .elements_with_class("rect", "chart-legend-swatch")
        .iter()
        .any(|node| attr(*node, "fill") == "#3498db"));
    assert!(
        bounds(
            legend
                .children()
                .find(|node| node.has_tag_name("rect"))
                .unwrap()
        )
        .width
            > 0.0,
        "legend should have a visible frame"
    );
}

#[test]
fn chart_line_series_arrays_use_axis_categories() {
    let src = "@startchart
h-axis \"Quarter\" [Q1,Q2,Q3]
v-axis \"Users\" 0 --> 80
line \"Actual\" [20,40,70] #16a34a
legend bottom
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("line chart arrays should render");
    assert!(svg.contains("data-chart-type=\"line\""));
    assert!(svg.contains("<polyline"));
    assert!(svg.contains("Quarter"));
    assert!(svg.contains("Users"));
    assert!(svg.contains("Actual"));
    assert!(svg.contains("#16a34a"));
}

#[test]
fn chart_horizontal_stacked_bar_mode_renders_metadata_and_legend() {
    let src = "@startchart
h-axis [Q1,Q2]
v-axis \"Revenue\" 0 --> 100
horizontal stacked
bar \"Product A\" [30,40] #0ea5e9
bar \"Product B\" [15,25] #f97316
legend top right
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("horizontal stacked chart should render");
    assert!(svg.contains("data-chart-horizontal=\"true\""));
    assert!(svg.contains("data-chart-stacked=\"true\""));
    assert!(svg.contains("Product A"));
    assert!(svg.contains("Product B"));
    assert!(svg.contains("#0ea5e9"));
    assert!(svg.contains("#f97316"));
}

#[test]
fn chart_palette_caption_annotations_and_tick_step_render_in_normalized_path() {
    let src = "@startchart
bar chart
palette red #2563eb green
caption Forecast confidence
h-axis \"Quarter\" [Q1,Q2,Q3]
v-axis \"Net\" -10 --> 20 step 10
bar \"Actual\" [-5,12,18]
bar \"Plan\" [0,10,15]
annotation \"Q2\" : peak quarter
legend bottom center
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("chart style parity slice should render");
    assert!(svg.contains("data-chart-palette=\"#ff0000 #2563eb #008000\""));
    assert!(svg.contains("data-chart-axis-v-range=\"-10..20\""));
    assert!(svg.contains(">-10<"));
    assert!(svg.contains(">0<"));
    assert!(svg.contains(">10<"));
    assert!(svg.contains(">20<"));
    assert!(svg.contains("fill=\"#ff0000\""));
    assert!(svg.contains("fill=\"#2563eb\""));
    assert!(svg.contains("data-chart-annotation=\"Q2\""));
    assert!(svg.contains("peak quarter"));
    assert!(svg.contains("data-chart-caption=\"true\""));
    assert!(svg.contains("Forecast confidence"));
    assert!(svg.contains("data-chart-legend=\"bottom\""));
}

#[test]
fn chart_legend_off_suppresses_multi_series_legend() {
    let src = "@startchart
h-axis [Q1,Q2]
v-axis \"Revenue\" 0 --> 100
bar \"Sales\" [45,62]
bar \"Costs\" [20,28]
legend off
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("chart legend off should render");
    assert!(!svg.contains("data-chart-legend="));
    assert!(svg.contains("Sales"));
    assert!(svg.contains("Costs"));
}

#[test]
fn chart_axis_and_legend_style_suffixes_render_and_normalize() {
    let src = "@startchart
bar chart
h-axis \"Sprint\" [Alpha,Beta] color #0f766e grid #ccfbf1 text #134e4a
v-axis \"Score\" 0 --> 10 step 5 color #7c2d12 grid #fed7aa text #9a3412
bar \"Actual\" [4,8] #2563eb
legend bottom left background #f8fafc border #0f172a text #111827
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("chart axis and legend styling should render");
    assert!(svg.contains("data-chart-axis-h-color=\"#0f766e\""));
    assert!(svg.contains("data-chart-axis-h-grid=\"#ccfbf1\""));
    assert!(svg.contains("data-chart-axis-v-color=\"#7c2d12\""));
    assert!(svg.contains("data-chart-axis-v-text=\"#9a3412\""));
    assert!(svg.contains("data-chart-legend=\"bottom-left\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#0f172a\""));
    assert!(svg.contains("fill=\"#111827\">Actual</text>"));

    let doc = parse_with_options(src, &ParseOptions::default()).expect("parse chart");
    let NormalizedDocument::Chart(model) = puml::normalize_family(doc).expect("normalize chart")
    else {
        panic!("expected chart model");
    };
    let h_axis = model.h_axis.expect("h-axis model");
    let v_axis = model.v_axis.expect("v-axis model");
    assert_eq!(h_axis.color.as_deref(), Some("#0f766e"));
    assert_eq!(h_axis.grid_color.as_deref(), Some("#ccfbf1"));
    assert_eq!(v_axis.color.as_deref(), Some("#7c2d12"));
    assert_eq!(v_axis.label_color.as_deref(), Some("#9a3412"));
    assert_eq!(model.legend.background_color.as_deref(), Some("#f8fafc"));
    assert_eq!(model.legend.border_color.as_deref(), Some("#0f172a"));
    assert_eq!(model.legend.text_color.as_deref(), Some("#111827"));
}

#[test]
fn chart_pie_uses_explicit_palette_for_slices() {
    let src = "@startchart
pie chart
palette #111827 #f97316 #22c55e
\"Alpha\" : 40
\"Beta\" : 35
\"Gamma\" : 25
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("pie palette chart should render");
    assert!(svg.contains("data-chart-type=\"pie\""));
    assert!(svg.contains("data-chart-palette=\"#111827 #f97316 #22c55e\""));
    assert!(svg.contains("fill=\"#111827\""));
    assert!(svg.contains("fill=\"#f97316\""));
    assert!(svg.contains("fill=\"#22c55e\""));
}

#[test]
fn chart_pie_named_series_arrays_use_axis_categories() {
    let src = "@startchart
pie chart
palette #111827 #f97316 #22c55e
h-axis [Alpha,Beta,Gamma]
pie \"Share\" [40,35,25]
legend off
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("pie series array chart should render");
    assert!(svg.contains("data-chart-type=\"pie\""));
    assert!(svg.contains("data-chart-series=\"Share\""));
    assert!(svg.contains("Alpha"));
    assert!(svg.contains("Beta"));
    assert!(svg.contains("Gamma"));
    assert!(svg.contains("fill=\"#111827\""));
    assert!(svg.contains("fill=\"#f97316\""));
    assert!(svg.contains("fill=\"#22c55e\""));
}

#[test]
fn chart_pie_slice_level_labels_colors_and_legend_render() {
    let src = "@startchart
pie chart
\"Frontend\" : 35 #0ea5e9
\"Backend\" : 40 #f97316
\"Ops\" : 25 #22c55e
legend position bottom left background #f8fafc border #334155 text #111827
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("pie slice-level legend chart should render");
    let doc = SvgDoc::parse(&svg);
    let pie_slice = doc.first_with_attr("path", "data-chart-slice", "Frontend");
    assert!(has_class(pie_slice, "chart-pie-slice"));
    assert!(has_class(pie_slice, "puml-node"));
    assert_eq!(attr(pie_slice, "data-puml-kind"), "chart-pie-slice");
    assert!(svg.contains("data-chart-slice=\"Frontend\""));
    assert!(svg.contains("data-chart-value=\"35\""));
    assert!(svg.contains("data-chart-percent=\"35%\""));
    let pie_label = doc.first_with_attr("text", "data-chart-slice-label", "Frontend");
    assert!(has_class(pie_label, "chart-pie-label"));
    assert!(has_class(pie_label, "puml-label"));
    assert_eq!(attr(pie_label, "data-puml-label-kind"), "slice-label");
    assert!(svg.contains(">Frontend 35%</text>"));
    assert!(svg.contains("data-chart-legend=\"bottom-left\""));
    assert!(svg.contains("class=\"chart-legend-swatch\""));
    assert!(svg.contains("fill=\"#0ea5e9\""));
    assert!(svg.contains("fill=\"#f97316\""));
    assert!(svg.contains("fill=\"#22c55e\""));
}

#[test]
fn chart_axis_ticks_emit_semantic_classes_and_positioned_legend_aliases() {
    let src = "@startchart
bar chart
h-axis \"Quarter\" [Q1,Q2,Q3] color #0f766e grid #ccfbf1 text #134e4a
v-axis \"Score\" 0 --> 20 step 5 color #7c2d12 grid #fed7aa text #9a3412
bar \"Actual\" [5,10,15]
legend at top center background #f8fafc border #0f172a text #111827
@endchart
";

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("chart axis semantic ticks should render");
    let doc = SvgDoc::parse(&svg);
    let tick = doc.first_with_attr("text", "data-chart-axis-tick", "5");
    assert!(has_class(tick, "chart-axis-tick"));
    assert!(has_class(tick, "chart-axis-tick-v"));
    assert!(has_class(tick, "puml-label"));
    assert_eq!(attr(tick, "data-puml-label-kind"), "axis-tick");
    assert!(!doc
        .elements_with_class("line", "chart-axis-grid-h")
        .is_empty());
    assert_eq!(
        attr(
            doc.first_with_attr("g", "data-chart-legend", "top"),
            "class"
        ),
        "chart-legend"
    );
}

#[test]
fn chart_label_position_modes_render_value_metadata_and_pie_callouts() {
    let pie = "@startchart
pie chart
labels outside
\"Frontend\" : 35 #0ea5e9
\"Backend\" : 40 #f97316
\"Ops\" : 25 #22c55e
legend at bottom right
@endchart
";
    let pie_svg = render_source_to_svg_for_family(pie, DiagramFamily::Chart)
        .expect("pie outside labels should render");
    assert!(pie_svg.contains("data-chart-label-mode=\"outside\""));
    assert!(pie_svg.contains("class=\"chart-pie-callout\""));
    assert!(pie_svg.contains("data-chart-legend-h=\"right\""));
    assert!(pie_svg.contains("data-chart-legend-v=\"bottom\""));

    let line = "@startchart
line chart
labels value
h-axis [Sprint1,Sprint2]
v-axis 0 --> 10
line \"Actual\" [3,8]
@endchart
";
    let line_svg = render_source_to_svg_for_family(line, DiagramFamily::Chart)
        .expect("line value labels should render");
    let line_doc = SvgDoc::parse(&line_svg);
    assert!(line_svg.contains("data-chart-label-mode=\"value\""));
    let point = line_doc.first_with_attr("circle", "data-chart-value", "8");
    assert!(has_class(point, "chart-point"));
    assert!(has_class(point, "puml-node"));
    assert_eq!(attr(point, "data-puml-kind"), "chart-point");
    assert!(!line_doc
        .elements_with_class("text", "chart-value-label")
        .is_empty());
}

#[test]
fn chart_long_title_expands_canvas_width() {
    let src = "@startchart
title This is a deliberately long chart title that should not clip near the right edge
bar \"Revenue\" [12,18,20]
@endchart
";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("long-title chart should render");
    let doc = SvgDoc::parse(&svg);
    let width = doc
        .root_attr("width")
        .and_then(|value| value.parse::<f64>().ok())
        .expect("chart width should be present");
    assert!(width > 780.0, "long title should expand chart width");
    assert!(svg.contains("deliberately long chart title"));
}

// Regression test for #488: chart type token ("bar"/"line"/"pie") must NOT
// appear as a visible <text> element in the SVG. The type is stored in the
// data-chart-type root attribute and a <metadata> element only.
#[test]
fn chart_type_name_does_not_leak_as_visible_text_label_488() {
    for (src, type_token) in &[
        (
            "@startchart\nbar chart\nQ1 : 10\nQ2 : 20\n@endchart\n",
            "bar",
        ),
        (
            "@startchart\nline chart\nJan : 10\nFeb : 20\n@endchart\n",
            "line",
        ),
        (
            "@startchart\npie chart\nAlpha : 40\nBeta : 60\n@endchart\n",
            "pie",
        ),
    ] {
        let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
            .unwrap_or_else(|_| panic!("chart type '{type_token}' should render"));
        let doc = SvgDoc::parse(&svg);
        // The type token must be present in the root data- attribute.
        assert_eq!(
            doc.root_attr("data-chart-type"),
            Some(*type_token),
            "data-chart-type attribute should contain the type token"
        );
        // It must NOT appear as a bare visible <text> node.
        // texts_containing only searches <text> elements, so any hit is a leak.
        let visible_type_texts = doc.texts_containing(type_token);
        assert!(
            visible_type_texts.is_empty(),
            "chart type token '{type_token}' leaked as a visible <text> element ({} hit(s))",
            visible_type_texts.len()
        );
    }
}

// Regression test for #491: line chart data labels must be offset from the
// x-axis tick label. The data label tx must exceed px (the data-point x)
// so it nudges away from the tick below it and avoids pixel-level collision.
#[test]
fn chart_line_data_label_origin_does_not_overlap_tick_491() {
    let src = "@startchart
line chart
Jan : 10
Feb : 15
Mar : 12
Apr : 20
@endchart
";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Chart)
        .expect("line chart with data labels should render");
    let doc = SvgDoc::parse(&svg);

    // Collect value-label elements.
    let value_labels = doc.elements_with_class("text", "chart-value-label");
    assert!(
        !value_labels.is_empty(),
        "line chart should emit chart-value-label elements"
    );

    // Find the Jan tick label x position.
    let jan_ticks: Vec<_> = doc.texts_containing("Jan");
    assert!(!jan_ticks.is_empty(), "Jan tick label should be present");
    let jan_x: f64 = jan_ticks
        .first()
        .and_then(|n| n.attribute("x"))
        .and_then(|v| v.parse().ok())
        .expect("Jan tick should have numeric x attribute");

    // The leftmost value label ("10") should be shifted right relative to the
    // Jan tick's x, ensuring no horizontal overlap at the origin.
    let leftmost_label = value_labels
        .iter()
        .min_by_key(|n| {
            n.attribute("x")
                .and_then(|v| v.parse::<f64>().ok())
                .map(|v| (v * 1000.0) as i64)
                .unwrap_or(i64::MAX)
        })
        .expect("at least one value label");
    let label_x: f64 = leftmost_label
        .attribute("x")
        .and_then(|v| v.parse().ok())
        .expect("value label should have numeric x attribute");

    assert!(
        label_x > jan_x,
        "data label x ({label_x}) should be right of Jan tick x ({jan_x}) to avoid overlap"
    );
}
