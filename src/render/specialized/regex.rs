use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{
    text_metrics::estimate_text_width_default, LabelBox, LabelRole, NodeBox, Rect, RenderScene,
    SceneNode,
};

pub fn render_regex_svg(document: &RegexDocument) -> String {
    render_regex_artifact(document).svg
}

/// Render a regex railroad diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged (byte-identical to the legacy `render_regex_svg`).
/// A [`RenderScene`] is built from the same laid-out geometry the SVG draws —
/// each token box at its actual rect — so the scene stays in sync with the
/// output and never diverges. Reference pattern for the typed-scene migration
/// (#1258, P2).
pub fn render_regex_artifact(document: &RegexDocument) -> RenderArtifact {
    // Auto-expand canvas width to prevent title / token row truncation (#514).
    let min_width = 760_i32;
    // Include the document title width in the canvas width calculation.
    let title_px = document
        .title
        .as_deref()
        .map(|t| (t.len() as i32) * 9 + 64)
        .unwrap_or(0);
    let max_row_px: i32 = document
        .patterns
        .iter()
        .map(|pat| {
            // Source regex string row width (shown as /source/). Regex
            // source is ASCII-only so byte-len == char count here.
            let source_px = estimate_text_width_default(&pat.source) + 64;
            // Token row: sum of token label widths
            let tokens_px: i32 = regex_tokens_to_labels(&pat.tokens)
                .iter()
                .map(|l| (l.len().max(1) as i32) * 8 + 18 + 8)
                .sum::<i32>()
                + 80;
            source_px.max(tokens_px)
        })
        .max()
        .unwrap_or(0);
    let width = min_width.max(title_px).max(max_row_px);
    let row_height = 80;
    let height = 80 + (document.patterns.len().max(1) as i32) * row_height;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">Railroad diagram (regex)</text>"
    ));
    y += 18;

    // Collect (pattern_index, token_index, x, y_baseline, box_w, label) for scene
    // construction — mirrors the exact coordinates emitted by render_regex_row.
    let mut scene_boxes: Vec<(String, i32, i32, i32, i32)> = Vec::new();

    if document.patterns.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for (pat_idx, pat) in document.patterns.iter().enumerate() {
            render_regex_row(
                &mut out,
                &pat.source,
                &pat.tokens,
                y,
                width,
                min_width,
                pat_idx,
                &mut scene_boxes,
            );
            y += row_height;
        }
    }
    out.push_str("</svg>");

    let scene = build_regex_scene(width, height, &scene_boxes);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from the token-box geometry collected during SVG
/// emission. Each entry in `scene_boxes` is `(id, x, baseline_y, box_w, 22)` —
/// the same values used to emit the `<rect>` elements, so scene and SVG are
/// always consistent.
fn build_regex_scene(
    width: i32,
    height: i32,
    scene_boxes: &[(String, i32, i32, i32, i32)],
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));

    for (id, bx, by, bw, bh) in scene_boxes {
        let bounds = Rect::new(*bx as f64, *by as f64, *bw as f64, *bh as f64);
        let label = LabelBox {
            id: format!("{id}::label"),
            text: id.clone(),
            bounds,
            owner_id: Some(id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id: id.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    scene
}

#[allow(clippy::too_many_arguments)]
fn render_regex_row(
    out: &mut String,
    source: &str,
    tokens: &[RegexToken],
    y: i32,
    width: i32,
    _min_width: i32,
    pat_idx: usize,
    scene_boxes: &mut Vec<(String, i32, i32, i32, i32)>,
) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">/{}/</text>",
        y - 4,
        escape_text(source)
    ));
    let baseline = y + 26;
    out.push_str(&format!(
        "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
        by = baseline,
        x2 = width - 48
    ));
    let mut x = 40;
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = x,
        by = baseline
    ));
    x += 18;
    let labels = regex_tokens_to_labels(tokens);
    for (tok_idx, label) in labels.iter().enumerate() {
        let box_w = (label.len().max(1) as i32) * 8 + 18;
        let box_w = box_w.min(width - x - 60);
        let (class_name, fill, stroke) = regex_label_style(label);
        out.push_str(&format!(
            "<rect class=\"regex-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            x = x,
            ry = baseline - 11,
            w = box_w
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#0c4a6e\">{}</text>",
            escape_text(label),
            tx = x + 6,
            ty = baseline + 4
        ));

        // Record this token box for scene construction using the exact same
        // geometry: x, baseline-11 is the top edge, box_w × 22 is width × height.
        let box_id = format!("pat{pat_idx}::tok{tok_idx}");
        scene_boxes.push((box_id, x, baseline - 11, box_w, 22));

        x += box_w + 8;
        // With auto-expanded canvas, only break at absolute canvas boundary.
        if x > width - 48 {
            break;
        }
    }
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = (width - 36),
        by = baseline
    ));
}

