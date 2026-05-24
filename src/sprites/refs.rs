use super::icons::{is_openiconic_icon, normalize_openiconic_name};
use super::{normalize_sprite_name, SpriteRef};

pub fn parse_sprite_ref_at(input: &str) -> Option<(SpriteRef, usize)> {
    let rest = input.strip_prefix("<$")?;
    let close = rest.find('>')?;
    let inner = &rest[..close];
    let consumed = close + 3;
    let parsed = parse_sprite_ref_inner(inner)?;
    Some((parsed, consumed))
}

pub fn parse_openiconic_ref_at(input: &str) -> Option<(SpriteRef, usize)> {
    if let Some(rest) = input.strip_prefix("<&") {
        let close = rest.find('>')?;
        let inner = &rest[..close];
        let consumed = close + 3;
        let mut parsed = parse_sprite_ref_inner(inner)?;
        parsed.name = normalize_openiconic_name(&parsed.name);
        return Some((parsed, consumed));
    }

    let rest = input.strip_prefix('&')?;
    let name_len = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .map(char::len_utf8)
        .sum::<usize>();
    if name_len == 0 {
        return None;
    }
    let name = normalize_openiconic_name(&rest[..name_len]);
    if !is_openiconic_icon(&name) {
        return None;
    }
    Some((
        SpriteRef {
            name,
            scale: 1.0,
            color: None,
        },
        name_len + 1,
    ))
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
