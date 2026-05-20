pub(crate) const MESSAGE_LABEL_LINE_GAP: i32 = 16;

pub(crate) fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

pub(crate) fn docs_example(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/docs/examples/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[derive(Debug, Clone)]
pub(crate) struct SvgRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) fill: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SvgText {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SvgLine {
    pub(crate) x1: i32,
    pub(crate) y1: i32,
    pub(crate) x2: i32,
    pub(crate) y2: i32,
    pub(crate) stroke: String,
    pub(crate) stroke_width: i32,
    pub(crate) dash: Option<String>,
    pub(crate) visibility: Option<String>,
    pub(crate) marker_end: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) direction: Option<String>,
    pub(crate) relation_style: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SvgCircle {
    pub(crate) stroke: String,
    pub(crate) class: Option<String>,
}

pub(crate) fn parse_svg_attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    for (start, _) in tag.match_indices(&pat) {
        let is_attr_boundary = start == 0
            || tag[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        if !is_attr_boundary {
            continue;
        }
        let rest = &tag[start + pat.len()..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    None
}

pub(crate) fn parse_svg_lines(svg: &str) -> Vec<SvgLine> {
    let mut lines = Vec::new();
    for tag in parse_svg_line_tags(svg) {
        let (Some(x1), Some(y1), Some(x2), Some(y2), Some(stroke), Some(stroke_width)) = (
            parse_svg_attr(tag, "x1").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "y1").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "x2").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "y2").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "stroke"),
            parse_svg_attr(tag, "stroke-width").and_then(|v| v.parse::<i32>().ok()),
        ) else {
            continue;
        };
        lines.push(SvgLine {
            x1,
            y1,
            x2,
            y2,
            stroke,
            stroke_width,
            dash: parse_svg_attr(tag, "stroke-dasharray"),
            visibility: parse_svg_attr(tag, "visibility"),
            marker_end: parse_svg_attr(tag, "marker-end"),
            from: parse_svg_attr(tag, "data-uml-from"),
            to: parse_svg_attr(tag, "data-uml-to"),
            direction: parse_svg_attr(tag, "data-uml-direction"),
            relation_style: parse_svg_attr(tag, "data-uml-relation-style"),
        });
    }
    lines
}

fn parse_svg_line_tags(svg: &str) -> Vec<&str> {
    parse_svg_self_closing_tags(svg, "line")
}

pub(crate) fn parse_svg_rect_tags(svg: &str) -> Vec<&str> {
    parse_svg_self_closing_tags(svg, "rect")
}

fn parse_svg_self_closing_tags<'a>(svg: &'a str, tag_name: &str) -> Vec<&'a str> {
    let needle = format!("<{tag_name} ");
    svg.split(&needle)
        .skip(1)
        .filter_map(|chunk| chunk.find("/>").map(|end| &chunk[..end]))
        .collect()
}

pub(crate) fn parse_svg_circles(svg: &str) -> Vec<SvgCircle> {
    parse_svg_self_closing_tags(svg, "circle")
        .into_iter()
        .filter_map(|tag| {
            Some(SvgCircle {
                stroke: parse_svg_attr(tag, "stroke")?,
                class: parse_svg_attr(tag, "class"),
            })
        })
        .collect()
}

pub(crate) fn parse_svg_rects(svg: &str) -> Vec<SvgRect> {
    let mut rects = Vec::new();
    for chunk in svg.split("<rect ").skip(1) {
        let Some(end) = chunk.find("/>") else {
            continue;
        };
        let tag = &chunk[..end];
        let (Some(x), Some(y), Some(width), Some(height), Some(fill)) = (
            parse_svg_attr(tag, "x").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "y").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "width").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "height").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "fill"),
        ) else {
            continue;
        };
        rects.push(SvgRect {
            x,
            y,
            width,
            height,
            fill,
        });
    }
    for chunk in svg.split("<path ").skip(1) {
        let Some(end) = chunk.find("/>") else {
            continue;
        };
        let tag = &chunk[..end];
        let (Some(d), Some(fill)) = (parse_svg_attr(tag, "d"), parse_svg_attr(tag, "fill")) else {
            continue;
        };
        if fill == "none" || !d.starts_with('M') {
            continue;
        }
        let parts = d.split_whitespace().collect::<Vec<_>>();
        let Some(start) = parts.first().and_then(|p| p.strip_prefix('M')) else {
            continue;
        };
        let Some((x, y)) = start.split_once(',') else {
            continue;
        };
        let (Ok(x), Ok(y)) = (x.parse::<i32>(), y.parse::<i32>()) else {
            continue;
        };
        let mut max_x = x;
        let mut max_y = y;
        for part in parts.iter().skip(1) {
            if let Some(value) = part.strip_prefix('H') {
                if let Ok(value) = value.parse::<i32>() {
                    max_x = max_x.max(value);
                }
            } else if let Some(value) = part.strip_prefix('V') {
                if let Ok(value) = value.parse::<i32>() {
                    max_y = max_y.max(value);
                }
            } else if let Some(value) = part.strip_prefix('L') {
                if let Ok(value) = value.parse::<i32>() {
                    max_x = max_x.max(value);
                }
            }
        }
        rects.push(SvgRect {
            x,
            y,
            width: max_x - x,
            height: max_y - y,
            fill,
        });
    }
    for chunk in svg.split("<polygon ").skip(1) {
        let Some(end) = chunk.find("/>") else {
            continue;
        };
        let tag = &chunk[..end];
        let (Some(points), Some(fill)) =
            (parse_svg_attr(tag, "points"), parse_svg_attr(tag, "fill"))
        else {
            continue;
        };
        let coords = points
            .split(|c: char| !c.is_ascii_digit() && c != '-')
            .filter_map(|n| n.parse::<i32>().ok())
            .collect::<Vec<_>>();
        if coords.len() < 6 {
            continue;
        }
        let xs = coords.iter().step_by(2).copied().collect::<Vec<_>>();
        let ys = coords
            .iter()
            .skip(1)
            .step_by(2)
            .copied()
            .collect::<Vec<_>>();
        let (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) = (
            xs.iter().min(),
            xs.iter().max(),
            ys.iter().min(),
            ys.iter().max(),
        ) else {
            continue;
        };
        rects.push(SvgRect {
            x: *min_x,
            y: *min_y,
            width: max_x - min_x,
            height: max_y - min_y,
            fill,
        });
    }
    rects
}

pub(crate) fn parse_svg_texts(svg: &str) -> Vec<SvgText> {
    let mut texts = Vec::new();
    for chunk in svg.split("<text ").skip(1) {
        let Some(close) = chunk.find('>') else {
            continue;
        };
        let attrs = &chunk[..close];
        let body = &chunk[close + 1..];
        let Some(end) = body.find("</text>") else {
            continue;
        };
        let content = body[..end].to_string();
        let (Some(x), Some(y)) = (
            parse_svg_attr(attrs, "x").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(attrs, "y").and_then(|v| v.parse::<i32>().ok()),
        ) else {
            continue;
        };
        texts.push(SvgText {
            x,
            y,
            text: content,
        });
    }
    texts
}

pub(crate) fn parse_svg_viewbox_width(svg: &str) -> Option<i32> {
    let svg_tag = svg.split("<svg ").nth(1)?.split('>').next()?;
    let viewbox = parse_svg_attr(svg_tag, "viewBox")?;
    let mut parts = viewbox.split_whitespace();
    let _min_x = parts.next()?;
    let _min_y = parts.next()?;
    let width = parts.next()?.parse::<i32>().ok()?;
    Some(width)
}
