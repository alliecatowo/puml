use puml::{render_source_to_svg_for_family, DiagramFamily};

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
    assert!(svg.contains("data-chart-type=\"bar\""));
    assert!(svg.contains("data-chart-legend=\"right\""));
    assert!(svg.contains("Revenue"));
    assert!(svg.contains("Q1"));
    assert!(svg.contains("Q2"));
    assert!(svg.contains("Sales"));
    assert!(svg.contains("Costs"));
    assert!(svg.contains("#3498db"));
    assert!(svg.contains("#e67e22"));
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
