use std::collections::BTreeMap;
use std::io::{Read, Write};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;

use crate::diagnostic::Diagnostic;

const ENCODE_6BIT: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteDefinition {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub gray_levels: u8,
    pub kind: SpriteKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpriteKind {
    Monochrome { pixels: Vec<u8> },
    Svg { source: String },
}

pub type SpriteRegistry = BTreeMap<String, SpriteDefinition>;

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRef {
    pub name: String,
    pub scale: f32,
    pub color: Option<String>,
}

pub fn normalize_sprite_name(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('$')
        .trim_matches('"')
        .trim()
        .to_string()
}

pub fn parse_sprite_ref_at(input: &str) -> Option<(SpriteRef, usize)> {
    let rest = input.strip_prefix("<$")?;
    let close = rest.find('>')?;
    let inner = &rest[..close];
    let consumed = close + 3;
    let parsed = parse_sprite_ref_inner(inner)?;
    Some((parsed, consumed))
}

fn parse_sprite_ref_inner(inner: &str) -> Option<SpriteRef> {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (name_part, params_part) = if let Some((name, params)) = trimmed.split_once('{') {
        let params = params.strip_suffix('}')?;
        (name.trim(), Some(params.trim()))
    } else if let Some((name, scale)) = trimmed.split_once('*') {
        return Some(SpriteRef {
            name: normalize_sprite_name(name),
            scale: parse_scale(scale.trim()).unwrap_or(1.0),
            color: None,
        });
    } else if let Some((name, params)) = trimmed.split_once(',') {
        (name.trim(), Some(params.trim()))
    } else {
        (trimmed, None)
    };

    let mut sprite_ref = SpriteRef {
        name: normalize_sprite_name(name_part),
        scale: 1.0,
        color: None,
    };
    if sprite_ref.name.is_empty() {
        return None;
    }

    if let Some(params) = params_part {
        apply_sprite_ref_params(params, &mut sprite_ref);
    }
    Some(sprite_ref)
}

fn apply_sprite_ref_params(params: &str, sprite_ref: &mut SpriteRef) {
    for token in params
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        if let Some((key, value)) = token.split_once('=') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "scale" => {
                    if let Some(scale) = parse_scale(value) {
                        sprite_ref.scale = scale;
                    }
                }
                "color" | "colour" if !value.is_empty() => {
                    sprite_ref.color = Some(value.to_string());
                }
                _ => {}
            }
        } else if let Some(scale) = parse_scale(token) {
            sprite_ref.scale = scale;
        }
    }
}

fn parse_scale(raw: &str) -> Option<f32> {
    let value = raw.parse::<f32>().ok()?;
    if value.is_finite() && value > 0.0 {
        Some(value.clamp(0.05, 32.0))
    } else {
        None
    }
}

pub fn parse_hex_grid_sprite(
    name: &str,
    width_hint: Option<u32>,
    height_hint: Option<u32>,
    gray_levels: u8,
    rows: &[String],
) -> Result<SpriteDefinition, Diagnostic> {
    if rows.is_empty() {
        return Err(Diagnostic::error(
            "[E_SPRITE_INVALID] sprite grid must contain at least one row",
        ));
    }
    let width =
        width_hint.unwrap_or_else(|| rows.iter().map(|row| row.len()).max().unwrap_or(0) as u32);
    let height = height_hint.unwrap_or(rows.len() as u32);
    if width == 0 || height == 0 {
        return Err(Diagnostic::error(
            "[E_SPRITE_INVALID] sprite dimensions must be positive",
        ));
    }
    if rows.len() as u32 != height {
        return Err(Diagnostic::error(format!(
            "[E_SPRITE_INVALID] sprite height says {height} rows but found {}",
            rows.len()
        )));
    }

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for row in rows {
        let compact = row.trim();
        if compact.len() as u32 != width {
            return Err(Diagnostic::error(format!(
                "[E_SPRITE_INVALID] sprite row width says {width} pixels but found {}",
                compact.len()
            )));
        }
        for ch in compact.chars() {
            let Some(value) = ch.to_digit(16) else {
                return Err(Diagnostic::error(format!(
                    "[E_SPRITE_INVALID] sprite grid contains non-hex pixel `{ch}`"
                )));
            };
            pixels.push(scale_to_gray_levels(value as u8, 16, gray_levels));
        }
    }

    Ok(SpriteDefinition {
        name: normalize_sprite_name(name),
        width,
        height,
        gray_levels,
        kind: SpriteKind::Monochrome { pixels },
    })
}

