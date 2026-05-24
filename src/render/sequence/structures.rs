use super::super::svg::creole_text;
use crate::scene::{Scene, StructureKind};

pub(super) fn render_sequence_structures(out: &mut String, scene: &Scene) {
    for s in &scene.structures {
        match s.kind {
            StructureKind::Delay => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#777\" stroke-width=\"1\" stroke-dasharray=\"3 7\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&creole_text(
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#444\"",
                        label,
                        "#444",
                    ));
                }
            }
            StructureKind::Divider => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1\" stroke-dasharray=\"8 5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&creole_text(
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\"",
                        label,
                        "#333",
                    ));
                }
            }
            StructureKind::Separator => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                let label = if let Some(label) = &s.label {
                    format!("== {} ==", label)
                } else {
                    "== ==".to_string()
                };
                out.push_str(&creole_text(
                    (s.x1 + s.x2) / 2,
                    s.y - 6,
                    "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#222\"",
                    &label,
                    "#222",
                ));
            }
            StructureKind::Spacer => {}
        }
    }
}
