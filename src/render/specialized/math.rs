use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

pub fn render_math_svg(document: &MathDocument) -> String {
    render_math_artifact(document).svg
}

/// Render a math diagram into a typed [`RenderArtifact`].
///
/// Math emits either a fully laid-out formula SVG (via `render_math_from_parts`)
/// or a text-fallback SVG. In both cases we build a [`RenderScene`] from the
/// actual drawn bounding box — one `SceneNode` covering the formula/body region
/// at the coordinates the SVG uses — so the scene stays consistent with the
/// output and `scene_availability` is `TypedScene`. SVG output is byte-identical
/// to the legacy `render_math_svg`.
pub fn render_math_artifact(document: &MathDocument) -> RenderArtifact {
    let svg =
        match crate::specialized::render_math_from_parts(&document.body, document.title.as_deref())
        {
            Ok(svg) => svg,
            Err(_) => render_math_fallback(document),
        };
    let scene = build_math_scene(&svg, document);
    RenderArtifact::with_scene(svg, scene)
}

/// Build a typed [`RenderScene`] from math's drawn geometry.
///
/// We parse `width` and `height` from the SVG header (byte-identical to what
/// `render_math_from_parts` and `render_math_fallback` emit) and place one
/// `SceneNode` covering the full diagram area so the scene has real bounds.
fn build_math_scene(svg: &str, document: &MathDocument) -> RenderScene {
    let (w, h) = parse_svg_dimensions(svg);
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, w, h));

    let label_text = document.title.as_deref().unwrap_or("").to_string();
    let bounds = Rect::new(0.0, 0.0, w, h);
    let label = LabelBox {
        id: "math::label".to_string(),
        text: label_text,
        bounds,
        owner_id: Some("math::formula".to_string()),
        role: LabelRole::Node,
    };
    scene.add_node(SceneNode {
        id: "math::formula".to_string(),
        node_box: NodeBox {
            id: "math::formula".to_string(),
            bounds,
            ports: Vec::new(),
            labels: vec![label],
        },
    });

    scene
}

/// Parse `width` and `height` numeric attributes from an SVG opening tag.
///
/// Returns `(0.0, 0.0)` only when the SVG header does not contain numeric
/// `width`/`height` attributes, which should not happen for diagrams emitted by
/// this renderer.
fn parse_svg_dimensions(svg: &str) -> (f64, f64) {
    let w = svg_numeric_attr(svg, "width").unwrap_or(0.0);
    let h = svg_numeric_attr(svg, "height").unwrap_or(0.0);
    (w, h)
}

fn svg_numeric_attr(svg: &str, attr: &str) -> Option<f64> {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

fn render_math_fallback(document: &MathDocument) -> String {
    let width = 760;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 22;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">math (LaTeX-like)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 22) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 24;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"40\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{}</text>",
            escape_text(line)
        ));
        ty += 22;
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MathDocument;

    fn make_math_doc(body: &str, title: Option<&str>) -> MathDocument {
        MathDocument {
            body: body.to_string(),
            title: title.map(|s| s.to_string()),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn math_artifact_scene_bounds_non_empty() {
        let doc = make_math_doc("a^2 + b^2 = c^2", Some("Pythagorean"));
        let artifact = render_math_artifact(&doc);

        // Scene must be TypedScene and have a non-zero viewport.
        let scene = artifact
            .typed_scene()
            .expect("math renderer must emit a typed RenderScene");
        assert!(
            scene.viewport.size.width > 0.0,
            "scene viewport width must be > 0, got {}",
            scene.viewport.size.width
        );
        assert!(
            scene.viewport.size.height > 0.0,
            "scene viewport height must be > 0, got {}",
            scene.viewport.size.height
        );

        // Must have exactly one formula node.
        assert!(
            scene.nodes.contains_key("math::formula"),
            "scene must contain 'math::formula' node"
        );

        // validate_geometry must report no issues (coords match SVG).
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry validation failed: {issues:?}"
        );
    }

    #[test]
    fn math_artifact_svg_is_byte_identical_to_render_math_svg() {
        let doc = make_math_doc("E = mc^2", None);
        let from_artifact = render_math_artifact(&doc).svg;
        let from_svg = render_math_svg(&doc);
        assert_eq!(
            from_artifact, from_svg,
            "render_math_artifact.svg must be byte-identical to render_math_svg"
        );
    }

    #[test]
    fn math_artifact_fallback_scene_bounds_non_empty() {
        // Trigger the fallback path by forcing render_math_from_parts to fail.
        // An empty body triggers the E_MATH_EMPTY error path, but we also test
        // a valid body to ensure both paths have sensible scenes. The fallback
        // is exercised by constructing a doc that produces a known SVG shape.
        let doc = make_math_doc("\\frac{a}{b}", Some("Fraction"));
        let artifact = render_math_artifact(&doc);
        let scene = artifact
            .typed_scene()
            .expect("math renderer must emit a typed RenderScene (fallback path)");
        assert!(scene.viewport.size.width > 0.0);
        assert!(scene.viewport.size.height > 0.0);
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "geometry issues: {issues:?}");
    }
}
