use puml::model::{FamilyNodeKind, NormalizedDocument};

// height is part of the SVG rect schema and may be needed by future tests.
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct SvgRect {
    class: Option<String>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone)]
struct SvgText {
    class: Option<String>,
    x: i32,
    text: String,
}

fn svg_attr(tag: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

fn parse_i32_attr(tag: &str, key: &str) -> Option<i32> {
    svg_attr(tag, key)?.parse::<i32>().ok()
}

fn svg_viewbox(svg: &str) -> Option<(i32, i32)> {
    let tag = svg.split("<svg ").nth(1)?.split('>').next()?;
    let viewbox = svg_attr(tag, "viewBox")?;
    let parts = viewbox
        .split_whitespace()
        .filter_map(|v| v.parse::<i32>().ok())
        .collect::<Vec<_>>();
    Some((*parts.get(2)?, *parts.get(3)?))
}

fn svg_rects(svg: &str) -> Vec<SvgRect> {
    svg.split("<rect ")
        .skip(1)
        .filter_map(|chunk| {
            let tag = chunk.split('>').next()?;
            Some(SvgRect {
                class: svg_attr(tag, "class"),
                x: parse_i32_attr(tag, "x")?,
                y: parse_i32_attr(tag, "y")?,
                width: parse_i32_attr(tag, "width")?,
                height: parse_i32_attr(tag, "height")?,
            })
        })
        .collect()
}

fn svg_texts(svg: &str) -> Vec<SvgText> {
    svg.split("<text ")
        .skip(1)
        .filter_map(|chunk| {
            let attrs = chunk.split('>').next()?;
            let body = chunk.split_once('>')?.1.split("</text>").next()?;
            Some(SvgText {
                class: svg_attr(attrs, "class"),
                x: parse_i32_attr(attrs, "x")?,
                text: body.to_string(),
            })
        })
        .collect()
}

fn polyline_tags(svg: &str) -> Vec<&str> {
    svg.split("<polyline ")
        .skip(1)
        .filter_map(|chunk| chunk.split('>').next())
        .collect()
}

fn parse_points(points: &str) -> Vec<(i32, i32)> {
    points
        .split_whitespace()
        .filter_map(|pair| {
            let (x, y) = pair.split_once(',')?;
            Some((x.parse::<i32>().ok()?, y.parse::<i32>().ok()?))
        })
        .collect()
}

fn polygon_points(svg: &str) -> Vec<Vec<(i32, i32)>> {
    svg.split("<polygon ")
        .skip(1)
        .filter_map(|chunk| {
            let tag = chunk.split('>').next()?;
            Some(parse_points(&svg_attr(tag, "points")?))
        })
        .collect()
}

/// Approximate pixel x for time `t` by interpolating from rendered tick labels.
/// Falls back to a viewBox-proportional estimate when tick labels are absent.
fn time_to_x_approx(svg: &str, t: i32, total_t: i32) -> i32 {
    let mut tick_map: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
    for chunk in svg.split("<text ").skip(1) {
        let Some(attrs_end) = chunk.find('>') else {
            continue;
        };
        let attrs = &chunk[..attrs_end];
        // Only process timing-tick labels.
        if svg_attr(attrs, "class").as_deref() != Some("timing-tick") {
            continue;
        }
        let Some(x) = parse_i32_attr(attrs, "x") else {
            continue;
        };
        let body = &chunk[attrs_end + 1..];
        let Some(end) = body.find("</text>") else {
            continue;
        };
        let Ok(tick_t) = body[..end].trim().parse::<i32>() else {
            continue;
        };
        tick_map.insert(tick_t, x);
    }
    if tick_map.len() >= 2 {
        let (&t0, &x0) = tick_map.iter().next().unwrap();
        let (&t1, &x1) = tick_map.iter().last().unwrap();
        if t1 != t0 {
            let slope = (x1 - x0) as f64 / (t1 - t0) as f64;
            return x0 + ((t - t0) as f64 * slope).round() as i32;
        }
    }
    // Fallback: derive proportionally from viewBox.
    let (view_w, _) = svg_viewbox(svg).unwrap_or((1008, 500));
    let left_pad = 130;
    let right_pad = 118;
    let chart_w = view_w - left_pad - right_pad;
    left_pad + ((t as f64 / total_t as f64) * chart_w as f64) as i32
}

#[test]
fn timing_advanced_semantics_have_model_oracle_metadata() {
    let src = include_str!("fixtures/families/valid_timing_advanced_geometry.puml");
    let document = puml::parse(src).expect("parse timing fixture");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize timing fixture")
    else {
        panic!("timing fixture should normalize as family model");
    };

    let signals = model
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                FamilyNodeKind::TimingConcise
                    | FamilyNodeKind::TimingRobust
                    | FamilyNodeKind::TimingBinary
                    | FamilyNodeKind::TimingClock
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(signals.len(), 4);
    assert!(
        signals.iter().any(|node| node.name == "CLK"
            && node
                .members
                .iter()
                .any(|member| member.text == "period 6 pulse 2 offset 0")),
        "clock period/pulse/offset controls should be preserved in model"
    );

    let events = model
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, FamilyNodeKind::TimingEvent))
        .collect::<Vec<_>>();
    assert!(
        events
            .iter()
            .any(|node| node.name == "8" && node.alias.as_deref() == Some("EN")),
        "relative @+3 event should resolve from @5 to absolute time 8"
    );
    assert!(
        events.iter().any(|node| {
            node.name == "5" && node.label.as_deref() == Some("range:12:active window")
        }),
        "@5 <-> @12 range should be retained as oracle metadata"
    );
    assert!(
        events
            .iter()
            .any(|node| node.name == "12" && node.label.as_deref() == Some("range:18:cooldown")),
        "highlight 12 to 18 should be retained as range metadata"
    );
}

