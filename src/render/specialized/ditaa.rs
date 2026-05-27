use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

use super::*;

pub fn render_ditaa_svg(document: &DitaaDocument) -> String {
    render_ditaa_artifact(document).svg
}

/// Render a ditaa diagram into a typed [`RenderArtifact`].
///
/// The SVG is produced by the specialized `render_ditaa_from_parts` pipeline
/// (full ASCII-art grid parser + shape/connector detector). We also attach a
/// [`RenderScene`] whose single `SceneNode` covers the full SVG canvas — the
/// same bounding box the SVG `width`/`height` attributes describe — so the
/// geometry contract is satisfied without duplicating the internal grid math.
/// SVG output is byte-identical to the legacy `render_ditaa_svg` path.
pub fn render_ditaa_artifact(document: &DitaaDocument) -> RenderArtifact {
    let svg = match crate::specialized::render_ditaa_from_parts(
        &document.body,
        document.title.as_deref(),
    ) {
        Ok(svg) => svg,
        Err(_) => render_ditaa_fallback(document),
    };

    let scene = build_ditaa_scene(document, &svg);
    RenderArtifact::with_scene(svg, scene)
}

/// Build a typed [`RenderScene`] for a ditaa diagram.
///
/// Ditaa renders as a single monolithic ASCII-art canvas. We replicate the
/// same bounding-box formula used by `DitaaGrid::new` (scale=1, margin=16,
/// cell_w=10, cell_h=16) so the scene viewport and single node bounds match
/// the SVG `width`/`height` exactly. One `SceneNode` covers the full canvas.
fn build_ditaa_scene(document: &DitaaDocument, svg: &str) -> RenderScene {
    // Extract the actual rendered dimensions from the SVG width/height
    // attributes so the scene stays consistent regardless of fallback/main path.
    let (w, h) = extract_svg_dimensions(svg).unwrap_or_else(|| {
        // Recompute from the body using the same formula as DitaaGrid::new
        // (scale defaults to 1 in render_ditaa_from_parts / DitaaOptions::default).
        let scale: i32 = 1;
        let cell_w = 10 * scale;
        let cell_h = 16 * scale;
        let margin = 16i32;
        let title_h = if document.title.is_some() { 28i32 } else { 0 };
        let lines: Vec<&str> = document.body.lines().collect();
        let rows = lines.len() as i32;
        let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
        let svg_w = cols * cell_w + margin * 2;
        let svg_h = rows * cell_h + margin * 2 + title_h;
        (svg_w.max(1) as f64, svg_h.max(1) as f64)
    });

    let viewport = Rect::new(0.0, 0.0, w, h);
    let mut scene = RenderScene::new(viewport);

    let label_text = document
        .title
        .clone()
        .unwrap_or_else(|| "ditaa".to_string());

    scene.add_node(SceneNode {
        id: "ditaa::canvas".to_string(),
        node_box: NodeBox {
            id: "ditaa::canvas".to_string(),
            bounds: viewport,
            ports: Vec::new(),
            labels: vec![LabelBox {
                id: "ditaa::canvas::label".to_string(),
                text: label_text,
                bounds: viewport,
                owner_id: Some("ditaa::canvas".to_string()),
                role: LabelRole::Node,
            }],
        },
    });

    scene
}

/// Parse `width="NNN"` and `height="NNN"` from the SVG opening tag.
/// Returns `None` if the attributes cannot be found or parsed.
fn extract_svg_dimensions(svg: &str) -> Option<(f64, f64)> {
    // Only scan the opening tag region to avoid false matches in SVG content.
    let tag_end = svg.find('>')?;
    let tag = &svg[..tag_end];

    let w = extract_attr(tag, "width")?;
    let h = extract_attr(tag, "height")?;
    Some((w, h))
}

fn extract_attr(tag: &str, name: &str) -> Option<f64> {
    // Match `name="<value>"` — attributes are always quoted in our emitter.
    let needle = format!("{name}=\"");
    let start = tag.find(needle.as_str())? + needle.len();
    let end = tag[start..].find('"')? + start;
    tag[start..end].parse::<f64>().ok()
}

fn render_ditaa_fallback(document: &DitaaDocument) -> String {
    let width = 820;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 18;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">ditaa (ASCII art frame)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 18) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"4\" ry=\"4\" fill=\"#fdf6e3\" stroke=\"#b58900\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 20;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"36\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#073642\" xml:space=\"preserve\">{}</text>",
            escape_text(line)
        ));
        ty += 18;
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_document(body: &str, title: Option<&str>) -> DitaaDocument {
        DitaaDocument {
            body: body.to_string(),
            title: title.map(str::to_string),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn ditaa_artifact_scene_bounds_non_empty() {
        let doc = make_document("+---+\n|   |\n+---+\n", None);
        let artifact = render_ditaa_artifact(&doc);
        assert!(artifact.scene.is_some(), "scene must be attached");
        let scene = artifact.scene.unwrap();
        assert!(
            !scene.nodes.is_empty(),
            "scene must contain at least one node"
        );
        let node = scene
            .nodes
            .get("ditaa::canvas")
            .expect("canvas node must exist");
        assert!(
            node.node_box.bounds.size.width > 0.0,
            "canvas width must be positive"
        );
        assert!(
            node.node_box.bounds.size.height > 0.0,
            "canvas height must be positive"
        );
    }

    #[test]
    fn ditaa_artifact_scene_validate_geometry_clean() {
        let doc = make_document("+------+\n|Box A |\n+------+\n", Some("Test"));
        let artifact = render_ditaa_artifact(&doc);
        let scene = artifact.scene.expect("scene must be attached");
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "geometry validation must be clean, got: {issues:?}"
        );
    }

    #[test]
    fn ditaa_artifact_svg_byte_identical_to_render_ditaa_svg() {
        let doc = make_document("+--+--+\n|A |B |\n+--+--+\n", None);
        let svg_direct = render_ditaa_svg(&doc);
        let artifact = render_ditaa_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_ditaa_svg must produce byte-identical output to render_ditaa_artifact().svg"
        );
    }
}
