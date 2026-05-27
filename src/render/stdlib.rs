use crate::model::StdlibDocument;
use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};
use crate::stdlib::StdlibPackStatus;

use super::{creole_text, escape_text};

pub fn render_stdlib_svg(doc: &StdlibDocument) -> String {
    render_stdlib_artifact(doc).svg
}

/// Render a stdlib catalog diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged from the original `render_stdlib_svg` logic.
/// A [`RenderScene`] is built from the *actual* drawn geometry — one node per
/// rendered section box (title header, each pack row, each alias row, each entry
/// row) at the exact pixel coordinates the SVG uses — so the scene stays
/// consistent with the output and never drifts. SVG bytes are byte-identical to
/// the legacy `render_stdlib_svg`; the scene is attached for the typed-geometry
/// validation path.
pub fn render_stdlib_artifact(doc: &StdlibDocument) -> RenderArtifact {
    let width = 920;
    let row_height = 24;
    let pack_rows = doc.packs.len().max(1) as i32;
    let alias_rows = doc.aliases.len().max(1) as i32;
    let entry_preview = doc.entries.iter().take(18).collect::<Vec<_>>();
    let entry_rows = entry_preview.len().max(1) as i32;
    let height = 174 + (pack_rows + alias_rows + entry_rows) * row_height;
    let available_count = doc
        .packs
        .iter()
        .filter(|pack| {
            matches!(
                pack.status,
                StdlibPackStatus::Available | StdlibPackStatus::Builtin
            )
        })
        .count();
    let unavailable_count = doc.packs.len().saturating_sub(available_count);

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" data-stdlib-catalog=\"true\" data-stdlib-entry-count=\"{}\" data-stdlib-pack-count=\"{}\" data-stdlib-unavailable-pack-count=\"{}\">",
        doc.entries.len(),
        available_count,
        unavailable_count
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str("<rect x=\"24\" y=\"24\" width=\"872\" height=\"56\" rx=\"4\" ry=\"4\" fill=\"#f8fafc\" stroke=\"#334155\" stroke-width=\"1.5\"/>");
    out.push_str(&format!(
        "<text x=\"40\" y=\"48\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
        escape_text(doc.title.as_deref().unwrap_or("PlantUML stdlib catalog"))
    ));
    out.push_str(&format!(
        "<text x=\"40\" y=\"68\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{} entries, {} available packs, {} unavailable upstream packs</text>",
        doc.entries.len(),
        available_count,
        unavailable_count
    ));

    let mut y = 112;
    section_header(&mut out, "Packs", y);
    y += 18;
    if doc.packs.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
        y += row_height;
    } else {
        for pack in &doc.packs {
            let status = match pack.status {
                StdlibPackStatus::Available => "available",
                StdlibPackStatus::Builtin => "builtin",
                StdlibPackStatus::Unavailable => "unavailable",
            };
            let color = match pack.status {
                StdlibPackStatus::Available => "#166534",
                StdlibPackStatus::Builtin => "#0369a1",
                StdlibPackStatus::Unavailable => "#991b1b",
            };
            text_row(
                &mut out,
                y,
                &pack.name,
                &format!(
                    "{status}; files {}; alias paths {}",
                    pack.files, pack.aliases
                ),
                color,
            );
            y += row_height;
        }
    }

    y += 12;
    section_header(&mut out, "Aliases", y);
    y += 18;
    if doc.aliases.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
        y += row_height;
    } else {
        for (slug, target) in &doc.aliases {
            text_row(&mut out, y, slug, &format!("-> {target}"), "#1d4ed8");
            y += row_height;
        }
    }

    y += 12;
    section_header(&mut out, "Sample Include Paths", y);
    y += 18;
    if entry_preview.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
    } else {
        for entry in entry_preview {
            let value = if entry.alias {
                format!("-> {}", entry.physical_path)
            } else {
                "direct".to_string()
            };
            text_row(&mut out, y, &entry.path, &value, "#334155");
            y += row_height;
        }
    }

    out.push_str("</svg>");

    let scene = build_stdlib_scene(doc, width as f64, height as f64);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from the stdlib catalog's drawn geometry.