#[test]
fn timing_advanced_semantics_render_expected_svg_geometry() {
    let src = include_str!("fixtures/families/valid_timing_advanced_geometry.puml");
    let svg = puml::render_source_to_svg(src).expect("render timing fixture");
    const TOTAL_T: i32 = 18; // fixture spans t=0..18

    let range_rects = svg_rects(&svg)
        .into_iter()
        .filter(|rect| rect.class.as_deref() == Some("timing-range"))
        .collect::<Vec<_>>();
    assert_eq!(range_rects.len(), 2, "expected range and highlight bands");

    // Sort by x so active (@5..@12) is leftmost and cooldown (@12..@18) is rightmost.
    let mut sorted = range_rects.clone();
    sorted.sort_by_key(|r| r.x);
    let active = &sorted[0];
    let cooldown = &sorted[1];

    // Active must end before or exactly at cooldown's left edge (allow ±2px rounding).
    assert!(
        active.x + active.width <= cooldown.x + 2,
        "active window should end before cooldown: active.x+w={}, cooldown.x={}",
        active.x + active.width,
        cooldown.x
    );
    assert_eq!(
        cooldown.y, active.y,
        "range/highlight bands share axis origin"
    );

    // Proportional width: active spans 7 time units, cooldown 6 — active must be wider.
    assert!(
        active.width > cooldown.width,
        "active (7 units) should be wider than cooldown (6 units): {}, {}",
        active.width,
        cooldown.width
    );
    let ratio = active.width as f64 / cooldown.width as f64;
    assert!(
        ratio > 0.95 && ratio < 1.55,
        "active/cooldown width ratio should be ~7/6 ≈ 1.17, got {ratio:.2}"
    );

    // Range labels: each label x must lie inside its band's x-range.
    let range_labels = svg_texts(&svg)
        .into_iter()
        .filter(|text| text.class.as_deref() == Some("timing-range-label"))
        .collect::<Vec<_>>();
    assert!(
        range_labels.iter().any(|t| {
            t.text == "active window" && t.x >= active.x && t.x <= active.x + active.width
        }),
        "active window label x should be inside band [{}, {}]: {:?}",
        active.x,
        active.x + active.width,
        range_labels
    );
    assert!(
        range_labels.iter().any(|t| {
            t.text == "cooldown" && t.x >= cooldown.x && t.x <= cooldown.x + cooldown.width
        }),
        "cooldown label x should be inside band [{}, {}]: {:?}",
        cooldown.x,
        cooldown.x + cooldown.width,
        range_labels
    );

    // Clock polyline: structural metadata attributes must be present.
    let polylines = polyline_tags(&svg);
    let clock = polylines
        .iter()
        .find(|tag| svg_attr(tag, "data-timing-period").as_deref() == Some("6"))
        .expect("clock polyline should expose period metadata");
    assert_eq!(svg_attr(clock, "data-timing-pulse").as_deref(), Some("2"));
    assert_eq!(svg_attr(clock, "data-timing-offset").as_deref(), Some("0"));
    // Waveform must alternate between at least 2 y-levels.
    let clock_points = parse_points(&svg_attr(clock, "points").expect("clock points"));
    assert!(
        clock_points.len() >= 4,
        "clock needs ≥4 points for one period"
    );
    let clock_ys: std::collections::BTreeSet<i32> = clock_points.iter().map(|(_, y)| *y).collect();
    assert!(
        clock_ys.len() >= 2,
        "clock waveform must alternate high/low: {clock_ys:?}"
    );

    // Binary waveform (EN): must have ≥2 vertical transitions (same x, different y).
    let binary = polylines
        .iter()
        .find(|tag| svg_attr(tag, "data-timing-period").is_none())
        .expect("binary waveform should exist");
    let binary_points = parse_points(&svg_attr(binary, "points").expect("binary points"));
    let mut x_to_ys: std::collections::BTreeMap<i32, Vec<i32>> = std::collections::BTreeMap::new();
    for (x, y) in &binary_points {
        x_to_ys.entry(*x).or_default().push(*y);
    }
    let transitions = x_to_ys
        .values()
        .filter(|ys| ys.iter().min() != ys.iter().max())
        .count();
    assert!(
        transitions >= 2,
        "binary EN should have ≥2 vertical transitions, got {transitions}"
    );

    // Robust BUS polygon: span should start at chart origin and end near @5.
    let x5 = time_to_x_approx(&svg, 5, TOTAL_T);
    let (view_w, _) = svg_viewbox(&svg).unwrap_or((1008, 500));
    let tol = view_w / 12;
    let polygons = polygon_points(&svg);
    assert!(
        polygons.iter().any(|pts| {
            let min_x = pts.iter().map(|(x, _)| *x).min().unwrap_or_default();
            let max_x = pts.iter().map(|(x, _)| *x).max().unwrap_or_default();
            min_x < x5 && (max_x - x5).abs() <= tol
        }),
        "BUS polygon should span @0..@5 (x5≈{x5}, tol={tol})"
    );
}

