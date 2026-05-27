use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

const BOARD_COLUMN_W: i32 = 220;
const BOARD_GAP: i32 = 18;
const BOARD_MARGIN: i32 = 24;
const BOARD_HEADER_H: i32 = 34;
const BOARD_CARD_H: i32 = 46;

pub fn render_board_svg(document: &BoardDocument) -> String {
    render_board_artifact(document).svg
}

/// Render a board (kanban) diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged. A [`RenderScene`] is built from the same drawn
/// geometry — column boxes at their laid-out positions and card boxes at the same
/// positions the SVG draws them — so scene and SVG remain consistent. Output is
/// byte-identical to the legacy `render_board_svg`; the scene is attached for the
/// typed-geometry validation path.
pub fn render_board_artifact(document: &BoardDocument) -> RenderArtifact {
    let columns = document.columns.len().max(1) as i32;
    let max_cards = document
        .columns
        .iter()
        .map(|column| column.cards.len())
        .max()
        .unwrap_or(0) as i32;
    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);
    let width = BOARD_MARGIN * 2 + columns * BOARD_COLUMN_W + (columns - 1) * BOARD_GAP;
    let height =
        BOARD_MARGIN * 2 + title_h + BOARD_HEADER_H + 16 + max_cards.max(1) * (BOARD_CARD_H + 10);
    let mut out = svg_root(width, height);
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>");

    let mut y = BOARD_MARGIN;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text class=\"board-title\" x=\"{BOARD_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += title_h;
    }

    for (idx, column) in document.columns.iter().enumerate() {
        let x = BOARD_MARGIN + idx as i32 * (BOARD_COLUMN_W + BOARD_GAP);
        let col_h = height - y - BOARD_MARGIN;
        out.push_str(&format!(
            "<g class=\"board-column\" data-board-column=\"{}\">",
            escape_text(&column.title)
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{BOARD_COLUMN_W}\" height=\"{col_h}\" rx=\"8\" ry=\"8\" fill=\"#e2e8f0\" stroke=\"#94a3b8\"/>"
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{BOARD_COLUMN_W}\" height=\"{BOARD_HEADER_H}\" rx=\"8\" ry=\"8\" fill=\"#334155\" stroke=\"#334155\"/>"
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"700\" fill=\"white\">{}</text>",
            x + 12,
            y + 22,
            escape_text(&column.title)
        ));
        let mut card_y = y + BOARD_HEADER_H + 12;
        for card in &column.cards {
            render_board_card(&mut out, x + 10, card_y, card);
            card_y += BOARD_CARD_H + 10;
        }
        out.push_str("</g>");
    }

    out.push_str("</svg>");

    let scene = build_board_scene(document, width, height);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from board's laid-out geometry. Column boxes and
/// card boxes use the same positions/sizes the SVG draws, so scene and SVG never
/// diverge. No edges are present (board is a pure containment layout).
fn build_board_scene(document: &BoardDocument, width: i32, height: i32) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));

    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);

    // col_y is the y of the column header start (same as the SVG's `y` after title).
    // We replicate the same y computation as the SVG loop.
    let col_y = BOARD_MARGIN + title_h;
    let col_h = height - col_y - BOARD_MARGIN;

    for (idx, column) in document.columns.iter().enumerate() {
        let x = BOARD_MARGIN + idx as i32 * (BOARD_COLUMN_W + BOARD_GAP);
        let col_id = format!("col{idx}");
        let col_bounds = Rect::new(x as f64, col_y as f64, BOARD_COLUMN_W as f64, col_h as f64);
        let col_label = LabelBox {
            id: format!("{col_id}::label"),
            text: column.title.clone(),
            bounds: col_bounds,
            owner_id: Some(col_id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: col_id.clone(),
            node_box: NodeBox {
                id: col_id,
                bounds: col_bounds,
                ports: Vec::new(),
                labels: vec![col_label],
            },
        });

        let mut card_y = col_y + BOARD_HEADER_H + 12;
        for (cidx, card) in column.cards.iter().enumerate() {
            let indent = ((card.depth.saturating_sub(1) as i32) * 12).min(48);
            let card_x = x + 10 + indent;
            let card_w = BOARD_COLUMN_W - 20 - indent;
            let card_id = format!("col{idx}card{cidx}");
            let card_bounds = Rect::new(
                card_x as f64,
                card_y as f64,
                card_w as f64,
                BOARD_CARD_H as f64,
            );
            let card_label = LabelBox {
                id: format!("{card_id}::label"),
                text: card.title.clone(),
                bounds: card_bounds,
                owner_id: Some(card_id.clone()),
                role: LabelRole::Node,
            };
            scene.add_node(SceneNode {
                id: card_id.clone(),
                node_box: NodeBox {
                    id: card_id,
                    bounds: card_bounds,
                    ports: Vec::new(),
                    labels: vec![card_label],
                },
            });
            card_y += BOARD_CARD_H + 10;
        }
    }

    scene
}

