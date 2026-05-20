#[derive(Debug, PartialEq, Eq)]
pub(super) struct SvgRectGeom {
    pub(super) x: i32,
    pub(super) y: i32,
}

pub(super) fn svg_rect_y(svg: &str, rect_needle: &str, following_text: &str) -> Option<i32> {
    let text_ix = svg.find(following_text)?;
    let before_text = &svg[..text_ix];
    let rect_ix = before_text.rfind(rect_needle)?;
    let tag = before_text[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "y")
}

pub(super) fn svg_node_rect(svg: &str, name: &str, addresses: &str) -> Option<SvgRectGeom> {
    let mut rest = svg;
    let name_attr = format!("data-nwdiag-name=\"{name}\"");
    let addresses_attr = format!("data-nwdiag-addresses=\"{addresses}\"");
    while let Some(ix) = rest.find("<rect class=\"nwdiag-node\"") {
        rest = &rest[ix..];
        let tag = rest.split_once('>')?.0;
        if tag.contains(&name_attr) && tag.contains(&addresses_attr) {
            return Some(SvgRectGeom {
                x: svg_attr_i32(tag, "x")?,
                y: svg_attr_i32(tag, "y")?,
            });
        }
        rest = &rest["<rect".len()..];
    }
    None
}

pub(super) fn svg_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!(" {attr}=\"");
    let rest = tag.split_once(&needle)?.1;
    let value = rest.split_once('"')?.0;
    value.parse::<f64>().ok().map(|value| value.round() as i32)
}

pub(super) fn svg_text_positions(svg: &str, text: &str) -> Vec<(i32, i32)> {
    let mut positions = Vec::new();
    let mut start = 0usize;
    while let Some(rel_ix) = svg[start..].find("<text ") {
        let tag_start = start + rel_ix;
        let Some(open_end) = svg[tag_start..].find('>').map(|ix| tag_start + ix) else {
            break;
        };
        let tag = svg[tag_start..]
            .split_once('>')
            .map(|(tag, _)| tag)
            .unwrap_or("");
        let content_start = open_end + 1;
        let Some(close_rel) = svg[content_start..].find("</text>") else {
            break;
        };
        let close = content_start + close_rel;
        let raw_text = svg_text_content(&svg[content_start..close]);
        let decoded_text = decode_svg_text_entities(&raw_text);
        start = close + "</text>".len();

        let decoded_expected = decode_svg_text_entities(text);
        if !raw_text.contains(text) && !decoded_text.contains(&decoded_expected) {
            continue;
        }

        let Some(x) = svg_attr_i32(tag, "x") else {
            continue;
        };
        let Some(y) = svg_attr_i32(tag, "y") else {
            continue;
        };
        positions.push((x, y));
    }
    positions
}

pub(super) fn svg_text_content(raw: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text
}

pub(super) fn decode_svg_text_entities(raw: &str) -> String {
    raw.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

pub(super) fn svg_relation_element<'a>(svg: &'a str, from: &str, to: &str) -> Option<&'a str> {
    let from_attr = format!("data-uml-from=\"{from}\"");
    let to_attr = format!("data-uml-to=\"{to}\"");
    svg.split('<')
        .find(|element| element.contains(&from_attr) && element.contains(&to_attr))
}

pub(super) fn svg_relation_end(element: &str) -> Option<(i32, i32)> {
    if let Some((_, points_rest)) = element.split_once("points=\"") {
        let points = points_rest.split_once('"')?.0;
        let last = points.split_whitespace().last()?;
        let (x, y) = last.split_once(',')?;
        return Some((x.parse().ok()?, y.parse().ok()?));
    }
    Some((svg_attr_i32(element, "x2")?, svg_attr_i32(element, "y2")?))
}

pub(super) fn state_svg_element_after_metadata<'a>(
    doc: &'a roxmltree::Document<'a>,
    node_name: &str,
) -> roxmltree::Node<'a, 'a> {
    doc.descendants()
        .find(|node| {
            node.has_tag_name("metadata") && node.attribute("data-state-node") == Some(node_name)
        })
        .and_then(|node| node.next_sibling_element())
        .unwrap_or_else(|| panic!("missing rendered element for state node {node_name}"))
}

