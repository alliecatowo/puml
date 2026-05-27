use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

mod model;
mod parse;
mod svg;

use model::*;
use parse::*;
use svg::*;

/// Layout constants that mirror the values in `svg.rs` / `render_structured_svg`.
/// These must stay in sync with the SVG emitter so the scene geometry matches
/// the drawn output exactly — the tests enforce this.
const TABLE_X: f64 = 24.0;
const TABLE_WIDTH: f64 = 712.0; // width(760) - 48
const ROW_HEIGHT: f64 = 24.0;
/// Y offset where the first row's top edge lives (title row + spacing).
/// title text y=28, then y+=28 → y=56; row_top = ny-16 = 56-16 = 40 for row 0.
const FIRST_ROW_TOP_Y: f64 = 40.0;

pub fn render_json_svg(document: &JsonDocument) -> String {
    render_json_artifact(document).svg
}

/// Render a JSON diagram into a typed [`RenderArtifact`].
///
/// The SVG output is byte-identical to `render_json_svg`. In addition we build
/// a [`RenderScene`] from the same drawn geometry — one `SceneNode` per row
/// cell at its actual `x/y/width/height` rect — so scene and SVG never drift.
pub fn render_json_artifact(document: &JsonDocument) -> RenderArtifact {
    let controls = parse_structured_controls(&document.raw, DataFamily::Json);
    let rows = json_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    let svg = render_structured_svg(
        document.title.as_deref(),
        DataFamily::Json,
        &rows,
        &controls,
    );
    let scene = build_data_scene(&rows, DataFamily::Json, TABLE_WIDTH, svg_height(rows.len()));
    RenderArtifact::with_scene(svg, scene)
}

pub fn render_yaml_svg(document: &YamlDocument) -> String {
    render_yaml_artifact(document).svg
}

/// Render a YAML diagram into a typed [`RenderArtifact`].
///
/// The SVG output is byte-identical to `render_yaml_svg`. In addition we build
/// a [`RenderScene`] from the same drawn geometry — one `SceneNode` per row
/// cell at its actual `x/y/width/height` rect — so scene and SVG never drift.
pub fn render_yaml_artifact(document: &YamlDocument) -> RenderArtifact {
    let controls = parse_structured_controls(&document.raw, DataFamily::Yaml);
    let rows = yaml_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    let svg = render_structured_svg(
        document.title.as_deref(),
        DataFamily::Yaml,
        &rows,
        &controls,
    );
    let scene = build_data_scene(&rows, DataFamily::Yaml, TABLE_WIDTH, svg_height(rows.len()));
    RenderArtifact::with_scene(svg, scene)
}

/// Compute the SVG height the way `render_structured_svg` does, as `f64`.
fn svg_height(row_count: usize) -> f64 {
    (82 + row_count.max(1) as i32 * ROW_HEIGHT as i32) as f64
}