///
/// Each rendered section (title header, pack rows, alias rows, entry rows) is
/// recorded as a [`SceneNode`] at its exact pixel coordinates — the same rect
/// the SVG emits — so scene and SVG are always consistent.
fn build_stdlib_scene(doc: &StdlibDocument, width: f64, height: f64) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    // Title/header box: rect at x=24, y=24, width=872, height=56
    let title_text = doc
        .title
        .as_deref()
        .unwrap_or("PlantUML stdlib catalog")
        .to_string();
    let title_bounds = Rect::new(24.0, 24.0, 872.0, 56.0);
    scene.add_node(SceneNode {
        id: "stdlib::header".to_string(),
        node_box: NodeBox {
            id: "stdlib::header".to_string(),
            bounds: title_bounds,
            ports: Vec::new(),
            labels: vec![LabelBox {
                id: "stdlib::header::label".to_string(),
                text: title_text,
                bounds: title_bounds,
                owner_id: Some("stdlib::header".to_string()),
                role: LabelRole::Node,
            }],
        },
    });

    // Row geometry mirrors text_row: rect at (24, y-16, 872, 24) where y is the
    // text baseline. We track y with the same arithmetic the SVG uses.
    let row_height = 24_i32;
    let mut y = 112_i32;

    // --- Packs section ---
    // section_header advances y by nothing (it draws a label); y += 18 for the heading.
    y += 18;
    let pack_rows: Vec<(String, String)> = if doc.packs.is_empty() {
        vec![("(none)".to_string(), String::new())]
    } else {
        doc.packs
            .iter()
            .map(|pack| {
                let status = match pack.status {
                    StdlibPackStatus::Available => "available",
                    StdlibPackStatus::Builtin => "builtin",
                    StdlibPackStatus::Unavailable => "unavailable",
                };
                (
                    pack.name.clone(),
                    format!(
                        "{status}; files {}; alias paths {}",
                        pack.files, pack.aliases
                    ),
                )
            })
            .collect()
    };
    for (idx, (key, _value)) in pack_rows.iter().enumerate() {
        let row_top = y - 16;
        let bounds = Rect::new(24.0, row_top as f64, 872.0, 24.0);
        let node_id = format!("stdlib::pack::{idx}");
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![LabelBox {
                    id: format!("{node_id}::label"),
                    text: key.clone(),
                    bounds,
                    owner_id: Some(node_id),
                    role: LabelRole::Node,
                }],
            },
        });
        y += row_height;
    }

    // --- Aliases section ---
    y += 12; // gap before section
    y += 18; // section_header y advance
    let alias_rows: Vec<(String, String)> = if doc.aliases.is_empty() {
        vec![("(none)".to_string(), String::new())]
    } else {
        doc.aliases
            .iter()
            .map(|(slug, target)| (slug.clone(), format!("-> {target}")))
            .collect()
    };
    for (idx, (key, _value)) in alias_rows.iter().enumerate() {
        let row_top = y - 16;
        let bounds = Rect::new(24.0, row_top as f64, 872.0, 24.0);
        let node_id = format!("stdlib::alias::{idx}");
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![LabelBox {
                    id: format!("{node_id}::label"),
                    text: key.clone(),
                    bounds,
                    owner_id: Some(node_id),
                    role: LabelRole::Node,
                }],
            },
        });
        y += row_height;
    }

    // --- Sample Include Paths section ---
    let entry_preview: Vec<_> = doc.entries.iter().take(18).collect();
    y += 12; // gap before section
    y += 18; // section_header y advance
    let entry_rows: Vec<String> = if entry_preview.is_empty() {
        vec!["(none)".to_string()]
    } else {
        entry_preview.iter().map(|e| e.path.clone()).collect()
    };
    for (idx, key) in entry_rows.iter().enumerate() {
        let row_top = y - 16;
        let bounds = Rect::new(24.0, row_top as f64, 872.0, 24.0);
        let node_id = format!("stdlib::entry::{idx}");
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![LabelBox {
                    id: format!("{node_id}::label"),
                    text: key.clone(),
                    bounds,
                    owner_id: Some(node_id),
                    role: LabelRole::Node,
                }],
            },
        });
        y += row_height;
    }

    scene
}

fn section_header(out: &mut String, label: &str, y: i32) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
        escape_text(label)
    ));
}