pub fn parse_packed_sprite(
    name: &str,
    width: u32,
    height: u32,
    gray_levels: u8,
    compressed: bool,
    payload: &str,
) -> Result<SpriteDefinition, Diagnostic> {
    if width == 0 || height == 0 {
        return Err(Diagnostic::error(
            "[E_SPRITE_INVALID] sprite dimensions must be positive",
        ));
    }
    let pixels = if compressed {
        decode_compressed_pixels(payload, width, height, gray_levels)?
    } else {
        decode_uncompressed_pixels(payload, width, height, gray_levels)?
    };
    Ok(SpriteDefinition {
        name: normalize_sprite_name(name),
        width,
        height,
        gray_levels,
        kind: SpriteKind::Monochrome { pixels },
    })
}

pub fn parse_svg_sprite(name: &str, source: &str) -> Result<SpriteDefinition, Diagnostic> {
    let normalized = normalize_sprite_name(name);
    if normalized.is_empty() {
        return Err(Diagnostic::error(
            "[E_SPRITE_INVALID] sprite name cannot be empty",
        ));
    }
    let width = parse_svg_dimension(source, "width")
        .or_else(|| parse_svg_viewbox_dimension(source, 2))
        .unwrap_or(16.0)
        .ceil()
        .max(1.0) as u32;
    let height = parse_svg_dimension(source, "height")
        .or_else(|| parse_svg_viewbox_dimension(source, 3))
        .unwrap_or(16.0)
        .ceil()
        .max(1.0) as u32;
    Ok(SpriteDefinition {
        name: normalized,
        width,
        height,
        gray_levels: 16,
        kind: SpriteKind::Svg {
            source: source.to_string(),
        },
    })
}

pub fn builtin_sprite(name: &str, seed: &str) -> SpriteDefinition {
    let normalized = normalize_sprite_name(name);
    let mut pixels = vec![0_u8; 16 * 16];
    for y in 0..16_usize {
        for x in 0..16_usize {
            let border = x == 0 || y == 0 || x == 15 || y == 15;
            let diagonal = (x + y + seed.len()).is_multiple_of(7);
            let value = if border {
                15
            } else if diagonal {
                11
            } else if x > 3 && x < 12 && y > 3 && y < 12 {
                6
            } else {
                0
            };
            pixels[y * 16 + x] = value;
        }
    }
    SpriteDefinition {
        name: normalized,
        width: 16,
        height: 16,
        gray_levels: 16,
        kind: SpriteKind::Monochrome { pixels },
    }
}

