use std::cell::RefCell;

use crate::creole::{render_creole_line_to_tspans, render_creole_to_svg_tspans, tokenize_creole};
use crate::sprites::{
    bootstrap_icon_sprite, material_icon_sprite, openiconic_sprite, openiconic_sprites,
    parse_openiconic_ref_at, parse_sprite_ref_at, render_sprite, SpriteDefinition, SpriteRef,
    SpriteRegistry,
};
use crate::text_markup::{decode_unicode_escapes, escape_svg_text};

thread_local! {
    static ACTIVE_SPRITES: RefCell<SpriteRegistry> = const { RefCell::new(SpriteRegistry::new()) };
}

pub(crate) fn with_sprite_registry<T>(sprites: &SpriteRegistry, f: impl FnOnce() -> T) -> T {
    ACTIVE_SPRITES.with(|cell| {
        let previous = cell.replace(sprites.clone());
        let result = f();
        let _ = cell.replace(previous);
        result
    })
}

pub(crate) fn render_sprite_sheet(sprites: &SpriteRegistry) -> String {
    // listsprites shows user-defined sprites merged with the openiconic set (PlantUML parity).
    // We intentionally exclude bootstrap/material builtins — including all 4471+ icons
    // produces a 196 000px-tall blank canvas (bug #1536).
    let mut sheet_sprites = openiconic_sprites();
    for (name, sprite) in sprites {
        sheet_sprites.insert(name.clone(), sprite.clone());
    }
    let sprites = sheet_sprites;
    let count = sprites.len();
    let row_h = 44_i32;
    let width = 420_i32;
    let height = (count.max(1) as i32 * row_h) + 32;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" data-sprite-list=\"true\" data-sprite-count=\"{count}\">"
    );
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>");
    out.push_str("<text x=\"16\" y=\"22\" font-family=\"monospace\" font-size=\"13\" font-weight=\"700\" fill=\"#111827\">Sprites</text>");
    if sprites.is_empty() {
        out.push_str("<text x=\"16\" y=\"52\" font-family=\"monospace\" font-size=\"12\" fill=\"#64748b\">No sprites defined</text>");
    }
    for (idx, (name, sprite)) in sprites.iter().enumerate() {
        let y = 42 + (idx as i32 * row_h);
        let scale = (24.0 / sprite.width.max(sprite.height).max(1) as f32).clamp(1.0, 4.0);
        let sprite_ref = SpriteRef {
            name: name.clone(),
            scale,
            color: None,
        };
        out.push_str(&render_sprite(sprite, 18.0, y as f32, &sprite_ref));
        out.push_str(&format!(
            "<text x=\"56\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#111827\">${}</text>",
            y + 16,
            escape_text(name)
        ));
    }
    out.push_str("</svg>");
    out
}

