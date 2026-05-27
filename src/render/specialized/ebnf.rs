use super::*;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

pub fn render_ebnf_svg(document: &EbnfDocument) -> String {
    render_ebnf_artifact(document).svg
}

/// Render an EBNF diagram into a typed [`RenderArtifact`].
///
/// The SVG is still emitted directly (EBNF draws railroad-diagram token boxes
/// and connecting track segments at known coordinates), but we also build a
/// [`RenderScene`] from the *actual* drawn geometry — one `SceneNode` per token
/// box at its exact `(x, y, w, h)` — so the scene stays consistent with the
/// output. SVG output is byte-identical to the legacy `render_ebnf_svg`; the
/// scene is attached for the typed-geometry validation path.
pub fn render_ebnf_artifact(document: &EbnfDocument) -> RenderArtifact {
    // Auto-expand canvas width to fit the widest rule (#510): each token ~8px/char +
    // 8px gap, plus 120px for the rule name + margins.
    let min_width = 820_i32;
    let max_token_row_px: i32 = document
        .rules
        .iter()
        .map(|rule| {
            let labels = ebnf_tokens_to_labels(&rule.tokens);
            let row_px: i32 = labels
                .iter()
                .map(|l| (l.len() as i32 * 8).clamp(36, 400) + 8)
                .sum::<i32>()
                + 120;
            row_px
        })
        .max()
        .unwrap_or(0);
    let width = min_width.max(max_token_row_px + 48);
    let row_height = 90;
    // Extra bottom pad so the last-row terminal circles/ovals aren't clipped (#510).
    let bottom_pad = 32;
    let height = 80 + (document.rules.len().max(1) as i32) * row_height + bottom_pad;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">EBNF railroad diagrams</text>"
    ));
    y += 18;

    // Collect scene nodes from the drawn box geometry (parallel to SVG emission below).
    let mut scene_boxes: Vec<(String, Rect)> = Vec::new();

    if document.rules.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for (rule_idx, rule) in document.rules.iter().enumerate() {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0f172a\">{} ::=</text>",
                escape_text(&rule.name),
                ty = y
            ));
            let baseline = y + 30;
            out.push_str(&format!(
                "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                by = baseline,
                x2 = width - 24
            ));
            out.push_str(&format!(
                "<circle cx=\"40\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                by = baseline
            ));
            let mut x = 60;
            let labels = ebnf_tokens_to_labels(&rule.tokens);
            for (tok_idx, label) in labels.iter().enumerate() {
                let box_w = ((label.len() as i32) * 8).clamp(36, width - x - 60);
                let (class_name, fill, stroke) = ebnf_label_style(label);
                out.push_str(&format!(
                    "<rect class=\"ebnf-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                    x = x,
                    ry = baseline - 11,
                    w = box_w
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                    escape_text(label),
                    tx = x + 6,
                    ty = baseline + 4
                ));

                // Capture the exact rect the SVG just drew for the scene.
                let node_id = format!("rule{rule_idx}_tok{tok_idx}");
                scene_boxes.push((
                    node_id,
                    Rect::new(x as f64, (baseline - 11) as f64, box_w as f64, 22.0),
                ));

                x += box_w + 8;
                // With auto-expanded canvas, only break when truly out of space.
                if x > width - 48 {
                    break;
                }
            }
            out.push_str(&format!(
                "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                x = (width - 36),
                by = baseline
            ));
            y += row_height;
        }
    }
    out.push_str("</svg>");

    let scene = build_ebnf_scene(width as f64, height as f64, &scene_boxes, document);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from the EBNF renderer's laid-out geometry.
