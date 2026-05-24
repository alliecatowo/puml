use crate::ast::NoteKind;
use crate::scene::Scene;

pub(super) fn render_sequence_note_shape(
    out: &mut String,
    kind: NoteKind,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scene: &Scene,
) {
    let fill = &scene.style.note_background_color;
    let stroke = &scene.style.note_border_color;
    match kind {
        NoteKind::Folded => {
            let fold = 14.min(width / 4).min(height / 3).max(8);
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + width - fold,
                x + width,
                y + fold,
                y + height,
                fill,
                stroke
            ));
            out.push_str(&format!(
                "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + width - fold,
                y + fold,
                x + width,
                stroke
            ));
        }
        NoteKind::Hexagonal => {
            let cut = 16.min(width / 5).max(8);
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + cut,
                y,
                x + width - cut,
                y,
                x + width,
                y + height / 2,
                x + width - cut,
                y + height,
                x + cut,
                y + height,
                x,
                y + height / 2,
                fill,
                stroke
            ));
        }
        NoteKind::Rectangle => {
            out.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                fill,
                stroke
            ));
        }
    }
}