fn regex_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.contains("alt(") {
        ("regex-alt", "#fef3c7", "#d97706")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("regex-repeat", "#dcfce7", "#16a34a")
    } else if label.starts_with('[') {
        ("regex-class", "#ede9fe", "#7c3aed")
    } else if label == "^" || label == "$" {
        ("regex-anchor", "#fee2e2", "#dc2626")
    } else {
        ("regex-literal", "#e0f2fe", "#0284c7")
    }
}

fn regex_tokens_to_labels(tokens: &[RegexToken]) -> Vec<String> {
    let mut out = Vec::new();
    for t in tokens {
        out.push(regex_token_label(t));
    }
    out
}

fn regex_token_label(token: &RegexToken) -> String {
    match token {
        RegexToken::Literal(s) => format!("'{}'", s),
        RegexToken::CharClass(s) => format!("[{}]", s),
        RegexToken::Group(inner) => format!("({})", regex_tokens_to_labels(inner).join(" ")),
        RegexToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| regex_tokens_to_labels(b).join(" "))
                .collect();
            format!("alt({})", parts.join("|"))
        }
        RegexToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
                RepeatKind::Exact(n) => return format!("{}{{{}}}", regex_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        regex_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
            };
            format!("{}{}", regex_token_label(inner), suffix)
        }
        RegexToken::Escape(c) => format!("\\{}", c),
        RegexToken::AnyChar => ".".to_string(),
        RegexToken::Anchor(s) => s.clone(),
        RegexToken::Unsupported(s) => format!("?{}?", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RegexDocument, RegexPattern, RegexToken};

    fn make_simple_doc() -> RegexDocument {
        // Represents: /a(b|c)*d?/ with tokens: 'a', (alt(b|c))*, d?
        RegexDocument {
            title: Some("Test".to_string()),
            patterns: vec![RegexPattern {
                source: "a(b|c)*d?".to_string(),
                tokens: vec![
                    RegexToken::Literal("a".to_string()),
                    RegexToken::Repeat {
                        inner: Box::new(RegexToken::Alt(vec![
                            vec![RegexToken::Literal("b".to_string())],
                            vec![RegexToken::Literal("c".to_string())],
                        ])),
                        kind: RepeatKind::ZeroOrMore,
                    },
                    RegexToken::Repeat {
                        inner: Box::new(RegexToken::Literal("d".to_string())),
                        kind: RepeatKind::ZeroOrOne,
                    },
                ],
            }],
            warnings: Vec::new(),
        }
    }

    #[test]
    fn render_regex_artifact_produces_typed_scene() {
        let doc = make_simple_doc();
        let artifact = render_regex_artifact(&doc);

        // SVG must be present and valid
        assert!(artifact.svg.contains("<svg"), "artifact should contain SVG");

        // Scene must be typed (TypedScene availability)
        let scene = artifact
            .typed_scene()
            .expect("regex artifact must have a typed RenderScene");

        // 3 tokens → 3 scene nodes (one per token box)
        assert_eq!(
            scene.nodes.len(),
            3,
            "expected 3 scene nodes for 3 tokens, got {}",
            scene.nodes.len()
        );

        // Geometry must be valid — no issues allowed
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry validation failed: {:?}",
            issues
        );
    }

    #[test]
    fn render_regex_svg_is_byte_identical_to_artifact_svg() {
        let doc = make_simple_doc();
        let svg_direct = render_regex_svg(&doc);
        let artifact = render_regex_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_regex_svg must be byte-identical to render_regex_artifact().svg"
        );
    }

    #[test]
    fn render_regex_artifact_empty_document_no_panic() {
        let doc = RegexDocument {
            title: None,
            patterns: Vec::new(),
            warnings: Vec::new(),
        };
        let artifact = render_regex_artifact(&doc);
        assert!(artifact.svg.contains("<svg"));
        // Empty doc has no token boxes → no scene nodes
        let scene = artifact.typed_scene().expect("must have typed scene");
        assert_eq!(scene.nodes.len(), 0);
        assert!(scene.validate_geometry().is_empty());
    }
}
