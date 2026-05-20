/// Parse `viewBox="x y w h"` from an SVG string.
/// Returns `(x, y, width, height)` as integers, or `None` if not found.
pub(crate) fn parse_viewbox(svg: &str) -> Option<(i32, i32, i32, i32)> {
    // Find `viewBox="…"` attribute.
    let start = svg.find("viewBox=\"")?;
    let inner_start = start + "viewBox=\"".len();
    let end = svg[inner_start..].find('"')? + inner_start;
    let parts: Vec<i32> = svg[inner_start..end]
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() == 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

/// Replace the `viewBox="…"` value in `svg` with the given dimensions.
pub(crate) fn replace_viewbox(svg: &str, x: i32, y: i32, w: i32, h: i32) -> String {
    let new_vb = format!("{x} {y} {w} {h}");
    // Replace first occurrence only (the root <svg> viewBox).
    if let Some(pos) = svg.find("viewBox=\"") {
        let inner_start = pos + "viewBox=\"".len();
        if let Some(rel_end) = svg[inner_start..].find('"') {
            let end = inner_start + rel_end;
            let mut result = svg[..inner_start].to_string();
            result.push_str(&new_vb);
            result.push_str(&svg[end..]);
            return result;
        }
    }
    svg.to_string()
}

/// Also update `width="…"` and `height="…"` on the root `<svg>` element to
/// match the new viewBox dimensions (prevents the SVG from being cropped when
/// the viewBox is expanded but the intrinsic size stays small).
pub(crate) fn sync_svg_dimensions(svg: &str, vb_x: i32, vb_y: i32, vb_w: i32, vb_h: i32) -> String {
    let svg = replace_viewbox(svg, vb_x, vb_y, vb_w, vb_h);
    // Update width="…" on the opening <svg> tag only.
    let svg = replace_root_attr(&svg, "width", &vb_w.to_string());
    replace_root_attr(&svg, "height", &vb_h.to_string())
}

/// Replace the value of `attr="…"` in the first tag of `svg`.
fn replace_root_attr(svg: &str, attr: &str, new_val: &str) -> String {
    let needle = format!("{attr}=\"");
    if let Some(pos) = svg.find(&needle) {
        let inner_start = pos + needle.len();
        if let Some(rel_end) = svg[inner_start..].find('"') {
            let end = inner_start + rel_end;
            let mut result = svg[..inner_start].to_string();
            result.push_str(new_val);
            result.push_str(&svg[end..]);
            return result;
        }
    }
    svg.to_string()
}

/// Parse a named integer attribute `attr="value"` from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character or
/// the start of the string to avoid matching partial attribute names
/// (e.g. `y="` inside `data-uml-visibility="public"`).
pub(crate) fn parse_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let mut search_pos = 0;
    while let Some(rel) = tag[search_pos..].find(&needle) {
        let match_pos = search_pos + rel;
        // Verify that the character before the attribute name is a whitespace,
        // '/' (for self-closing), or start-of-string — never a letter/digit.
        let ok = match_pos == 0
            || tag
                .as_bytes()
                .get(match_pos - 1)
                .map(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                .unwrap_or(false);
        let value_start = match_pos + needle.len();
        if ok {
            let end = value_start + tag[value_start..].find('"')?;
            let raw = &tag[value_start..end];
            return raw
                .parse::<i32>()
                .ok()
                .or_else(|| raw.parse::<f64>().ok().map(|value| value.round() as i32));
        }
        search_pos = match_pos + needle.len();
    }
    None
}

pub(crate) fn parse_bbox(raw: &str) -> Option<(i32, i32, i32, i32)> {
    let parts: Vec<i32> = raw
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f64>().ok().map(|value| value.round() as i32))
        .collect();
    if parts.len() == 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

pub(crate) fn parse_points_bbox(points: &str) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for pair in points.split_whitespace() {
        let Some((x, y)) = pair.split_once(',') else {
            continue;
        };
        let x = x.parse::<f64>().ok()?;
        let y = y.parse::<f64>().ok()?;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    min_x.is_finite().then_some((
        min_x.round() as i32,
        min_y.round() as i32,
        (max_x - min_x).round() as i32,
        (max_y - min_y).round() as i32,
    ))
}

pub(crate) fn svg_element_tags(svg: &str) -> Vec<&str> {
    svg_element_tags_with_pos(svg)
        .into_iter()
        .map(|(_, tag)| tag)
        .collect()
}

pub(crate) fn svg_element_tags_with_pos(svg: &str) -> Vec<(usize, &str)> {
    let mut tags = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find('<') {
        let start = pos + rel;
        if svg[start..].starts_with("</") || svg[start..].starts_with("<!--") {
            pos = start + 1;
            continue;
        }
        let Some(rel_end) = svg[start..].find('>') else {
            break;
        };
        tags.push((start, &svg[start..start + rel_end + 1]));
        pos = start + rel_end + 1;
    }
    tags
}

pub(crate) fn extract_text_content_at(svg: &str, tag_start: usize) -> Option<String> {
    if !svg[tag_start..].starts_with("<text ") {
        return None;
    }
    let tag_end = tag_start + svg[tag_start..].find('>')?;
    let content_start = tag_end + 1;
    let content_end = content_start + svg[content_start..].find("</text>")?;
    Some(
        svg[content_start..content_end]
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\""),
    )
}

pub(crate) fn tag_has_class(tag: &str, class_name: &str) -> bool {
    extract_attr_str(tag, "class")
        .map(|class| class.split_whitespace().any(|part| part == class_name))
        .unwrap_or(false)
}

/// Extract a string attribute value from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character to
/// avoid false matches on suffix strings (e.g. `id` inside `data-uml-id`).
pub(crate) fn extract_attr_str(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let mut search_pos = 0;
    while let Some(rel) = tag[search_pos..].find(&needle) {
        let match_pos = search_pos + rel;
        let ok = match_pos == 0
            || tag
                .as_bytes()
                .get(match_pos - 1)
                .map(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                .unwrap_or(false);
        let value_start = match_pos + needle.len();
        if ok {
            let end = value_start + tag[value_start..].find('"')?;
            return Some(tag[value_start..end].to_string());
        }
        search_pos = match_pos + needle.len();
    }
    None
}