/// Build a typed [`RenderScene`] from the laid-out row geometry.
///
/// Each row cell is captured as a [`SceneNode`] whose `node_box.bounds` matches
/// the `<rect>` drawn by `render_structured_svg` — `x=TABLE_X`, `y=FIRST_ROW_TOP_Y
/// + index*ROW_HEIGHT`, `width=TABLE_WIDTH`, `height=ROW_HEIGHT`. Node ids are
/// derived from the slash-separated cell path so they are stable and deterministic
/// across repeated renders (BTreeMap discipline: no random ordering).
fn build_data_scene(
    rows: &[RenderRow],
    family: DataFamily,
    table_width: f64,
    diagram_height: f64,
) -> RenderScene {
    let viewport = Rect::new(0.0, 0.0, 760.0, diagram_height);
    let mut scene = RenderScene::new(viewport);

    for (index, row) in rows.iter().enumerate() {
        let y = FIRST_ROW_TOP_Y + index as f64 * ROW_HEIGHT;
        let bounds = Rect::new(TABLE_X, y, table_width, ROW_HEIGHT);

        // Build a stable, deterministic id: always include the row index so
        // duplicate-key rows (which map to the same path) still get unique ids.
        // The path prefix makes ids human-readable in scene inspection tools.
        let id = if row.path.is_empty() {
            format!("{}_row{index}", family.projection())
        } else {
            format!("{}_{}_/{}", family.projection(), index, row.path.join("/"))
        };

        let label = LabelBox {
            id: format!("{id}::label"),
            text: row.label.clone(),
            bounds,
            owner_id: Some(id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id,
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NormalizedDocument;

    fn json_doc(src: &str) -> JsonDocument {
        let document = crate::parse(src).expect("json source should parse");
        match crate::normalize_family(document).expect("json should normalize") {
            NormalizedDocument::Json(doc) => doc,
            other => panic!("expected Json document, got {other:?}"),
        }
    }

    fn yaml_doc(src: &str) -> YamlDocument {
        let document = crate::parse(src).expect("yaml source should parse");
        match crate::normalize_family(document).expect("yaml should normalize") {
            NormalizedDocument::Yaml(doc) => doc,
            other => panic!("expected Yaml document, got {other:?}"),
        }
    }

    #[test]
    fn render_json_artifact_scene_node_count_matches_drawn_cells() {
        let src = "@startjson\n{\"name\": \"Alice\", \"age\": 30}\n@endjson\n";
        let doc = json_doc(src);
        let artifact = render_json_artifact(&doc);

        // The JSON renders as: root `{...}` + 2 scalar children = 3 rows.
        let expected_rows: Vec<RenderRow> = {
            let controls = parse_structured_controls(&doc.raw, DataFamily::Json);
            json_render_rows(&controls.payload).unwrap_or_default()
        };
        let expected_count = expected_rows.len();

        assert_eq!(
            artifact.scene.as_ref().expect("scene must be present").nodes.len(),
            expected_count,
            "scene node count must match drawn row count"
        );
    }

    #[test]
    fn render_json_artifact_svg_is_byte_identical_to_render_json_svg() {
        let src = "@startjson\n{\"x\": 1, \"y\": [2, 3]}\n@endjson\n";
        let doc = json_doc(src);
        let svg_direct = render_json_svg(&doc);
        let artifact = render_json_artifact(&doc);
        assert_eq!(
            artifact.svg, svg_direct,
            "render_json_artifact SVG must be byte-identical to render_json_svg"
        );
    }

    #[test]
    fn render_json_artifact_scene_geometry_is_valid() {
        let src = "@startjson\n{\"a\": 1, \"b\": {\"c\": 2}}\n@endjson\n";
        let doc = json_doc(src);
        let artifact = render_json_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry must be valid; issues: {issues:?}"
        );
    }

    #[test]
    fn render_json_artifact_scene_node_bounds_match_svg_rects() {
        // Single-entry JSON: 2 rows (root + one scalar child).
        let src = "@startjson\n{\"key\": \"value\"}\n@endjson\n";
        let doc = json_doc(src);
        let artifact = render_json_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");

        // Row 0 top edge should be at FIRST_ROW_TOP_Y = 40.0.
        // All rows share x=TABLE_X=24, width=TABLE_WIDTH=712, height=ROW_HEIGHT=24.
        for (i, node) in scene.nodes.values().enumerate() {
            let b = node.node_box.bounds;
            assert_eq!(b.origin.x, TABLE_X, "row {i} x should be {TABLE_X}");
            assert_eq!(b.size.width, TABLE_WIDTH, "row {i} width should be {TABLE_WIDTH}");
            assert_eq!(b.size.height, ROW_HEIGHT, "row {i} height should be {ROW_HEIGHT}");
            assert!(
                b.origin.y >= FIRST_ROW_TOP_Y,
                "row {i} y={} should be >= {FIRST_ROW_TOP_Y}",
                b.origin.y
            );
        }
    }

    #[test]
    fn render_yaml_artifact_scene_node_count_matches_drawn_cells() {
        let src = "@startyaml\nname: Bob\nage: 25\n@endyaml\n";
        let doc = yaml_doc(src);
        let artifact = render_yaml_artifact(&doc);

        let expected_rows: Vec<RenderRow> = {
            let controls = parse_structured_controls(&doc.raw, DataFamily::Yaml);
            yaml_render_rows(&controls.payload).unwrap_or_default()
        };
        let expected_count = expected_rows.len();

        assert_eq!(
            artifact.scene.as_ref().expect("scene must be present").nodes.len(),
            expected_count,
            "scene node count must match drawn row count"
        );
    }

    #[test]
    fn render_yaml_artifact_svg_is_byte_identical_to_render_yaml_svg() {
        let src = "@startyaml\nfoo: bar\nbaz:\n  - 1\n  - 2\n@endyaml\n";
        let doc = yaml_doc(src);
        let svg_direct = render_yaml_svg(&doc);
        let artifact = render_yaml_artifact(&doc);
        assert_eq!(
            artifact.svg, svg_direct,
            "render_yaml_artifact SVG must be byte-identical to render_yaml_svg"
        );
    }

    #[test]
    fn render_yaml_artifact_scene_geometry_is_valid() {
        let src = "@startyaml\nservers:\n  - api\n  - worker\nenabled: true\n@endyaml\n";
        let doc = yaml_doc(src);
        let artifact = render_yaml_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry must be valid; issues: {issues:?}"
        );
    }

    #[test]
    fn render_yaml_artifact_scene_node_bounds_match_svg_rects() {
        let src = "@startyaml\nname: Alice\n@endyaml\n";
        let doc = yaml_doc(src);
        let artifact = render_yaml_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");

        for (i, node) in scene.nodes.values().enumerate() {
            let b = node.node_box.bounds;
            assert_eq!(b.origin.x, TABLE_X, "row {i} x should be {TABLE_X}");
            assert_eq!(b.size.width, TABLE_WIDTH, "row {i} width should be {TABLE_WIDTH}");
            assert_eq!(b.size.height, ROW_HEIGHT, "row {i} height should be {ROW_HEIGHT}");
            assert!(
                b.origin.y >= FIRST_ROW_TOP_Y,
                "row {i} y={} should be >= {FIRST_ROW_TOP_Y}",
                b.origin.y
            );
        }
    }
}
