use puml::model::{FamilyNodeKind, NormalizedDocument};

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

fn time_to_x(width: i32, t: i32) -> i32 {
    let left_pad = 130;
    let right_pad = 32;
    let chart_w = width - left_pad - right_pad;
    left_pad + ((t as f64 / 18.0) * chart_w as f64) as i32
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
    let (view_w, _view_h) = svg_viewbox(&svg).expect("timing svg should have viewBox");

    let range_rects = svg_rects(&svg)
        .into_iter()
        .filter(|rect| rect.class.as_deref() == Some("timing-range"))
        .collect::<Vec<_>>();
    assert_eq!(range_rects.len(), 2, "expected range and highlight bands");

    let active = range_rects
        .iter()
        .find(|rect| rect.x == time_to_x(view_w, 5))
        .expect("active window should start at @5");
    assert_eq!(active.width, time_to_x(view_w, 12) - time_to_x(view_w, 5));
    assert!(
        active.height > 48,
        "range band should span axis plus signal rows"
    );

    let cooldown = range_rects
        .iter()
        .find(|rect| rect.x == time_to_x(view_w, 12))
        .expect("highlight should start at @12");
    assert_eq!(
        cooldown.width,
        time_to_x(view_w, 18) - time_to_x(view_w, 12)
    );
    assert_eq!(
        cooldown.y, active.y,
        "range/highlight bands share axis origin"
    );

    let range_labels = svg_texts(&svg)
        .into_iter()
        .filter(|text| text.class.as_deref() == Some("timing-range-label"))
        .collect::<Vec<_>>();
    assert!(
        range_labels
            .iter()
            .any(|text| text.text == "active window" && text.x == active.x + active.width / 2),
        "active window label should be centered in its band"
    );
    assert!(
        range_labels
            .iter()
            .any(|text| text.text == "cooldown" && text.x == cooldown.x + cooldown.width / 2),
        "highlight label should be centered in its band"
    );

    let polylines = polyline_tags(&svg);
    let clock = polylines
        .iter()
        .find(|tag| svg_attr(tag, "data-timing-period").as_deref() == Some("6"))
        .expect("clock polyline should expose period metadata");
    assert_eq!(svg_attr(clock, "data-timing-pulse").as_deref(), Some("2"));
    assert_eq!(svg_attr(clock, "data-timing-offset").as_deref(), Some("0"));
    let clock_points = parse_points(&svg_attr(clock, "points").expect("clock points"));
    let clock_edge_x = time_to_x(view_w, 2);
    assert!(
        clock_points
            .iter()
            .filter(|(x, _)| *x == clock_edge_x)
            .count()
            >= 2,
        "clock pulse width should place a vertical transition at @2"
    );

    let binary = polylines
        .iter()
        .find(|tag| svg_attr(tag, "data-timing-period").is_none())
        .expect("binary waveform polyline should render separately from clock");
    let binary_points = parse_points(&svg_attr(binary, "points").expect("binary points"));
    for t in [5, 8] {
        let edge_x = time_to_x(view_w, t);
        let ys = binary_points
            .iter()
            .filter_map(|(x, y)| (*x == edge_x).then_some(*y))
            .collect::<Vec<_>>();
        assert!(
            ys.len() >= 2 && ys.iter().min() != ys.iter().max(),
            "binary EN should have vertical waveform transition at @{t}: {ys:?}"
        );
    }

    let polygons = polygon_points(&svg);
    assert!(
        polygons.iter().any(|points| {
            let min_x = points.iter().map(|(x, _)| *x).min().unwrap_or_default();
            let max_x = points.iter().map(|(x, _)| *x).max().unwrap_or_default();
            min_x == time_to_x(view_w, 0) && max_x == time_to_x(view_w, 5)
        }),
        "robust BUS state geometry should cover @0..@5"
    );
}