fn text_row(out: &mut String, y: i32, key: &str, value: &str, color: &str) {
    let row_top = y - 16;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{row_top}\" width=\"872\" height=\"24\" fill=\"#ffffff\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>"
    ));
    out.push_str(&creole_text(
        40,
        y,
        "font-family=\"monospace\" font-size=\"12\"",
        key,
        color,
    ));
    out.push_str(&creole_text(
        360,
        y,
        "font-family=\"monospace\" font-size=\"12\"",
        value,
        "#475569",
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_doc() -> StdlibDocument {
        use crate::stdlib::{StdlibEntry, StdlibPackSummary};
        StdlibDocument {
            title: Some("Test stdlib catalog".to_string()),
            root: "/tmp/stdlib".to_string(),
            entries: vec![
                StdlibEntry {
                    path: "awslib/Compute/EC2.puml".to_string(),
                    physical_path: "awslib14/Compute/EC2.puml".to_string(),
                    alias: true,
                },
                StdlibEntry {
                    path: "tupadr3/common.puml".to_string(),
                    physical_path: "tupadr3/common.puml".to_string(),
                    alias: false,
                },
            ],
            packs: vec![StdlibPackSummary {
                name: "awslib".to_string(),
                status: StdlibPackStatus::Available,
                files: 5,
                aliases: 2,
            }],
            aliases: vec![("awslib".to_string(), "awslib14".to_string())],
            missing_packs: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn render_stdlib_artifact_scene_bounds_non_empty() {
        let doc = make_test_doc();
        let artifact = render_stdlib_artifact(&doc);

        let scene = artifact
            .typed_scene()
            .expect("stdlib artifact must have a typed RenderScene");

        // Scene must have at least the header node plus pack/alias/entry rows.
        assert!(
            !scene.nodes.is_empty(),
            "scene must contain at least one node"
        );

        // Viewport must have positive dimensions.
        assert!(
            scene.viewport.size.width > 0.0 && scene.viewport.size.height > 0.0,
            "scene viewport must be non-empty: {:?}",
            scene.viewport
        );
    }

    #[test]
    fn render_stdlib_artifact_geometry_validates_clean() {
        let doc = make_test_doc();
        let artifact = render_stdlib_artifact(&doc);

        let scene = artifact
            .typed_scene()
            .expect("stdlib artifact must have a typed RenderScene");

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry must validate without issues: {issues:?}"
        );
    }

    #[test]
    fn render_stdlib_svg_output_is_byte_identical_to_artifact_svg() {
        // render_stdlib_svg is now a thin wrapper — verify the SVG bytes match.
        let doc = make_test_doc();
        let svg_direct = render_stdlib_svg(&doc);
        let artifact = render_stdlib_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_stdlib_svg must return the same bytes as render_stdlib_artifact().svg"
        );
    }

    #[test]
    fn render_stdlib_artifact_node_ids_are_deterministic() {
        let doc = make_test_doc();
        let artifact1 = render_stdlib_artifact(&doc);
        let artifact2 = render_stdlib_artifact(&doc);

        let ids1: Vec<_> = artifact1
            .typed_scene()
            .unwrap()
            .nodes
            .keys()
            .cloned()
            .collect();
        let ids2: Vec<_> = artifact2
            .typed_scene()
            .unwrap()
            .nodes
            .keys()
            .cloned()
            .collect();
        assert_eq!(ids1, ids2, "node IDs must be deterministic across runs");
    }

    #[test]
    fn render_stdlib_artifact_empty_packs_and_aliases_still_valid() {
        let doc = StdlibDocument {
            title: None,
            root: "/tmp/stdlib".to_string(),
            entries: vec![],
            packs: vec![],
            aliases: vec![],
            missing_packs: vec![],
            warnings: vec![],
        };
        let artifact = render_stdlib_artifact(&doc);
        let scene = artifact
            .typed_scene()
            .expect("empty stdlib must still produce a typed scene");

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "empty stdlib scene must validate cleanly: {issues:?}"
        );

        // Even with empty collections the header node must exist.
        assert!(
            scene.nodes.contains_key("stdlib::header"),
            "header node must always be present"
        );
    }
}
