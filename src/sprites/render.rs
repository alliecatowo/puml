use super::{SpriteDefinition, SpriteKind, SpriteRef};

pub fn render_sprite(def: &SpriteDefinition, x: f32, y: f32, reference: &SpriteRef) -> String {
    let scale = reference.scale;
    match &def.kind {
        SpriteKind::Svg { source } => {
            let color = reference.color.as_deref().unwrap_or("#111827");
            format!(
            "<g class=\"puml-sprite puml-sprite-svg\" data-sprite=\"{}\" transform=\"translate({x:.2},{y:.2}) scale({scale:.3})\" fill=\"{}\">{}</g>",
            escape_attr(&def.name),
            escape_attr(color),
            source
        )
        }
        SpriteKind::Monochrome { pixels } => {
            let color = reference.color.as_deref().unwrap_or("#111827");
            let mut out = format!(
                "<g class=\"puml-sprite\" data-sprite=\"{}\" transform=\"translate({x:.2},{y:.2}) scale({scale:.3})\">",
                escape_attr(&def.name)
            );
            out.push_str(&format!(
                "<metadata data-sprite-width=\"{}\" data-sprite-height=\"{}\" data-sprite-gray-levels=\"{}\"/>",
                def.width, def.height, def.gray_levels
            ));
            for row in 0..def.height {
                for col in 0..def.width {
                    let idx = (row * def.width + col) as usize;
                    let value = pixels.get(idx).copied().unwrap_or_default();
                    if value == 0 {
                        continue;
                    }
                    let opacity = (value as f32
                        / (def.gray_levels.saturating_sub(1).max(1) as f32))
                        .clamp(0.0, 1.0);
                    out.push_str(&format!(
                        "<rect x=\"{col}\" y=\"{row}\" width=\"1\" height=\"1\" fill=\"{}\" fill-opacity=\"{opacity:.3}\"/>",
                        escape_attr(color)
                    ));
                }
            }
            out.push_str("</g>");
            out
        }
    }
}

fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
