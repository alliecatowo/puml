use super::{EXIT_IO, EXIT_VALIDATION};
use std::path::Path;

pub(super) fn run_encodesprite(args: &[String]) -> Result<(), (u8, String)> {
    let [format, image_path] = args else {
        return Err((
            EXIT_VALIDATION,
            "encodesprite requires a format and image path".to_string(),
        ));
    };
    let (gray_levels, compressed) = parse_sprite_encode_format(format)?;
    let path = Path::new(image_path);
    let image = image::open(path)
        .map_err(|e| {
            (
                EXIT_IO,
                format!("failed to read image '{}': {e}", path.display()),
            )
        })?
        .to_rgba8();
    let width = image.width();
    let height = image.height();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        let luminance = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;
        let alpha = a as f32 / 255.0;
        let darkness = (255_u8.saturating_sub(luminance)) as f32 / 255.0;
        pixels.push(((darkness * alpha * 15.0).round() as u8).min(15));
    }
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sprite")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let encoded =
        puml::sprites::encode_pixels(&name, width, height, gray_levels, compressed, &pixels)
            .map_err(|d| (EXIT_VALIDATION, d.message))?;
    println!("{encoded}");
    Ok(())
}

fn parse_sprite_encode_format(raw: &str) -> Result<(u8, bool), (u8, String)> {
    let trimmed = raw.trim();
    let compressed = trimmed.ends_with('z') || trimmed.ends_with('Z');
    let level_text = if compressed {
        &trimmed[..trimmed.len().saturating_sub(1)]
    } else {
        trimmed
    };
    let gray_levels = level_text.parse::<u8>().map_err(|_| {
        (
            EXIT_VALIDATION,
            format!("invalid encodesprite format `{raw}`; expected 4, 8, 16, 4z, 8z, or 16z"),
        )
    })?;
    if matches!(gray_levels, 4 | 8 | 16) {
        Ok((gray_levels, compressed))
    } else {
        Err((
            EXIT_VALIDATION,
            format!("invalid encodesprite format `{raw}`; expected 4, 8, 16, 4z, 8z, or 16z"),
        ))
    }
}