pub(crate) fn creole_text(
    x: i32,
    y: i32,
    extra_attrs: &str,
    label: &str,
    base_color: &str,
) -> String {
    if label_has_inline_sprite(label) {
        return render_text_with_inline_sprites(x, y, extra_attrs, label, base_color);
    }

    let lines = tokenize_creole(label);
    let label_lower = label.to_ascii_lowercase();
    let has_markup = label.contains("**")
        || label.contains("//")
        || label.contains("\"\"")
        || label.contains("__")
        || label.contains("--")
        || label.contains("~~")
        || label.contains('~')
        || label.contains("[[")
        || (!is_centered_sequence_divider_label(label, extra_attrs)
            && has_creole_block_line(label))
        || label_lower.contains("<color")
        || label_lower.contains("</color")
        || label_lower.contains("<size")
        || label_lower.contains("</size")
        || label_lower.contains("<font")
        || label_lower.contains("</font")
        || label_lower.contains("<back")
        || label_lower.contains("</back")
        || label_lower.contains("<code>")
        || label_lower.contains("</code>")
        || label_lower.contains("<plain>")
        || label_lower.contains("</plain>")
        || label_lower.contains("<b>")
        || label_lower.contains("</b>")
        || label_lower.contains("<i>")
        || label_lower.contains("</i>")
        || label_lower.contains("<u>")
        || label_lower.contains("<u:")
        || label_lower.contains("</u>")
        || label_lower.contains("<s>")
        || label_lower.contains("<s:")
        || label_lower.contains("</s>")
        || label_lower.contains("<w>")
        || label_lower.contains("<w:")
        || label_lower.contains("</w>")
        || label_lower.contains("<sub>")
        || label_lower.contains("</sub>")
        || label_lower.contains("<sup>")
        || label_lower.contains("</sup>")
        || label.contains("<&")
        || label_lower.contains("<img:")
        || label_lower.contains("<br")
        || label_lower.contains("<strong>")
        || label_lower.contains("</strong>")
        || label_lower.contains("<em>")
        || label_lower.contains("</em>")
        || label_lower.contains("<del>")
        || label_lower.contains("</del>")
        || label_lower.contains("<strike>")
        || label_lower.contains("</strike>")
        || label_lower.contains("<tt>")
        || label_lower.contains("</tt>");

    if !has_markup && lines.len() == 1 {
        // Fast path — no markup, single line: emit fill when the color is non-default
        // and extra_attrs does not already carry a fill (avoids duplicate attributes).
        let color_attr = if !base_color.is_empty()
            && base_color != "black"
            && base_color != "#000000"
            && base_color != "#000"
            && !extra_attrs.contains("fill=")
        {
            format!(" fill=\"{}\"", base_color)
        } else {
            String::new()
        };
        let attrs = if extra_attrs.is_empty() {
            color_attr
        } else {
            format!(" {}{}", extra_attrs, color_attr)
        };
        return format!(
            "<text x=\"{}\" y=\"{}\"{}>{}</text>",
            x,
            y,
            attrs,
            escape_text(label)
        );
    }

    let inner = render_creole_to_svg_tspans(&lines, x, base_color);
    let color_attr = if !base_color.is_empty()
        && base_color != "black"
        && base_color != "#000000"
        && base_color != "#000"
        && !extra_attrs.contains("fill=")
    {
        format!(" fill=\"{}\"", base_color)
    } else {
        String::new()
    };
    format!(
        "<text x=\"{}\" y=\"{}\"{}>{}</text>",
        x,
        y,
        if extra_attrs.is_empty() {
            color_attr
        } else {
            format!(" {}{}", extra_attrs, color_attr)
        },
        inner
    )
}

fn is_centered_sequence_divider_label(label: &str, extra_attrs: &str) -> bool {
    let trimmed = label.trim();
    extra_attrs.contains("text-anchor=\"middle\"")
        && trimmed.starts_with("==")
        && trimmed.ends_with("==")
        && trimmed.len() > 4
}

fn has_creole_block_line(label: &str) -> bool {
    label.lines().any(|line| {
        let trimmed = line.trim();
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with("|_") || trimmed_start.starts_with('|') {
            return true;
        }
        if let Some(rest) = trimmed_start.strip_prefix("<#") {
            if let Some(close) = rest.find('>') {
                if rest[close + 1..].starts_with('|') {
                    return true;
                }
            }
        }
        if trimmed.len() >= 4
            && matches!(trimmed.as_bytes().first(), Some(b'-' | b'=' | b'_'))
            && trimmed.bytes().all(|b| b == trimmed.as_bytes()[0])
        {
            return true;
        }
        if trimmed.len() >= 4 && trimmed.starts_with("..") && trimmed.ends_with("..") {
            return true;
        }
        // `====+ Title ====+` titled section divider: 3+ equals on each side.
        {
            let leading = trimmed.chars().take_while(|&ch| ch == '=').count();
            if leading >= 3 {
                let rest = &trimmed[leading..];
                let trailing = rest.chars().rev().take_while(|&ch| ch == '=').count();
                if trailing >= 3 && !rest[..rest.len() - trailing].trim().is_empty() {
                    return true;
                }
            }
        }

        let marker = trimmed_start.chars().next();
        if matches!(marker, Some('*' | '#')) {
            let marker = marker.unwrap_or_default();
            let depth = trimmed_start.chars().take_while(|&ch| ch == marker).count();
            if trimmed_start
                .get(depth..)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
            {
                return true;
            }
        }

        let level = trimmed_start.chars().take_while(|&ch| ch == '=').count();
        if (1..=4).contains(&level)
            && trimmed_start
                .get(level..)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        {
            return true;
        }

        // Definition list: `; Term` or `; Term : Definition`
        if let Some(rest) = trimmed.strip_prefix(';') {
            if rest.starts_with(char::is_whitespace) {
                return true;
            }
        }

        false
    })
}

