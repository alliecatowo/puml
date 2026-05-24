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
