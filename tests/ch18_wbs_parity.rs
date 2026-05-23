use puml::model::{FamilyStyle, NormalizedDocument};

const WBS_THEME_VIBRANT_SRC: &str = r##"@startwbs
!theme vibrant
* Platform Launch
** Build
*** API
*** UI
** Validate
*** QA
*** Rollout
@endwbs
"##;

#[test]
fn wbs_theme_preset_is_carried_as_depth_style() {
    let document = puml::parser::parse(WBS_THEME_VIBRANT_SRC).expect("parse themed wbs");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize themed wbs")
    else {
        panic!("wbs diagram should normalize as Family");
    };

    let Some(FamilyStyle::MindMap(style)) = model.family_style else {
        panic!("wbs theme should be carried via tree depth style");
    };

    assert_eq!(
        style
            .depth_styles
            .get(&0)
            .and_then(|depth| depth.background_color.as_deref()),
        Some("#f5f3ff")
    );
    assert_eq!(
        style
            .depth_styles
            .get(&1)
            .and_then(|depth| depth.background_color.as_deref()),
        Some("#ede9fe")
    );
    assert_eq!(
        style
            .depth_styles
            .get(&2)
            .and_then(|depth| depth.border_color.as_deref()),
        Some("#d97706")
    );
}

#[test]
fn wbs_theme_preset_colors_depth_nodes_but_keeps_inline_fill_override() {
    let src = r##"@startwbs
!theme vibrant
* Platform Launch
**[#pink] Build
*** API
@endwbs
"##;
    let svg = puml::render_source_to_svg(src).expect("render themed wbs");

    assert!(
        svg.contains("class=\"wbs-node wbs-depth-0")
            && svg.contains("data-wbs-fill=\"#f5f3ff\"")
            && svg.contains("stroke=\"#8b5cf6\""),
        "root should use vibrant group colors; svg={svg}"
    );
    assert!(
        svg.contains("data-wbs-fill=\"pink\""),
        "inline WBS fill should still override theme fill; svg={svg}"
    );
    assert!(
        svg.contains("data-wbs-fill=\"#fef3c7\"") && svg.contains("stroke=\"#d97706\""),
        "depth-2 leaf should use vibrant note colors; svg={svg}"
    );
}