fn active_sprite(name: &str) -> Option<SpriteDefinition> {
    ACTIVE_SPRITES
        .with(|cell| cell.borrow().get(name).cloned())
        .or_else(|| openiconic_sprite(name))
        .or_else(|| bootstrap_icon_sprite(name))
        .or_else(|| material_icon_sprite(name))
}

fn label_has_inline_sprite(label: &str) -> bool {
    let mut offset = 0usize;
    while offset < label.len() {
        if let Some((sprite_ref, _consumed)) = parse_inline_sprite_ref_at(&label[offset..]) {
            if active_sprite(&sprite_ref.name).is_some() {
                return true;
            }
        }
        let Some(ch) = label[offset..].chars().next() else {
            break;
        };
        offset += ch.len_utf8();
    }
    false
}

fn label_has_inline_sprite_marker(label: &str) -> bool {
    let mut offset = 0usize;
    while offset < label.len() {
        let rest = &label[offset..];
        if rest.starts_with("<$") || rest.starts_with("<&") || rest.starts_with('&') {
            return true;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        offset += ch.len_utf8();
    }
    false
}

fn render_text_with_inline_sprites(
    x: i32,
    y: i32,
    extra_attrs: &str,
    label: &str,
    base_color: &str,
) -> String {
    let attrs = text_attrs(extra_attrs, base_color);
    let mut out = String::from("<g data-creole-sprites=\"true\">");
    for (line_idx, line) in normalize_sprite_text_lines(label).iter().enumerate() {
        let baseline_y = y + (line_idx as i32 * 16);
        let mut cursor_x = x as f32;
        let mut i = 0usize;
        while i < line.len() {
            let rest = &line[i..];
            if let Some((mut sprite_ref, consumed)) = parse_inline_sprite_ref_at(rest) {
                if let Some(sprite) = active_sprite(&sprite_ref.name) {
                    if sprite_ref.color.is_none() && !base_color.is_empty() {
                        sprite_ref.color = Some(base_color.to_string());
                    }
                    let sprite_y =
                        baseline_y as f32 - (sprite.height as f32 * sprite_ref.scale) + 3.0;
                    out.push_str(&render_sprite(&sprite, cursor_x, sprite_y, &sprite_ref));
                    cursor_x += sprite.width as f32 * sprite_ref.scale + 3.0;
                    i += consumed;
                    continue;
                }
            }
            let next_sprite = next_inline_sprite_marker(rest).unwrap_or(rest.len());
            let text = &rest[..next_sprite.max(1).min(rest.len())];
            // Apply creole markup to the text segment so bold/italic/color etc.
            // are respected even within sprite-containing labels.
            let creole_lines = tokenize_creole(text);
            if creole_lines.len() == 1 {
                let line = &creole_lines[0];
                // If all spans are plain (no markup attributes), emit the text
                // directly inside <text> so plain labels like "edge gateway" are
                // not needlessly wrapped in <tspan> elements.
                let all_plain = line.iter().all(|s| {
                    !s.bold
                        && !s.italic
                        && !s.mono
                        && !s.underline
                        && !s.strike
                        && !s.wave
                        && s.color.is_none()
                        && s.background.is_none()
                        && s.size.is_none()
                        && s.font.is_none()
                        && s.baseline_shift.is_none()
                        && s.decoration_color.is_none()
                        && s.link.is_none()
                });
                if all_plain {
                    let plain_text: String = line.iter().map(|s| s.text.as_str()).collect();
                    out.push_str(&format!(
                        "<text x=\"{cursor_x:.2}\" y=\"{baseline_y}\"{}>{}</text>",
                        attrs,
                        escape_text(&plain_text)
                    ));
                } else {
                    let inner = render_creole_line_to_tspans(line, cursor_x as i32, base_color);
                    out.push_str(&format!(
                        "<text x=\"{cursor_x:.2}\" y=\"{baseline_y}\"{}>{}</text>",
                        attrs, inner
                    ));
                }
            } else {
                // Multiple lines within a text segment (shouldn't happen since
                // normalize_sprite_text_lines already split on line breaks, but
                // be safe).
                out.push_str(&format!(
                    "<text x=\"{cursor_x:.2}\" y=\"{baseline_y}\"{}>{}</text>",
                    attrs,
                    escape_text(text)
                ));
            }
            cursor_x += estimate_text_width(text);
            i += text.len();
        }
    }
    out.push_str("</g>");
    out
}

fn parse_inline_sprite_ref_at(input: &str) -> Option<(SpriteRef, usize)> {
    parse_sprite_ref_at(input).or_else(|| parse_openiconic_ref_at(input))
}

fn next_inline_sprite_marker(input: &str) -> Option<usize> {
    if !label_has_inline_sprite_marker(input) {
        return None;
    }
    ["<$", "<&", "&"]
        .iter()
        .filter_map(|needle| input.find(needle))
        .min()
}

fn normalize_sprite_text_lines(text: &str) -> Vec<String> {
    let normalized = text
        .replace("\\n", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    normalized.split('\n').map(str::to_string).collect()
}

fn text_attrs(extra_attrs: &str, base_color: &str) -> String {
    let color_attr = if !base_color.is_empty() && !extra_attrs.contains("fill=") {
        format!(" fill=\"{}\"", escape_text(base_color))
    } else {
        String::new()
    };
    if extra_attrs.is_empty() {
        color_attr
    } else {
        format!(" {}{}", extra_attrs, color_attr)
    }
}

fn estimate_text_width(text: &str) -> f32 {
    crate::render_core::text_metrics::estimate_text_width_f64(&decode_unicode_escapes(text), 14.0)
        as f32
}

/// Canonical actor stick-figure renderer used across all diagram families.
///
/// Proportions (canonical, issue #715):
///   head  r = 6   (12 px diameter)
///   body  14 px   (neck bottom to hip)
///   arms  20 px wide centred on cx, at shoulder (neck bottom + 4)
///   legs  16 px spread (each leg goes ±8 px from hip)
///
/// `cx`, `cy` are the **centre** of the figure. The full figure spans roughly
/// 44 px in height: from `cy - 21` (top of head) to `cy + 23` (feet).
/// `stroke` is the SVG stroke colour string (e.g. `"#334155"`).
pub(crate) fn render_actor_stick_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    // Head: centre at (cx, cy - 15) -> top of figure is cy - 21
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle cx=\"{cx}\" cy=\"{head_cy}\" r=\"6\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Body: from neck (head_cy + 6) to hip (head_cy + 20)
    let neck_y = head_cy + 6;
    let hip_y = head_cy + 20;
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{neck_y}\" x2=\"{cx}\" y2=\"{hip_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Arms: centred on body at shoulder (neck_y + 4), spanning cx±10
    let arm_y = neck_y + 4;
    let arm_x1 = cx - 10;
    let arm_x2 = cx + 10;
    out.push_str(&format!(
        "<line x1=\"{arm_x1}\" y1=\"{arm_y}\" x2=\"{arm_x2}\" y2=\"{arm_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Legs: from hip, spread cx±8
    let leg_x_left = cx - 8;
    let leg_x_right = cx + 8;
    let leg_end_y = hip_y + 16;
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{hip_y}\" x2=\"{leg_x_left}\" y2=\"{leg_end_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{hip_y}\" x2=\"{leg_x_right}\" y2=\"{leg_end_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
}

pub(crate) fn escape_text(input: &str) -> String {
    escape_svg_text(input)
}
