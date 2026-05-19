use puml::{render_source_to_svg, render_source_to_svg_for_family, DiagramFamily};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct SvgElement {
    attrs: HashMap<String, String>,
    text: Option<String>,
}

impl SvgElement {
    fn attr(&self, name: &str) -> &str {
        self.attrs
            .get(name)
            .unwrap_or_else(|| panic!("missing SVG attribute {name}"))
    }

    fn attr_i32(&self, name: &str) -> i32 {
        self.attr(name)
            .parse()
            .unwrap_or_else(|_| panic!("attribute {name} should be an integer"))
    }

    fn attr_f64(&self, name: &str) -> f64 {
        self.attr(name)
            .parse()
            .unwrap_or_else(|_| panic!("attribute {name} should be numeric"))
    }

    fn class_contains(&self, name: &str) -> bool {
        self.attrs
            .get("class")
            .is_some_and(|classes| classes.split_whitespace().any(|class| class == name))
    }
}

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/families/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).expect("fixture should be readable")
}

fn parse_attrs(tag: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let bytes = tag.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        let key_start = idx;
        while idx < bytes.len()
            && (bytes[idx].is_ascii_alphanumeric() || matches!(bytes[idx], b'-' | b'_'))
        {
            idx += 1;
        }
        if key_start == idx {
            idx += 1;
            continue;
        }
        let key = &tag[key_start..idx];
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if bytes.get(idx) != Some(&b'=') {
            continue;
        }
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if bytes.get(idx) != Some(&b'"') {
            continue;
        }
        idx += 1;
        let value_start = idx;
        while idx < bytes.len() && bytes[idx] != b'"' {
            idx += 1;
        }
        attrs.insert(key.to_string(), tag[value_start..idx].to_string());
        idx += 1;
    }
    attrs
}

fn elements(svg: &str, name: &str) -> Vec<SvgElement> {
    let needle = format!("<{name} ");
    svg.split(&needle)
        .skip(1)
        .filter_map(|chunk| {
            let close = chunk.find('>')?;
            let tag = &chunk[..close];
            let text = if name == "text" {
                chunk[close + 1..]
                    .find("</text>")
                    .map(|end| chunk[close + 1..close + 1 + end].to_string())
            } else {
                None
            };
            Some(SvgElement {
                attrs: parse_attrs(tag),
                text,
            })
        })
        .collect()
}

fn rects(svg: &str) -> Vec<SvgElement> {
    elements(svg, "rect")
}

fn lines(svg: &str) -> Vec<SvgElement> {
    elements(svg, "line")
}

fn paths(svg: &str) -> Vec<SvgElement> {
    elements(svg, "path")
}

fn texts(svg: &str) -> Vec<SvgElement> {
    elements(svg, "text")
}

fn svg_attr(svg: &str, name: &str) -> String {
    let close = svg.find('>').expect("svg root tag should close");
    let tag = &svg[svg.find("<svg ").expect("svg root tag should exist") + 5..close];
    parse_attrs(tag)
        .remove(name)
        .unwrap_or_else(|| panic!("missing svg attribute {name}"))
}

