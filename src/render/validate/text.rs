use super::svg::{parse_attr_i32, parse_viewbox, sync_svg_dimensions};
use super::{AutoCorrect, InvariantKind, InvariantViolation};

/// Text anchor kind extracted from a `<text>` element.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextAnchor {
    Start,
    Middle,
    End,
}

/// Approximate character-width estimate in pixels at `font-size="12"`.
pub(crate) const CHAR_WIDTH_PX: i32 = 7;
/// Approximate descent below the y baseline.
pub(crate) const TEXT_DESCENT_PX: i32 = 4;
/// Approximate ascent above the y baseline.
pub(crate) const TEXT_ASCENT_PX: i32 = 12;

/// Extract every `<text …>` element from the SVG with its `x`, `y`,
/// `text-anchor`, and a short content snippet.
/// Returns `(x, y, anchor, snippet)`.
pub(crate) fn extract_text_elements(svg: &str) -> Vec<(i32, i32, TextAnchor, String)> {
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

        result.push((x, y, anchor, snippet));
        pos = tag_start + 1;
    }
    result
}

/// Check that every `<text>` element's estimated bounding box fits inside the
/// current viewBox.  If it overflows to the right or bottom, expand the viewBox
/// to contain it (auto-correct) and return the number of expansions applied.
///
/// This is invariant #2.
pub fn check_labels_inside_viewbox(svg: &mut String, mode: AutoCorrect) -> Vec<InvariantViolation> {
    let Some((vb_x, vb_y, mut vb_w, mut vb_h)) = parse_viewbox(svg) else {
        return Vec::new();
    };

    let texts = extract_text_elements(svg);
    let mut violations = Vec::new();
    let mut expanded = false;

    for (tx, ty, anchor, snippet) in &texts {
        let text_len: i32 = snippet.chars().count() as i32;
        let half_w = text_len * CHAR_WIDTH_PX / 2;
        // Compute the actual left/right edges depending on text-anchor.
        let (text_left, text_right) = match anchor {
            TextAnchor::Middle => (tx - half_w, tx + half_w),
            TextAnchor::End => (tx - text_len * CHAR_WIDTH_PX, *tx),
            TextAnchor::Start => (*tx, tx + text_len * CHAR_WIDTH_PX),
        };
        let text_bottom = ty + TEXT_DESCENT_PX;
        let text_top = ty - TEXT_ASCENT_PX;

        let left_overflow = (vb_x - text_left).max(0);
        let right_overflow = (text_right - (vb_x + vb_w)).max(0);
        let bottom_overflow = (text_bottom - (vb_y + vb_h)).max(0);
        let top_overflow = (vb_y - text_top).max(0);

        if left_overflow > 0 || right_overflow > 0 || bottom_overflow > 0 || top_overflow > 0 {
            let overflow_px = left_overflow
                .max(right_overflow)
                .max(bottom_overflow)
                .max(top_overflow);
            violations.push(InvariantViolation {
                kind: InvariantKind::LabelOutsideViewbox {
                    snippet: snippet.clone(),
                    overflow_px,
                },
                corrected: matches!(mode, AutoCorrect::Apply),
                message: format!(
                    "[INV-2] label {:?} overflows viewBox by {}px",
                    &snippet[..snippet.len().min(20)],
                    overflow_px
                ),
            });

            if matches!(mode, AutoCorrect::Apply) {
                // Expand viewBox to contain the overflow.
                if left_overflow > 0 {
                    let new_x = vb_x - left_overflow - 8;
                    vb_w += vb_x - new_x;
                    // vb_x = new_x; // keep vb_x stable; just expand width
                }
                vb_w = vb_w.max(text_right - vb_x + 8);
                vb_h = vb_h.max(text_bottom - vb_y + 8);
                if top_overflow > 0 {
                    vb_h += top_overflow;
                }
                expanded = true;
            }
        }
    }

    if expanded {
        *svg = sync_svg_dimensions(svg, vb_x, vb_y, vb_w, vb_h);
    }

    violations
}
