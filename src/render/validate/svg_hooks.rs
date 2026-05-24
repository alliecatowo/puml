/// Parse `viewBox="x y w h"` from an SVG string.
/// Returns `(x, y, width, height)` as integers, or `None` if not found.
pub fn parse_viewbox(svg: &str) -> Option<(i32, i32, i32, i32)> {
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
pub(super) fn replace_viewbox(svg: &str, x: i32, y: i32, w: i32, h: i32) -> String {
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
pub(super) fn sync_svg_dimensions(svg: &str, vb_x: i32, vb_y: i32, vb_w: i32, vb_h: i32) -> String {
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

/// Text anchor kind extracted from a `<text>` element.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextAnchor {
    Start,
    Middle,
    End,
}

/// A scraped SVG `<text>` element relevant to render invariants.
#[derive(Clone)]
pub(super) struct TextElement {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) anchor: TextAnchor,
    pub(super) snippet: String,
    pub(super) is_edge_label: bool,
}

/// Extract every `<text …>` element from the SVG with its `x`, `y`,
/// `text-anchor`, a short content snippet, and whether the renderer marked it
/// as a relation label.
pub(super) fn extract_text_elements(svg: &str) -> Vec<TextElement> {
    let mut result = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("<text ") {
        let tag_start = pos + rel;
        // Find the closing `>` of the opening tag.
        let Some(rel_close) = svg[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + rel_close;
        let attrs = &svg[tag_start..tag_end];

        let x = parse_attr_i32(attrs, "x").unwrap_or(0);
        let y = parse_attr_i32(attrs, "y").unwrap_or(0);
        let anchor = if attrs.contains("text-anchor=\"middle\"") {
            TextAnchor::Middle
        } else if attrs.contains("text-anchor=\"end\"") {
            TextAnchor::End
        } else {
            TextAnchor::Start
        };

        // Grab a short snippet from the content.
        let content_start = tag_end + 1;
        let snippet_end = svg[content_start..]
            .find("</text>")
            .map(|r| content_start + r)
            .unwrap_or(content_start + 40.min(svg.len() - content_start));
        let snippet = svg[content_start..snippet_end.min(svg.len())]
            .chars()
            .take(40)
            .collect::<String>();
        let snippet = snippet
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"");

        let is_edge_label = attrs.contains("class=\"uml-edge-label")
            || attrs.contains("data-uml-label-role=\"edge\"");
        result.push(TextElement {
            x,
            y,
            anchor,
            snippet,
            is_edge_label,
        });
        pos = tag_start + 1;
    }
    result
}

/// Parse a named integer attribute `attr="value"` from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character or
/// the start of the string to avoid matching partial attribute names
/// (e.g. `y="` inside `data-uml-visibility="public"`).
pub(super) fn parse_attr_i32(tag: &str, attr: &str) -> Option<i32> {
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
            return tag[value_start..end].parse().ok();
        }
        search_pos = match_pos + needle.len();
    }
    None
}

pub(super) fn parse_attr_f64(tag: &str, attr: &str) -> Option<f64> {
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
            return tag[value_start..end].parse().ok();
        }
        search_pos = match_pos + needle.len();
    }
    None
}

pub(super) fn parse_attr_i32_lossy(tag: &str, attr: &str) -> Option<i32> {
    parse_attr_i32(tag, attr)
        .or_else(|| parse_attr_f64(tag, attr).map(|value| value.round() as i32))
}

/// Approximate character-width estimate in pixels at `font-size="12"`.
pub(super) const CHAR_WIDTH_PX: i32 = 7;
/// Approximate descent below the y baseline.
pub(super) const TEXT_DESCENT_PX: i32 = 4;
/// Approximate ascent above the y baseline.
pub(super) const TEXT_ASCENT_PX: i32 = 12;