fn render_board_card(out: &mut String, x: i32, y: i32, card: &BoardCard) {
    let indent = ((card.depth.saturating_sub(1) as i32) * 12).min(48);
    let card_x = x + indent;
    let card_w = BOARD_COLUMN_W - 20 - indent;
    let fill = match card.depth {
        1 => "#ffffff",
        2 => "#fefce8",
        3 => "#ecfdf5",
        _ => "#eef2ff",
    };
    out.push_str(&format!(
        "<g class=\"board-card\" data-board-depth=\"{}\">",
        card.depth
    ));
    out.push_str(&format!(
        "<rect x=\"{card_x}\" y=\"{y}\" width=\"{card_w}\" height=\"{BOARD_CARD_H}\" rx=\"6\" ry=\"6\" fill=\"{fill}\" stroke=\"#cbd5e1\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
        card_x + 10,
        y + 20,
        escape_text(&truncate_label(&card.title, 24))
    ));
    if !card.tags.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">#{}</text>",
            card_x + 10,
            y + 36,
            escape_text(&card.tags.join(" #"))
        ));
    }
    out.push_str("</g>");
}

pub fn render_files_svg(document: &FilesDocument) -> String {
    render_files_artifact(document).svg
}

/// Render a files (file-tree) diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged. A [`RenderScene`] is built from the same drawn
/// geometry — one scene node per file-tree entry at its actual drawn x/y position —
/// so scene and SVG remain consistent. Output is byte-identical to the legacy
/// `render_files_svg`; the scene is attached for the typed-geometry validation path.
pub fn render_files_artifact(document: &FilesDocument) -> RenderArtifact {
    let row_count = count_file_rows(&document.roots) + document.top_notes.len();
    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);
    let width = 760;
    let height = 56 + title_h + (row_count.max(1) as i32 * 28);
    let mut out = svg_root(width, height);
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text class=\"files-title\" x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += 30;
    }
    for note in &document.top_notes {
        render_file_note(&mut out, 24, y - 14, note);
        y += 28;
    }
    for node in &document.roots {
        render_file_node(&mut out, node, 0, &mut y);
    }
    out.push_str("</svg>");

    let scene = build_files_scene(document, width, height);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from files' laid-out geometry. Each file-tree
/// entry becomes a scene node at its actual drawn position (same x/y the SVG uses),
/// so scene and SVG never diverge. No edges are present (files is a pure tree layout).
fn build_files_scene(document: &FilesDocument, width: i32, height: i32) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));
    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);

    // Replicate the same y tracking used by render_files_artifact.
    let mut y = 28 + title_h;
    // top_notes each consume one row
    for _ in &document.top_notes {
        y += 28;
    }

    let mut node_counter = 0usize;
    for node in &document.roots {
        collect_file_scene_nodes(&mut scene, node, 0, &mut y, &mut node_counter);
    }

    scene
}

/// Recursively collect scene nodes for file tree entries, replicating the exact
/// y-advancement logic from `render_file_node` so scene coords match SVG coords.
fn collect_file_scene_nodes(
    scene: &mut RenderScene,
    node: &FileTreeNode,
    depth: usize,
    y: &mut i32,
    counter: &mut usize,
) {
    let x = 24 + depth as i32 * 24;
    // The SVG text is at `y` (after icon rendering at `y - 14`). The node row
    // spans from `y - 14` to `y - 14 + 20` ≈ `y + 6`, but we model the full
    // 28-px row height to match the y stride used by the SVG.
    let node_x = x as f64;
    let node_y = (*y - 14) as f64;
    // Width: icon (26px) + gap (8px) + label area fills remaining width up to 760.
    let node_w = (760 - x) as f64;
    let node_h = 20.0; // icon height as drawn by the SVG path

    let node_id = format!("file{counter}");
    *counter += 1;

    let bounds = Rect::new(node_x, node_y, node_w, node_h);
    let label = LabelBox {
        id: format!("{node_id}::label"),
        text: node.name.clone(),
        bounds,
        owner_id: Some(node_id.clone()),
        role: LabelRole::Node,
    };
    scene.add_node(SceneNode {
        id: node_id.clone(),
        node_box: NodeBox {
            id: node_id,
            bounds,
            ports: Vec::new(),
            labels: vec![label],
        },
    });

    // Advance y exactly as render_file_node does.
    *y += 28;
    // notes advance y too
    for _ in &node.notes {
        *y += 28;
    }
    for child in &node.children {
        collect_file_scene_nodes(scene, child, depth + 1, y, counter);
    }
}

