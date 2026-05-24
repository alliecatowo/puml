use super::geometry::{
    extract_node_bboxes, extract_package_frames, extract_relation_segments, NodeBbox, PackageFrame,
    Segment,
};
use super::svg_hooks::{
    extract_text_elements, parse_viewbox, TextAnchor, TextElement, CHAR_WIDTH_PX, TEXT_ASCENT_PX,
    TEXT_DESCENT_PX,
};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ContentBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct QualityMetrics {
    pub viewbox_width: i32,
    pub viewbox_height: i32,
    pub aspect_ratio: f64,
    pub node_count: usize,
    pub relation_count: usize,
    pub package_count: usize,
    pub text_count: usize,
    pub route_length_px: f64,
    pub route_length_per_node_px: f64,
    pub content_bounds: Option<ContentBounds>,
    pub max_empty_gutter_ratio: f64,
}

/// Collect visual quality metrics from SVG output.
///
/// These are intentionally non-fatal first-pass metrics for #1113/#594. Tests
/// can promote thresholds gradually as layout fixes land.
pub fn collect_quality_metrics(svg: &str) -> QualityMetrics {
    let (_, _, viewbox_width, viewbox_height) = parse_viewbox(svg).unwrap_or_default();
    let nodes = extract_node_bboxes(svg);
    let relations = extract_relation_segments(svg);
    let packages = extract_package_frames(svg);
    let texts = extract_text_elements(svg);
    let route_length_px = relations
        .iter()
        .flat_map(|(_, _, segs)| segs.iter())
        .map(|seg| {
            let dx = (seg.x2 - seg.x1) as f64;
            let dy = (seg.y2 - seg.y1) as f64;
            dx.hypot(dy)
        })
        .sum::<f64>();
    let content_bounds = collect_content_bounds(&nodes, &relations, &packages, &texts);
    let max_empty_gutter_ratio = content_bounds.map_or(0.0, |bounds| {
        max_empty_gutter_ratio(viewbox_width, viewbox_height, bounds)
    });
    let aspect_ratio = if viewbox_width > 0 && viewbox_height > 0 {
        let w = viewbox_width as f64;
        let h = viewbox_height as f64;
        w.max(h) / w.min(h)
    } else {
        0.0
    };

    QualityMetrics {
        viewbox_width,
        viewbox_height,
        aspect_ratio,
        node_count: nodes.len(),
        relation_count: relations.len(),
        package_count: packages.len(),
        text_count: texts.len(),
        route_length_px,
        route_length_per_node_px: route_length_px / nodes.len().max(1) as f64,
        content_bounds,
        max_empty_gutter_ratio,
    }
}

fn collect_content_bounds(
    nodes: &[NodeBbox],
    relations: &[(String, String, Vec<Segment>)],
    packages: &[PackageFrame],
    texts: &[TextElement],
) -> Option<ContentBounds> {
    let mut bounds = Vec::new();
    bounds.extend(
        nodes
            .iter()
            .map(|node| (node.x, node.y, node.x + node.w, node.y + node.h)),
    );
    bounds.extend(packages.iter().map(|frame| {
        (
            frame.x,
            frame.y,
            frame.x + frame.width,
            frame.y + frame.header_height,
        )
    }));
    for (_, _, segs) in relations {
        bounds.extend(segs.iter().map(|seg| {
            (
                seg.x1.min(seg.x2),
                seg.y1.min(seg.y2),
                seg.x1.max(seg.x2),
                seg.y1.max(seg.y2),
            )
        }));
    }
    bounds.extend(texts.iter().map(text_bounds));
    let min_x = bounds.iter().map(|(x, _, _, _)| *x).min()?;
    let min_y = bounds.iter().map(|(_, y, _, _)| *y).min()?;
    let max_x = bounds.iter().map(|(_, _, x, _)| *x).max()?;
    let max_y = bounds.iter().map(|(_, _, _, y)| *y).max()?;
    Some(ContentBounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

fn text_bounds(text: &TextElement) -> (i32, i32, i32, i32) {
    let text_len = text.snippet.chars().count() as i32;
    let text_width = text_len * CHAR_WIDTH_PX;
    let (left, right) = match text.anchor {
        TextAnchor::Middle => (text.x - text_width / 2, text.x + text_width / 2),
        TextAnchor::End => (text.x - text_width, text.x),
        TextAnchor::Start => (text.x, text.x + text_width),
    };
    (
        left,
        text.y - TEXT_ASCENT_PX,
        right,
        text.y + TEXT_DESCENT_PX,
    )
}

fn max_empty_gutter_ratio(viewbox_width: i32, viewbox_height: i32, bounds: ContentBounds) -> f64 {
    if viewbox_width <= 0 || viewbox_height <= 0 {
        return 0.0;
    }
    let left = bounds.x.max(0) as f64 / viewbox_width as f64;
    let right = (viewbox_width - (bounds.x + bounds.width)).max(0) as f64 / viewbox_width as f64;
    let top = bounds.y.max(0) as f64 / viewbox_height as f64;
    let bottom =
        (viewbox_height - (bounds.y + bounds.height)).max(0) as f64 / viewbox_height as f64;
    left.max(right).max(top).max(bottom)
}
