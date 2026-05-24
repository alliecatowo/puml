use super::super::svg::escape_text;
use crate::scene::{LifecycleMarkerKind, Scene};

pub(super) fn render_lifecycle_markers(out: &mut String, scene: &Scene) {
    for marker in &scene.lifecycle_markers {
        match marker.kind {
            LifecycleMarkerKind::Create => {
                out.push_str(&format!(
                    "<circle class=\"sequence-create\" data-participant=\"{}\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#dcfce7\" stroke=\"#15803d\" stroke-width=\"1.5\"/>",
                    escape_text(&marker.participant_id),
                    marker.x,
                    marker.y
                ));
            }
            LifecycleMarkerKind::Destroy => {
                out.push_str(&format!(
                    "<g class=\"sequence-destroy\" data-participant=\"{}\" stroke=\"#b91c1c\" stroke-width=\"2\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/></g>",
                    escape_text(&marker.participant_id),
                    marker.x - 6,
                    marker.y - 6,
                    marker.x + 6,
                    marker.y + 6,
                    marker.x - 6,
                    marker.y + 6,
                    marker.x + 6,
                    marker.y - 6
                ));
            }
        }
    }
}
