mod encode;
mod icons;
mod model;
mod parse;
mod refs;
mod render;

pub use encode::encode_pixels;
pub use icons::{
    bootstrap_icon_sprite, bootstrap_icon_sprites, material_icon_sprite, material_icon_sprites,
    openiconic_sprite, openiconic_sprites, openiconic_svg_source,
};
pub use model::{builtin_sprite, normalize_sprite_name, SpriteDefinition, SpriteKind, SpriteRef};
pub use parse::{
    parse_hex_grid_sprite, parse_packed_sprite, parse_sprite_header_spec, parse_svg_sprite,
};
pub use refs::{parse_openiconic_ref_at, parse_sprite_ref_at};
pub use render::render_sprite;

pub type SpriteRegistry = std::collections::BTreeMap<String, SpriteDefinition>;

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

    #[test]
    fn parses_openiconic_references_and_loads_svg_sprites() {
        let (tag_ref, consumed) =
            parse_openiconic_ref_at("<&folder,scale=2,color=#2563eb> rest").expect("icon ref");
        assert_eq!(consumed, "<&folder,scale=2,color=#2563eb>".len());
        assert_eq!(tag_ref.name, "folder");
        assert_eq!(tag_ref.scale, 2.0);
        assert_eq!(tag_ref.color.as_deref(), Some("#2563eb"));

        let (bare_ref, consumed) =
            parse_openiconic_ref_at("&cloud_upload done").expect("bare icon ref");
        assert_eq!(consumed, "&cloud_upload".len());
        assert_eq!(bare_ref.name, "cloud-upload");

        assert!(parse_openiconic_ref_at("&definitely-not-openiconic").is_none());

        let folder = openiconic_sprite("folder").expect("folder icon");
        assert_eq!((folder.width, folder.height), (8, 8));
        assert!(matches!(folder.kind, SpriteKind::Svg { .. }));
        assert_eq!(openiconic_sprites().len(), 223);
    }

    #[test]
    fn loads_bootstrap_icons_with_prefixed_names() {
        let globe = bootstrap_icon_sprite("bi-globe").expect("globe icon");
        assert_eq!((globe.width, globe.height), (16, 16));
        assert_eq!(globe.name, "bi-globe");
        assert!(matches!(globe.kind, SpriteKind::Svg { .. }));

        let alias = bootstrap_icon_sprite("bi_bootstrap_fill").expect("underscore alias");
        assert_eq!(alias.name, "bi-bootstrap-fill");
        assert!(bootstrap_icon_sprite("globe").is_none());
        assert_eq!(bootstrap_icon_sprites().len(), 2078);
    }

    #[test]
    fn loads_material_icons_with_prefixed_names() {
        let folder = material_icon_sprite("ma_folder").expect("folder icon");
        assert_eq!((folder.width, folder.height), (24, 24));
        assert_eq!(folder.name, "ma_folder");
        assert!(matches!(folder.kind, SpriteKind::Svg { .. }));

        let alias = material_icon_sprite("ma-cloud-upload").expect("hyphen alias");
        assert_eq!(alias.name, "ma_cloud_upload");
        assert!(material_icon_sprite("folder").is_none());
        assert_eq!(material_icon_sprites().len(), 2170);
    }
}
