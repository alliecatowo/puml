use std::io::Write;

use flate2::write::DeflateEncoder;
use flate2::Compression;

use crate::diagnostic::Diagnostic;

use super::normalize_sprite_name;
use super::parse::validate_gray_levels;

const ENCODE_6BIT: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";

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

fn trim_final_zero_chars(input: &str) -> String {
    input.trim_end_matches('0').to_string()
}