fn render_file_node(out: &mut String, node: &FileTreeNode, depth: usize, y: &mut i32) {
    let x = 24 + depth as i32 * 24;
    let icon = if node.is_dir { "dir" } else { "file" };
    out.push_str(&format!(
        "<g class=\"files-entry\" data-files-path=\"{}\" data-files-kind=\"{icon}\">",
        escape_text(&node.path)
    ));
    if node.is_dir {
        out.push_str(&format!(
            "<path d=\"M{x} {} h9 l3 4 h14 v16 h-26 z\" fill=\"#dbeafe\" stroke=\"#2563eb\"/>",
            *y - 14
        ));
    } else {
        out.push_str(&format!(
            "<path d=\"M{x} {} h17 l7 7 v13 h-24 z\" fill=\"#f8fafc\" stroke=\"#64748b\"/>",
            *y - 14
        ));
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{y}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{}</text>",
        x + 34,
        escape_text(&node.name)
    ));
    out.push_str("</g>");
    *y += 28;
    for note in &node.notes {
        render_file_note(out, x + 34, *y - 16, note);
        *y += 28;
    }
    for child in &node.children {
        render_file_node(out, child, depth + 1, y);
    }
}

fn render_file_note(out: &mut String, x: i32, y: i32, note: &str) {
    let text = note.replace('\n', " / ");
    out.push_str(&format!(
        "<g class=\"files-note\"><rect x=\"{x}\" y=\"{y}\" width=\"360\" height=\"22\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#fdba74\"/><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#7c2d12\">{}</text></g>",
        x + 8,
        y + 15,
        escape_text(&truncate_label(&text, 46))
    ));
}

fn count_file_rows(nodes: &[FileTreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node.notes.len() + count_file_rows(&node.children))
        .sum()
}

fn svg_root(width: i32, height: i32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    )
}

fn truncate_label(label: &str, max_chars: usize) -> String {
    let mut chars = label.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BoardColumn, BoardDocument};

    fn make_board_doc() -> BoardDocument {
        BoardDocument {
            title: Some("Sprint Board".to_string()),
            columns: vec![
                BoardColumn {
                    title: "Todo".to_string(),
                    cards: vec![
                        BoardCard {
                            depth: 1,
                            title: "Fix bug".to_string(),
                            tags: vec!["bug".to_string()],
                        },
                        BoardCard {
                            depth: 2,
                            title: "Sub-task".to_string(),
                            tags: vec![],
                        },
                    ],
                },
                BoardColumn {
                    title: "Done".to_string(),
                    cards: vec![BoardCard {
                        depth: 1,
                        title: "Deploy".to_string(),
                        tags: vec![],
                    }],
                },
            ],
            warnings: vec![],
        }
    }

    fn make_files_doc() -> FilesDocument {
        FilesDocument {
            title: Some("Project".to_string()),
            roots: vec![
                FileTreeNode {
                    name: "src".to_string(),
                    path: "src".to_string(),
                    is_dir: true,
                    notes: vec![],
                    children: vec![FileTreeNode {
                        name: "main.rs".to_string(),
                        path: "src/main.rs".to_string(),
                        is_dir: false,
                        notes: vec![],
                        children: vec![],
                    }],
                },
                FileTreeNode {
                    name: "Cargo.toml".to_string(),
                    path: "Cargo.toml".to_string(),
                    is_dir: false,
                    notes: vec![],
                    children: vec![],
                },
            ],
            top_notes: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn board_artifact_svg_matches_svg_only() {
        let doc = make_board_doc();
        let svg_only = render_board_svg(&doc);
        let artifact = render_board_artifact(&doc);
        // SVG output must be byte-identical
        assert_eq!(artifact.svg, svg_only);
    }

    #[test]
    fn board_artifact_scene_node_count() {
        let doc = make_board_doc();
        let artifact = render_board_artifact(&doc);
        let scene = artifact.scene.expect("scene should be present");

        // 2 columns + 3 cards (2 in col0, 1 in col1)
        assert_eq!(
            scene.nodes.len(),
            5,
            "expected 2 columns + 3 cards = 5 nodes"
        );
    }

    #[test]
    fn board_artifact_scene_geometry_valid() {
        let doc = make_board_doc();
        let artifact = render_board_artifact(&doc);
        let scene = artifact.scene.expect("scene should be present");
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "scene geometry issues: {:?}", issues);
    }

    #[test]
    fn files_artifact_svg_matches_svg_only() {
        let doc = make_files_doc();
        let svg_only = render_files_svg(&doc);
        let artifact = render_files_artifact(&doc);
        // SVG output must be byte-identical
        assert_eq!(artifact.svg, svg_only);
    }

    #[test]
    fn files_artifact_scene_node_count() {
        let doc = make_files_doc();
        let artifact = render_files_artifact(&doc);
        let scene = artifact.scene.expect("scene should be present");

        // 3 nodes: src/ (dir), main.rs, Cargo.toml
        assert_eq!(scene.nodes.len(), 3, "expected 3 file nodes");
    }

    #[test]
    fn files_artifact_scene_geometry_valid() {
        let doc = make_files_doc();
        let artifact = render_files_artifact(&doc);
        let scene = artifact.scene.expect("scene should be present");
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "scene geometry issues: {:?}", issues);
    }
}