#[test]
fn timing_ch10_parity_model_preserves_anchors_options_messages_and_analog() {
    let src = include_str!("fixtures/families/valid_timing_ch10_parity.puml");
    let document = puml::parse(src).expect("parse timing fixture");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize timing fixture")
    else {
        panic!("timing fixture should normalize as family model");
    };

    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.label.as_deref() == Some("__timing:hide-time-axis")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.label.as_deref() == Some("__timing:scale:5 as 120 pixels")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.name == "V"
            && node
                .members
                .iter()
                .any(|member| member.text == "__timing:analog")
            && node
                .members
                .iter()
                .any(|member| member.text == "__timing:analog_between 0 6")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.name == "S"
            && node
                .members
                .iter()
                .any(|member| member.text == "__timing:compact")
    }));
    assert!(
        model
            .relations
            .iter()
            .any(|rel| rel.from == "C@5" && rel.to == "S@7" && rel.label.as_deref() == Some("GET")),
        "C -> S@+2 should resolve through :start/:send anchors and current time"
    );
    assert!(
        model.relations.iter().any(|rel| {
            rel.from == "S@10" && rel.to == "C@10" && rel.label.as_deref() == Some("200 OK")
        }),
        "message without endpoint times should use the current timing cursor"
    );
}

#[test]
fn timing_ch10_parity_renders_messages_hidden_colors_axis_scale_and_analog() {
    let src = include_str!("fixtures/families/valid_timing_ch10_parity.puml");
    let svg = puml::render_source_to_svg(src).expect("render timing fixture");
    let (view_w, view_h) = svg_viewbox(&svg).expect("timing svg should have viewBox");

    assert!(
        view_h < 390,
        "mode compact should reduce the row stack height, got {view_h}"
    );
    assert!(
        view_w < 760,
        "scale 5 as 120 pixels should produce a narrower chart, got {view_w}"
    );
    assert!(
        !svg.contains("class=\"timing-tick\""),
        "hide time-axis should suppress tick labels"
    );
    assert!(svg.contains("class=\"timing-message\""));
    assert!(svg.contains("GET"));
    assert!(svg.contains("200 OK"));
    assert!(svg.contains("class=\"timing-hidden-state\""));
    assert!(svg.contains("fill=\"LightCyan\""));
    assert!(svg.contains("stroke=\"Aqua\""));
    assert!(svg.contains("class=\"timing-analog\""));
    assert!(svg.contains("class=\"timing-analog-point\""));
}

