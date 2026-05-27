use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{LabelBox, LabelRole, NodeBox, Rect, RenderScene, SceneNode};

mod chronology;
mod dates;
mod details;
mod gantt;
mod gantt_scene;
mod rows;
mod scale;
mod util;

use chronology::*;
use dates::*;
use details::*;
use gantt::*;
use rows::*;
use scale::*;
use util::*;

pub fn render_timeline_stub_svg(document: &TimelineDocument) -> String {
    render_timeline_svg(document)
}

/// Render Gantt/Chronology timelines as real SVGs:
///   - Gantt: horizontal task bars on a date axis, milestone diamonds,
///     dashed arrows for `requires`/start/etc. constraints between bars.
///   - Chronology: vertical timeline with event bullets along a date axis.
pub fn render_timeline_svg(document: &TimelineDocument) -> String {
    render_timeline_artifact(document).svg
}

/// Render Gantt/Chronology timelines into a typed [`RenderArtifact`].
///
/// The SVG is produced identically to `render_timeline_svg`; additionally a
/// [`RenderScene`] is built from the *exact* laid-out geometry the SVG uses
/// (task-bar rects, milestone diamond centres, chronology event cards) so that
/// scene and SVG never diverge. SVG output is byte-identical to the legacy
/// `render_timeline_svg`; the scene is attached for the typed-geometry
/// validation path (#1258).
pub fn render_timeline_artifact(document: &TimelineDocument) -> RenderArtifact {
    match document.kind {
        DiagramKind::Chronology => render_chronology_artifact(document),
        _ => render_gantt_artifact(document),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_timeline(src: &str) -> TimelineDocument {
        let ast = crate::parser::parse(src).expect("parse failed");
        match crate::normalize::normalize_family(ast).expect("normalize failed") {
            crate::model::NormalizedDocument::Timeline(doc) => doc,
            other => panic!("expected Timeline, got {:?}", other),
        }
    }

    #[test]
    fn gantt_artifact_has_typed_scene_with_nodes() {
        let src = r#"@startgantt
Project starts 2026-01-05
[Design] lasts 5 days
[Implementation] lasts 10 days
[Testing] lasts 3 days
@endgantt
"#;
        let doc = parse_timeline(src);
        let artifact = render_timeline_artifact(&doc);
        assert!(
            artifact.scene.is_some(),
            "gantt artifact should carry a typed RenderScene"
        );
        let scene = artifact.scene.unwrap();
        assert!(
            !scene.nodes.is_empty(),
            "gantt scene should have at least one node (task bar)"
        );
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "gantt scene has geometry issues: {issues:?}"
        );
    }

    #[test]
    fn chronology_artifact_has_typed_scene_with_nodes() {
        let src = r#"@startchronology
2020: First event
2021: Second event
2022: Third event
@endchronology
"#;
        let doc = parse_timeline(src);
        let artifact = render_timeline_artifact(&doc);
        assert!(
            artifact.scene.is_some(),
            "chronology artifact should carry a typed RenderScene"
        );
        let scene = artifact.scene.unwrap();
        assert!(
            !scene.nodes.is_empty(),
            "chronology scene should have at least one node (event card)"
        );
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "chronology scene has geometry issues: {issues:?}"
        );
    }
}