pub fn render_sprite(def: &SpriteDefinition, x: f32, y: f32, reference: &SpriteRef) -> String {
    let scale = reference.scale;
    match &def.kind {
        SpriteKind::Svg { source } => format!(
            "<g class=\"puml-sprite puml-sprite-svg\" data-sprite=\"{}\" transform=\"translate({x:.2},{y:.2}) scale({scale:.3})\">{}</g>",
            escape_attr(&def.name),
            source
        ),
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
                    let opacity = (value as f32 / (def.gray_levels.saturating_sub(1).max(1) as f32))
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

pub fn encode_pixels(
    name: &str,
    width: u32,
    height: u32,
    gray_levels: u8,
    compressed: bool,
    pixels_16: &[u8],
) -> Result<String, Diagnostic> {
    let name = normalize_sprite_name(name);
    if pixels_16.len() != (width * height) as usize {
        return Err(Diagnostic::error(
            "[E_SPRITE_ENCODE_INVALID] pixel buffer does not match image dimensions",
        ));
    }
    let level = validate_gray_levels(gray_levels)?;
    let body = if compressed {
        let coef = 16 / level;
        let raw = pixels_16
            .iter()
            .map(|px| (px / coef).min(level - 1))
            .collect::<Vec<_>>();
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&raw)
            .map_err(|e| Diagnostic::error(format!("[E_SPRITE_ENCODE_IO] {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| Diagnostic::error(format!("[E_SPRITE_ENCODE_IO] {e}")))?;
        trim_final_zero_chars(&encode_plantuml_6bit(&compressed))
    } else {
        encode_uncompressed_pixels(width, height, level, pixels_16)
    };
    let suffix = if compressed { "z" } else { "" };
    if compressed {
        Ok(format!(
            "sprite ${name} [{width}x{height}/{level}{suffix}] {body}"
        ))
    } else {
        let lines = body
            .lines()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!(
            "sprite ${name} [{width}x{height}/{level}{suffix}] {{\n{lines}\n}}"
        ))
    }
}

pub fn parse_sprite_header_spec(raw: &str) -> Option<(u32, u32, u8, bool)> {
    let spec = raw.trim().strip_prefix('[')?.strip_suffix(']')?;
    let (size, depth) = spec.split_once('/')?;
    let (w, h) = size.split_once('x').or_else(|| size.split_once('X'))?;
    let width = w.trim().parse::<u32>().ok()?;
    let height = h.trim().parse::<u32>().ok()?;
    let depth = depth.trim();
    let compressed = depth.ends_with('z') || depth.ends_with('Z');
    let level_text = if compressed {
        &depth[..depth.len().saturating_sub(1)]
    } else {
        depth
    };
    let levels = level_text.parse::<u8>().ok()?;
    validate_gray_levels(levels).ok()?;
    Some((width, height, levels, compressed))
}

fn validate_gray_levels(levels: u8) -> Result<u8, Diagnostic> {
    if matches!(levels, 4 | 8 | 16) {
        Ok(levels)
    } else {
        Err(Diagnostic::error(format!(
            "[E_SPRITE_INVALID] sprite gray level must be 4, 8, or 16, got {levels}"
        )))
    }
}

fn decode_uncompressed_pixels(
    payload: &str,
    width: u32,
    height: u32,
    gray_levels: u8,
) -> Result<Vec<u8>, Diagnostic> {
    let compact = payload.lines().map(str::trim).collect::<Vec<_>>().join("");
    match gray_levels {
        16 => {
            let rows = compact
                .as_bytes()
                .chunks(width as usize)
                .map(|chunk| String::from_utf8_lossy(chunk).to_string())
                .collect::<Vec<_>>();
            parse_hex_grid_sprite("$inline", Some(width), Some(height), 16, &rows).map(|def| {
                if let SpriteKind::Monochrome { pixels } = def.kind {
                    pixels
                } else {
                    Vec::new()
                }
            })
        }
        8 => decode_packed_6bit(&compact, width, height, 8),
        4 => decode_packed_6bit(&compact, width, height, 4),
        _ => validate_gray_levels(gray_levels).map(|_| Vec::new()),
    }
}

fn decode_compressed_pixels(
    payload: &str,
    width: u32,
    height: u32,
    gray_levels: u8,
) -> Result<Vec<u8>, Diagnostic> {
    validate_gray_levels(gray_levels)?;
    let mut encoded = decode_plantuml_6bit(payload)?;
    encoded.extend(std::iter::repeat_n(0, 256));
    let mut decoder = DeflateDecoder::new(encoded.as_slice());
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw).map_err(|e| {
        Diagnostic::error(format!(
            "[E_SPRITE_INVALID] unable to decompress encoded sprite payload: {e}"
        ))
    })?;
    let expected = (width * height) as usize;
    if raw.len() < expected {
        return Err(Diagnostic::error(format!(
            "[E_SPRITE_INVALID] compressed sprite has {} pixels but expected {expected}",
            raw.len()
        )));
    }
    Ok(raw
        .into_iter()
        .take(expected)
        .map(|value| value.min(gray_levels.saturating_sub(1)))
        .collect())
}

fn decode_packed_6bit(
    compact: &str,
    width: u32,
    height: u32,
    gray_levels: u8,
) -> Result<Vec<u8>, Diagnostic> {
    let row_group = if gray_levels == 8 { 2 } else { 3 };
    let encoded_rows = height.div_ceil(row_group);
    let expected = (width * encoded_rows) as usize;
    if compact.chars().count() < expected {
        return Err(Diagnostic::error(format!(
            "[E_SPRITE_INVALID] packed sprite has {} payload cells but expected {expected}",
            compact.chars().count()
        )));
    }
    let mut pixels = vec![0_u8; (width * height) as usize];
    for (idx, ch) in compact.chars().take(expected).enumerate() {
        let value = decode_6bit(ch).ok_or_else(|| {
            Diagnostic::error(format!(
                "[E_SPRITE_INVALID] packed sprite contains invalid 6-bit character `{ch}`"
            ))
        })?;
        let col = (idx as u32) % width;
        let group = (idx as u32) / width;
        if gray_levels == 8 {
            let a = value / 8;
            let b = value % 8;
            set_packed_pixel(&mut pixels, width, height, col, group * 2, a);
            set_packed_pixel(&mut pixels, width, height, col, group * 2 + 1, b);
        } else {
            let a = value / 16;
            let rem = value % 16;
            let b = rem / 4;
            let c = rem % 4;
            set_packed_pixel(&mut pixels, width, height, col, group * 3, a);
            set_packed_pixel(&mut pixels, width, height, col, group * 3 + 1, b);
            set_packed_pixel(&mut pixels, width, height, col, group * 3 + 2, c);
        }
    }
    Ok(pixels)
}

fn set_packed_pixel(pixels: &mut [u8], width: u32, height: u32, col: u32, row: u32, value: u8) {
    if row < height {
        pixels[(row * width + col) as usize] = value;
    }
}

fn scale_to_gray_levels(value: u8, from_levels: u8, to_levels: u8) -> u8 {
    if to_levels == from_levels {
        return value.min(to_levels.saturating_sub(1));
    }
    let from_max = from_levels.saturating_sub(1).max(1) as f32;
    let to_max = to_levels.saturating_sub(1).max(1) as f32;
    ((value as f32 / from_max) * to_max).round() as u8
}

fn encode_uncompressed_pixels(
    width: u32,
    height: u32,
    gray_levels: u8,
    pixels_16: &[u8],
) -> String {
    match gray_levels {
        16 => {
            let mut lines = Vec::new();
            for row in 0..height {
                let mut line = String::new();
                for col in 0..width {
                    let value = pixels_16[(row * width + col) as usize].min(15);
                    line.push(
                        char::from_digit(value as u32, 16)
                            .unwrap()
                            .to_ascii_uppercase(),
                    );
                }
                lines.push(line);
            }
            lines.join("\n")
        }
        8 => encode_packed_rows(width, height, gray_levels, 2, pixels_16),
        4 => encode_packed_rows(width, height, gray_levels, 3, pixels_16),
        _ => String::new(),
    }
}

fn encode_packed_rows(
    width: u32,
    height: u32,
    gray_levels: u8,
    row_group: u32,
    pixels_16: &[u8],
) -> String {
    let coef = 16 / gray_levels;
    let mut lines = Vec::new();
    for row in (0..height).step_by(row_group as usize) {
        let mut line = String::new();
        for col in 0..width {
            let value = if gray_levels == 8 {
                let a = packed_source_pixel(width, height, pixels_16, col, row) / coef;
                let b = packed_source_pixel(width, height, pixels_16, col, row + 1) / coef;
                a * 8 + b
            } else {
                let a = packed_source_pixel(width, height, pixels_16, col, row) / coef;
                let b = packed_source_pixel(width, height, pixels_16, col, row + 1) / coef;
                let c = packed_source_pixel(width, height, pixels_16, col, row + 2) / coef;
                a * 16 + b * 4 + c
            };
            line.push(ENCODE_6BIT[value as usize] as char);
        }
        lines.push(line);
    }
    lines.join("\n")
}

fn packed_source_pixel(width: u32, height: u32, pixels_16: &[u8], col: u32, row: u32) -> u8 {
    if col >= width || row >= height {
        0
    } else {
        pixels_16[(row * width + col) as usize].min(15)
    }
}

fn encode_plantuml_6bit(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b1 = chunk.first().copied().unwrap_or_default();
        let b2 = chunk.get(1).copied().unwrap_or_default();
        let b3 = chunk.get(2).copied().unwrap_or_default();
        let c1 = b1 >> 2;
        let c2 = ((b1 & 0x03) << 4) | (b2 >> 4);
        let c3 = ((b2 & 0x0f) << 2) | (b3 >> 6);
        let c4 = b3 & 0x3f;
        out.push(ENCODE_6BIT[c1 as usize] as char);
        out.push(ENCODE_6BIT[c2 as usize] as char);
        out.push(ENCODE_6BIT[c3 as usize] as char);
        out.push(ENCODE_6BIT[c4 as usize] as char);
    }
    out
}

fn decode_plantuml_6bit(payload: &str) -> Result<Vec<u8>, Diagnostic> {
    let compact = payload.split_whitespace().collect::<String>();
    let chars = compact.chars().collect::<Vec<_>>();
    let len = if chars.len() % 4 == 0 {
        chars.len()
    } else {
        chars.len() + (4 - (chars.len() % 4))
    };
    let mut out = Vec::with_capacity((len * 3).div_ceil(4));
    for idx in (0..len).step_by(4) {
        let c1 = decode_6bit(chars.get(idx).copied().unwrap_or('0')).ok_or_else(|| {
            Diagnostic::error("[E_SPRITE_INVALID] invalid encoded sprite character")
        })?;
        let c2 = decode_6bit(chars.get(idx + 1).copied().unwrap_or('0')).ok_or_else(|| {
            Diagnostic::error("[E_SPRITE_INVALID] invalid encoded sprite character")
        })?;
        let c3 = decode_6bit(chars.get(idx + 2).copied().unwrap_or('0')).ok_or_else(|| {
            Diagnostic::error("[E_SPRITE_INVALID] invalid encoded sprite character")
        })?;
        let c4 = decode_6bit(chars.get(idx + 3).copied().unwrap_or('0')).ok_or_else(|| {
            Diagnostic::error("[E_SPRITE_INVALID] invalid encoded sprite character")
        })?;
        out.push((c1 << 2) | (c2 >> 4));
        out.push(((c2 & 0x0f) << 4) | (c3 >> 2));
        out.push(((c3 & 0x03) << 6) | c4);
    }
    Ok(out)
}

fn decode_6bit(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some(ch as u8 - b'0'),
        'A'..='Z' => Some(ch as u8 - b'A' + 10),
        'a'..='z' => Some(ch as u8 - b'a' + 36),
        '-' => Some(62),
        '_' => Some(63),
        _ => None,
    }
}

