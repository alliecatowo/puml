use std::io::Read;

use flate2::read::DeflateDecoder;

use crate::diagnostic::Diagnostic;

use super::{normalize_sprite_name, SpriteDefinition, SpriteKind};

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

pub(super) fn validate_gray_levels(levels: u8) -> Result<u8, Diagnostic> {
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