pub(super) fn state_svg_attr_i32(node: roxmltree::Node<'_, '_>, attr: &str) -> i32 {
    node.attribute(attr)
        .unwrap_or_else(|| panic!("missing attribute {attr}"))
        .parse::<i32>()
        .unwrap_or_else(|_| panic!("invalid integer attribute {attr}"))
}

pub(super) fn state_svg_center_x(node: roxmltree::Node<'_, '_>) -> i32 {
    match node.tag_name().name() {
        "rect" => state_svg_attr_i32(node, "x") + state_svg_attr_i32(node, "width") / 2,
        "circle" => state_svg_attr_i32(node, "cx"),
        "polygon" => {
            let points = node
                .attribute("points")
                .unwrap_or_else(|| panic!("missing polygon points"));
            let xs = points
                .split_whitespace()
                .filter_map(|pair| pair.split_once(','))
                .map(|(x, _)| x.parse::<i32>().expect("polygon x should be an integer"))
                .collect::<Vec<_>>();
            let min_x = xs
                .iter()
                .min()
                .copied()
                .expect("polygon should have x points");
            let max_x = xs
                .iter()
                .max()
                .copied()
                .expect("polygon should have x points");
            (min_x + max_x) / 2
        }
        other => panic!("unsupported state SVG node for center extraction: {other}"),
    }
}

/// Extract start (x1,y1) and end (x2,y2) coordinates from a state transition `<path>`
/// element. The `d` attribute has the form `M x y [L x y]*`.
/// Returns (x1, y1, x2, y2) — the first and last coordinate pairs.
pub(super) fn state_path_endpoints(node: roxmltree::Node<'_, '_>) -> (i32, i32, i32, i32) {
    let d = node
        .attribute("d")
        .unwrap_or_else(|| panic!("state transition path should have d attribute"));
    let nums: Vec<i32> = d
        .split_ascii_whitespace()
        .filter_map(|tok| tok.parse::<i32>().ok())
        .collect();
    assert!(
        nums.len() >= 4,
        "state transition path d should have at least two coordinate pairs; d={d:?}"
    );
    let x1 = nums[0];
    let y1 = nums[1];
    let x2 = nums[nums.len() - 2];
    let y2 = nums[nums.len() - 1];
    (x1, y1, x2, y2)
}

pub(super) fn extract_svg_width_attr(svg: &str) -> Option<i32> {
    let key = "width=\"";
    let start = svg.find(key)? + key.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].parse::<i32>().ok()
}

pub(super) fn svg_elements_with_attr<'a>(svg: &'a str, attr: &str, value: &str) -> Vec<&'a str> {
    let needle = format!("{attr}=\"{value}\"");
    svg.split('<')
        .filter(|element| element.contains(&needle))
        .collect()
}

pub(super) fn svg_elements_with_class<'a>(
    svg: &'a str,
    tag: &str,
    class_name: &str,
) -> Vec<&'a str> {
    svg.split('<')
        .filter(|element| {
            svg_element_has_tag(element, tag) && svg_element_has_class(element, class_name)
        })
        .collect()
}

pub(super) fn svg_element_has_tag(element: &str, tag: &str) -> bool {
    element.strip_prefix(tag).is_some_and(|rest| {
        rest.starts_with(char::is_whitespace) || rest.starts_with('>') || rest.starts_with('/')
    })
}

pub(super) fn svg_element_has_class(element: &str, class_name: &str) -> bool {
    let Some((_, rest)) = element.split_once("class=\"") else {
        return false;
    };
    let Some((classes, _)) = rest.split_once('"') else {
        return false;
    };
    classes.split_whitespace().any(|class| class == class_name)
}

pub(super) fn svg_attr_i32_required(element: &str, attr: &str) -> i32 {
    svg_attr_i32(element, attr)
        .unwrap_or_else(|| panic!("expected numeric SVG attr {attr:?} in {element}"))
}

pub(super) fn svg_group_with_attr<'a>(svg: &'a str, attr: &str, value: &str) -> &'a str {
    let needle = format!("{attr}=\"{value}\"");
    let start = svg.find(&needle).expect("group attribute");
    let rest = &svg[start..];
    let end = rest.find("</g>").expect("group close") + "</g>".len();
    &rest[..end]
}