fn text_position(svg: &str, label: &str) -> (i32, i32) {
    let text = texts(svg)
        .into_iter()
        .find(|text| text.text.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing text label {label}"));
    (text.attr_i32("x"), text.attr_i32("y"))
}

fn rect_containing_text(svg: &str, label: &str, class_name: &str) -> SvgElement {
    let (tx, ty) = text_position(svg, label);
    rects(svg)
        .into_iter()
        .find(|rect| {
            rect.class_contains(class_name)
                && tx >= rect.attr_i32("x")
                && tx <= rect.attr_i32("x") + rect.attr_i32("width")
                && ty >= rect.attr_i32("y")
                && ty <= rect.attr_i32("y") + rect.attr_i32("height")
        })
        .unwrap_or_else(|| panic!("missing {class_name} rect containing {label}"))
}

#[test]
fn chart_negative_axis_tick_step_and_positioned_legend_have_geometry() {
    let src = read_fixture("valid_chart_axis_legend_depth.puml");
    let svg = render_source_to_svg_for_family(&src, DiagramFamily::Chart)
        .expect("chart depth fixture should render");

    let zero_axis = lines(&svg)
        .into_iter()
        .find(|line| line.class_contains("chart-zero-axis"))
        .expect("negative chart should render a zero axis");
    let zero_y = zero_axis.attr_i32("y1");
    assert_eq!(zero_y, zero_axis.attr_i32("y2"));

    let actual_neg = rects(&svg)
        .into_iter()
        .find(|rect| rect.attr("fill") == "#dc2626" && rect.attr_i32("y") >= zero_y)
        .filter(|rect| rect.attr_i32("width") > 20)
        .expect("actual negative bar should start below the zero axis");
    assert!(
        actual_neg.attr_i32("y") + actual_neg.attr_i32("height") > zero_y,
        "negative bar should extend downward from the zero axis: {actual_neg:?}"
    );

    let plan_pos = rects(&svg)
        .into_iter()
        .find(|rect| rect.attr("fill") == "#2563eb" && rect.attr_i32("y") < zero_y)
        .filter(|rect| rect.attr_i32("width") > 20)
        .expect("plan positive bar should render above the zero axis");
    assert!(
        plan_pos.attr_i32("y") + plan_pos.attr_i32("height") <= zero_y,
        "positive bar should terminate at or above the zero axis: {plan_pos:?}"
    );

    let tick_values = texts(&svg)
        .into_iter()
        .filter(|text| text.class_contains("chart-axis-tick-v"))
        .map(|text| text.attr("data-chart-axis-tick").to_string())
        .collect::<Vec<_>>();
    assert_eq!(tick_values, ["-10", "0", "10", "20"]);

    let legend_start = svg
        .find("<g class=\"chart-legend\"")
        .expect("legend group should render");
    let legend_rect = rects(&svg[legend_start..])
        .into_iter()
        .next()
        .expect("legend should contain a bounding rect");
    assert_eq!(legend_rect.attr("fill"), "#f8fafc");
    assert!(
        legend_rect.attr_i32("y") < zero_y,
        "top legend should be above the plot's zero axis: {legend_rect:?}"
    );
}

#[test]
fn pie_outside_labels_render_outside_the_slice_radius_with_callouts() {
    let src = read_fixture("valid_chart_pie_outside_depth.puml");
    let svg = render_source_to_svg_for_family(&src, DiagramFamily::Chart)
        .expect("pie depth fixture should render");

    let first_slice = paths(&svg)
        .into_iter()
        .find(|path| path.class_contains("chart-pie-slice"))
        .expect("pie should render slices");
    let d = first_slice.attr("d");
    let coords = d
        .split_whitespace()
        .filter_map(|part| part.parse::<f64>().ok())
        .collect::<Vec<_>>();
    let (cx, cy) = (coords[0], coords[1]);
    let radius = 120.0;

    let labels = texts(&svg)
        .into_iter()
        .filter(|text| text.class_contains("chart-pie-label"))
        .collect::<Vec<_>>();
    let callouts = lines(&svg)
        .into_iter()
        .filter(|line| line.class_contains("chart-pie-callout"))
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 3);
    assert_eq!(callouts.len(), 3);

    for label in labels {
        let dx = label.attr_f64("x") - cx;
        let dy = label.attr_f64("y") - cy;
        let distance = (dx * dx + dy * dy).sqrt();
        assert!(
            distance > radius,
            "outside pie label should sit beyond the slice radius: {label:?}"
        );
    }
}

#[test]
fn mindmap_left_right_styles_are_asserted_by_node_geometry() {
    let src = read_fixture("valid_mindmap_visual_depth.puml");
    let svg = render_source_to_svg(&src).expect("mindmap depth fixture should render");

    let root = rect_containing_text(&svg, "Platform", "mindmap-root");
    let risk = rect_containing_text(&svg, "Risk", "mindmap-node");
    let security = rect_containing_text(&svg, "Security", "mindmap-node");
    let delivery = rect_containing_text(&svg, "Delivery", "mindmap-node");
    let release = rect_containing_text(&svg, "Release", "mindmap-node");

    assert_eq!(risk.attr("data-mindmap-side"), "left");
    assert_eq!(risk.attr("fill"), "#fecaca");
    assert!(
        risk.attr_i32("x") + risk.attr_i32("width") < root.attr_i32("x"),
        "left branch should be placed left of the root"
    );
    assert!(
        security.attr_i32("x") + security.attr_i32("width") < risk.attr_i32("x"),
        "left grandchild should be farther left than its parent"
    );

    assert_eq!(delivery.attr("data-mindmap-side"), "right");
    assert_eq!(delivery.attr("fill"), "#bbf7d0");
    assert!(
        delivery.attr_i32("x") > root.attr_i32("x") + root.attr_i32("width"),
        "right branch should be placed right of the root"
    );
    assert!(
        release.attr_i32("x") > delivery.attr_i32("x") + delivery.attr_i32("width"),
        "right grandchild should be farther right than its parent"
    );
}

