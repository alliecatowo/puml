use crate::bootstrap_icons::BOOTSTRAP_ICONS_SVG;
use crate::material_icons::MATERIAL_ICONS_SVG;
use crate::openiconic::OPENICONIC_SVG;

use super::{normalize_sprite_name, parse_svg_sprite, SpriteDefinition, SpriteRegistry};

pub fn openiconic_sprite(name: &str) -> Option<SpriteDefinition> {
    let (normalized, source) = openiconic_svg_source(name)?;
    parse_svg_sprite(&normalized, source).ok()
}

pub fn openiconic_svg_source(name: &str) -> Option<(String, &'static str)> {
    let normalized = normalize_openiconic_name(name);
    let source = OPENICONIC_SVG
        .iter()
        .find_map(|(icon_name, svg)| (*icon_name == normalized).then_some(*svg))?;
    Some((normalized, source))
}

pub fn openiconic_icon_names() -> Vec<&'static str> {
    let mut names = OPENICONIC_SVG
        .iter()
        .map(|(name, _svg)| *name)
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    names
}

pub fn openiconic_sprites() -> SpriteRegistry {
    OPENICONIC_SVG
        .iter()
        .filter_map(|(name, svg)| {
            parse_svg_sprite(name, svg)
                .ok()
                .map(|sprite| ((*name).to_string(), sprite))
        })
        .collect()
}

pub fn bootstrap_icon_sprite(name: &str) -> Option<SpriteDefinition> {
    let normalized = normalize_bootstrap_icon_name(name)?;
    let icon_name = normalized.strip_prefix("bi-").unwrap_or(&normalized);
    let source = BOOTSTRAP_ICONS_SVG
        .iter()
        .find_map(|(name, svg)| (*name == icon_name).then_some(*svg))?;
    parse_svg_sprite(&normalized, source).ok()
}

pub fn bootstrap_icon_sprites() -> SpriteRegistry {
    BOOTSTRAP_ICONS_SVG
        .iter()
        .filter_map(|(name, svg)| {
            let sprite_name = format!("bi-{name}");
            parse_svg_sprite(&sprite_name, svg)
                .ok()
                .map(|sprite| (sprite_name, sprite))
        })
        .collect()
}

pub fn material_icon_sprite(name: &str) -> Option<SpriteDefinition> {
    let normalized = normalize_material_icon_name(name)?;
    let icon_name = normalized.strip_prefix("ma_").unwrap_or(&normalized);
    let source = MATERIAL_ICONS_SVG
        .iter()
        .find_map(|(name, svg)| (*name == icon_name).then_some(*svg))?;
    parse_svg_sprite(&normalized, source).ok()
}

pub fn material_icon_sprites() -> SpriteRegistry {
    MATERIAL_ICONS_SVG
        .iter()
        .filter_map(|(name, svg)| {
            let sprite_name = format!("ma_{name}");
            parse_svg_sprite(&sprite_name, svg)
                .ok()
                .map(|sprite| (sprite_name, sprite))
        })
        .collect()
}

pub(super) fn is_openiconic_icon(name: &str) -> bool {
    OPENICONIC_SVG
        .iter()
        .any(|(icon_name, _svg)| *icon_name == name)
}

pub(super) fn normalize_openiconic_name(raw: &str) -> String {
    let normalized = normalize_sprite_name(raw)
        .trim_start_matches('&')
        .replace('_', "-")
        .to_ascii_lowercase();
    normalized
        .strip_prefix("oi-")
        .unwrap_or(&normalized)
        .to_string()
}

fn normalize_bootstrap_icon_name(raw: &str) -> Option<String> {
    let normalized = normalize_sprite_name(raw)
        .replace('_', "-")
        .to_ascii_lowercase();
    let icon_name = normalized.strip_prefix("bi-")?;
    (!icon_name.is_empty()).then(|| format!("bi-{icon_name}"))
}

fn normalize_material_icon_name(raw: &str) -> Option<String> {
    let normalized = normalize_sprite_name(raw).to_ascii_lowercase();
    let icon_name = normalized
        .strip_prefix("ma_")
        .or_else(|| normalized.strip_prefix("ma-"))?
        .replace('-', "_");
    (!icon_name.is_empty()).then(|| format!("ma_{icon_name}"))
}