fn trim_final_zero_chars(input: &str) -> String {
    input.trim_end_matches('0').to_string()
}

fn parse_svg_dimension(source: &str, attr: &str) -> Option<f32> {
    let needle = format!("{attr}=\"");
    let start = source.find(&needle)? + needle.len();
    let end = source[start..].find('"')? + start;
    source[start..end]
        .trim_end_matches("px")
        .parse::<f32>()
        .ok()
}

fn parse_svg_viewbox_dimension(source: &str, index: usize) -> Option<f32> {
    let lower = source.to_ascii_lowercase();
    let start = lower.find("viewbox=\"")? + "viewbox=\"".len();
    let end = source[start..].find('"')? + start;
    source[start..end]
        .split([',', ' ', '\t', '\n'])
        .filter(|part| !part.is_empty())
        .nth(index)?
        .parse::<f32>()
        .ok()
}

fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plantuml_compressed_sprite_payload() {
        let def = parse_packed_sprite(
            "$printer",
            15,
            15,
            8,
            true,
            "NOtH3W0W208HxFz_kMAhj7lHWpa1XC716sz0Pq4MVPEWfBHIuxP3L6kbTcizR8tAhzaqFvXwvFfPEqm0",
        )
        .expect("compressed sample should decode");
        assert_eq!(def.width, 15);
        assert_eq!(def.height, 15);
        let SpriteKind::Monochrome { pixels } = def.kind else {
            panic!("expected monochrome sprite")
        };
        assert_eq!(pixels.len(), 225);
        assert!(pixels.iter().any(|px| *px > 0));
    }

    #[test]
    fn parses_sprite_reference_options() {
        let (sprite_ref, consumed) =
            parse_sprite_ref_at("<$foo,scale=3.4,color=orange> rest").expect("sprite ref");
        assert_eq!(consumed, 29);
        assert_eq!(sprite_ref.name, "foo");
        assert!((sprite_ref.scale - 3.4).abs() < f32::EPSILON);
        assert_eq!(sprite_ref.color.as_deref(), Some("orange"));
    }
}