#[test]
fn parity_status_mindmap_nodes_stay_inside_svg_bounds_with_padding() {
    let src = r#"@startmindmap
* PlantUML\nParity Status

** Core UML Families
*** Sequence\n[broad]
**** Participants & lifelines
**** Alt / opt / loop blocks
**** Autonumber
**** Notes & separators
*** Class\n[broad]
**** Inheritance / composition
**** Interfaces & abstract
**** Generics
*** State\n[broad]
**** Nested sub-states
**** Concurrent regions
**** History pseudostates
**** Fork / join / choice
*** Activity\n[partial]
**** Basic flow
**** Branch & merge
**** Swimlanes
*** Use Case\n[partial]
*** Component\n[partial]
*** Deployment\n[partial]
*** Object\n[partial]

** Non-UML Families
*** Mindmap\n[broad]
*** WBS\n[broad]
*** Gantt\n[partial]
**** Tasks & milestones
**** Dependencies
*** Salt (wireframe)\n[partial]
*** JSON / YAML\n[partial]
*** Timing\n[partial]
*** Chronology\n[partial]
*** NWDiag\n[partial]
*** Archimate\n[partial]
*** Chart\n[partial]
*** EBNF\n[partial]
*** Regex\n[stub]
*** SDL\n[stub]
*** Ditaa\n[stub]
*** Math\n[stub]

** Preprocessor
*** !include (file)\n[broad]
*** !include (URL)\n[opt-in]
*** !define / !undef\n[broad]
*** !if / !ifdef\n[broad]
*** Macros\n[broad]
*** !theme\n[partial]
*** stdlib icons\n[partial]

** Output Formats
*** SVG\n[broad]
*** PNG\n[broad]
*** JPEG / WEBP\n[broad]
*** utxt / atxt\n[partial]
*** HTML\n[partial]
@endmindmap"#;
    let svg = render_source_to_svg(src).expect("parity status mindmap should render");
    let svg_width = svg_attr(&svg, "width")
        .parse::<i32>()
        .expect("svg width should be numeric");

    let mindmap_nodes = rects(&svg)
        .into_iter()
        .filter(|rect| rect.class_contains("mindmap-node"))
        .collect::<Vec<_>>();
    assert!(
        !mindmap_nodes.is_empty(),
        "parity status render should include mindmap nodes"
    );

    for rect in mindmap_nodes {
        assert!(
            rect.attr_i32("x") >= 8,
            "mindmap node should keep at least 8px left padding: {rect:?}"
        );
        assert!(
            rect.attr_i32("x") + rect.attr_i32("width") <= svg_width - 8,
            "mindmap node should keep at least 8px right padding: {rect:?}"
        );
    }
}

#[test]
fn wbs_left_to_right_progress_and_colors_are_asserted_by_geometry() {
    let src = read_fixture("valid_wbs_visual_depth.puml");
    let svg = render_source_to_svg(&src).expect("wbs depth fixture should render");

    let root = rect_containing_text(&svg, "Program", "wbs-node");
    let build = rect_containing_text(&svg, "Build", "wbs-node");
    let parser = rect_containing_text(&svg, "Parser", "wbs-node");
    let launch = rect_containing_text(&svg, "Launch [80%]", "wbs-node");

    assert_eq!(root.attr("data-wbs-depth"), "0");
    assert_eq!(build.attr("data-wbs-depth"), "1");
    assert_eq!(parser.attr("data-wbs-depth"), "2");
    assert_eq!(build.attr("fill"), "#dbeafe");
    assert_eq!(launch.attr("fill"), "#fef9c3");
    assert_eq!(build.attr("data-wbs-checkbox"), "checked");
    assert_eq!(launch.attr("data-wbs-checkbox"), "progress");
    assert_eq!(launch.attr("data-wbs-progress"), "80");

    let root_cx = root.attr_i32("x") + root.attr_i32("width") / 2;
    let build_cx = build.attr_i32("x") + build.attr_i32("width") / 2;
    let parser_cx = parser.attr_i32("x") + parser.attr_i32("width") / 2;
    assert!(
        root_cx < build_cx && build_cx < parser_cx,
        "left-to-right WBS should increase x by depth"
    );

    let progress_track = rects(&svg)
        .into_iter()
        .find(|rect| rect.class_contains("wbs-progress-track"))
        .expect("progress node should render a track");
    let progress_fill = rects(&svg)
        .into_iter()
        .find(|rect| rect.class_contains("wbs-progress-fill"))
        .expect("progress node should render a fill");
    assert_eq!(
        progress_fill.attr_i32("width"),
        progress_track.attr_i32("width") * 80 / 100
    );
}