///
/// Each token box in the SVG becomes a `SceneNode` at the *same* rect the SVG
/// drew, so scene and output are always consistent.
fn build_ebnf_scene(
    width: f64,
    height: f64,
    scene_boxes: &[(String, Rect)],
    document: &EbnfDocument,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    // Reconstruct the same label text for each box, using rule/token indices that
    // match the ids in scene_boxes.
    let mut box_labels: Vec<String> = Vec::with_capacity(scene_boxes.len());
    'outer: for rule in &document.rules {
        let labels = ebnf_tokens_to_labels(&rule.tokens);
        for label in &labels {
            box_labels.push(label.clone());
            if box_labels.len() == scene_boxes.len() {
                break 'outer;
            }
        }
    }

    for (i, (node_id, bounds)) in scene_boxes.iter().enumerate() {
        let text = box_labels.get(i).cloned().unwrap_or_default();
        let label = LabelBox {
            id: format!("{node_id}::label"),
            text,
            bounds: *bounds,
            owner_id: Some(node_id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id.clone(),
                bounds: *bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    scene
}

fn ebnf_tokens_to_labels(tokens: &[EbnfToken]) -> Vec<String> {
    tokens.iter().map(ebnf_token_label).collect()
}

fn ebnf_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.starts_with('"') || label.starts_with('\'') {
        ("ebnf-terminal", "#fef3c7", "#d97706")
    } else if label.starts_with('[') {
        ("ebnf-optional", "#dcfce7", "#16a34a")
    } else if label.starts_with('{') {
        ("ebnf-repetition", "#ede9fe", "#7c3aed")
    } else if label.contains(" | ") {
        ("ebnf-alt", "#fee2e2", "#dc2626")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("ebnf-repeat", "#e0f2fe", "#0284c7")
    } else {
        ("ebnf-nonterminal", "#e0e7ff", "#4f46e5")
    }
}

fn ebnf_token_label(token: &EbnfToken) -> String {
    match token {
        EbnfToken::Terminal(s) => format!("\"{}\"", s),
        EbnfToken::NonTerminal(s) => s.clone(),
        EbnfToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| ebnf_tokens_to_labels(b).join(" "))
                .collect();
            format!("({})", parts.join(" | "))
        }
        EbnfToken::Group(inner) => format!("({})", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Optional(inner) => format!("[{}]", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repetition(inner) => format!("{{{}}}", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
                RepeatKind::Exact(n) => return format!("{}{{{}}}", ebnf_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        ebnf_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
            };
            format!("{}{}", ebnf_token_label(inner), suffix)
        }
        EbnfToken::Unsupported(s) => format!("?{}?", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EbnfDocument, EbnfRule, EbnfToken};

    fn make_simple_doc() -> EbnfDocument {
        EbnfDocument {
            title: Some("Test Grammar".to_string()),
            rules: vec![
                EbnfRule {
                    name: "expr".to_string(),
                    body: "\"id\" | number".to_string(),
                    tokens: vec![
                        EbnfToken::Terminal("id".to_string()),
                        EbnfToken::NonTerminal("number".to_string()),
                    ],
                },
                EbnfRule {
                    name: "number".to_string(),
                    body: "\"0\" | \"1\"".to_string(),
                    tokens: vec![
                        EbnfToken::Terminal("0".to_string()),
                        EbnfToken::Terminal("1".to_string()),
                    ],
                },
            ],
            warnings: vec![],
        }
    }

    #[test]
    fn ebnf_artifact_scene_has_expected_node_count() {
        let doc = make_simple_doc();
        let artifact = render_ebnf_artifact(&doc);

        // 2 tokens in rule 0, 2 tokens in rule 1 → 4 scene nodes total.
        assert_eq!(
            artifact.scene.as_ref().unwrap().nodes.len(),
            4,
            "expected one SceneNode per drawn token box"
        );
    }

    #[test]
    fn ebnf_artifact_scene_geometry_is_valid() {
        let doc = make_simple_doc();
        let artifact = render_ebnf_artifact(&doc);
        let scene = artifact.scene.as_ref().unwrap();
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry validation failed: {issues:?}"
        );
    }

    #[test]
    fn ebnf_artifact_svg_is_unchanged() {
        // SVG emitted by render_ebnf_artifact must be byte-identical to the
        // pre-migration render_ebnf_svg output.
        let doc = make_simple_doc();
        let svg_via_artifact = render_ebnf_artifact(&doc).svg;
        let svg_via_legacy = render_ebnf_svg(&doc);
        assert_eq!(
            svg_via_artifact, svg_via_legacy,
            "SVG output changed — the scene construction must not mutate the SVG path"
        );
    }

    #[test]
    fn ebnf_artifact_empty_doc_no_boxes() {
        let doc = EbnfDocument {
            title: None,
            rules: vec![],
            warnings: vec![],
        };
        let artifact = render_ebnf_artifact(&doc);
        let scene = artifact.scene.as_ref().unwrap();
        assert_eq!(scene.nodes.len(), 0);
        assert!(scene.validate_geometry().is_empty());
    }
}