#[test]
fn timing_by_clock_ticks_resolve_using_clock_period() {
    let src = r#"@startuml
clock "clk" as clk with period 50
concise "Signal1" as S1
@clk*0
S1 is 0
@clk*1
S1 is 1
@clk*3
S1 is 2
@enduml
"#;
    let document = puml::parse(src).expect("parse timing clock fixture");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize timing clock fixture")
    else {
        panic!("timing clock fixture should normalize as family model");
    };

    let event_times = model
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.kind, FamilyNodeKind::TimingEvent) && node.alias.as_deref() == Some("S1")
        })
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(event_times, vec!["0", "50", "150"]);

    let svg = puml::render_source_to_svg(src).expect("render timing clock fixture");
    assert!(svg.contains(">@50</text>"));
    assert!(svg.contains(">@150</text>"));
}

#[test]
fn timing_anchor_referenced_messages_resolve_to_absolute_ticks() {
    let src = r#"@startuml
scale 5 as 150 pixels
clock clk with period 1
binary "enable" as en
concise "dataBus" as db
concise "address bus" as addr
@6 as :write_beg
@10 as :write_end
@0
en is low
db is "0x0"
addr is "0x03f"
@:write_beg-1
en is high
@:write_beg
db is "0xDEADBEEF"
@:write_end
en is low
db@:write_beg-1 -> addr@:write_end+1 : hold
@enduml
"#;
    let document = puml::parse(src).expect("parse anchored message fixture");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize anchored message fixture")
    else {
        panic!("anchored message fixture should normalize as family model");
    };

    assert!(model.relations.iter().any(|rel| {
        rel.from == "db@5" && rel.to == "addr@11" && rel.label.as_deref() == Some("hold")
    }));
    let svg = puml::render_source_to_svg(src).expect("render anchored message fixture");
    assert!(svg.contains("class=\"timing-message\""));
    assert!(svg.contains(">hold</text>"));
}

#[test]
fn timing_distributed_trace_fixture_renders_cross_lane_messages_and_cache_window() {
    let src = include_str!("fixtures/families/valid_timing_distributed_trace.puml");
    let svg = puml::render_source_to_svg(src).expect("render distributed trace timing fixture");

    let message_count = svg.matches("class=\"timing-message\"").count();
    assert!(
        message_count >= 4,
        "expected four distributed trace messages"
    );
    assert!(svg.contains("GET"));
    assert!(svg.contains("If-Modified-Since: 150"));
    assert!(svg.contains("200 OK"));
    assert!(svg.contains("304 Not Modified"));
    assert!(svg.contains("no need to re-request from server"));
    assert!(svg.contains("fresh"));
    assert!(svg.contains("stale"));
}

#[test]
fn timing_date_axis_values_render_as_ticks_and_states() {
    let src = r#"@startuml
robust "Web Browser" as WB
concise "Web User" as WU
@2019/07/02
WU is Idle
WB is Idle
@2019/07/04
WU is Waiting
WB is Processing
@2019/07/05
WB is Waiting
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("render dated timing fixture");

    assert!(svg.contains("@2019/07/02"));
    assert!(svg.contains("@2019/07/04"));
    assert!(svg.contains("@2019/07/05"));
    assert!(svg.contains("Waiting"));
    assert!(svg.contains("Processing"));
}
