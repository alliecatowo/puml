use puml::theme::{
    activity_style_from_sequence_theme, chart_style_from_sequence_theme,
    class_style_from_sequence_theme, classify_archimate_skinparam, classify_ditaa_skinparam,
    classify_gantt_skinparam, classify_mindmap_skinparam, classify_nwdiag_skinparam,
    classify_salt_skinparam, classify_sdl_skinparam, classify_timeline_skinparam,
    classify_wbs_skinparam, component_style_from_sequence_theme, css3_color_to_hex,
    hex_color_is_dark, mindmap_style_from_sequence_theme, resolve_sequence_theme_preset,
    state_style_from_sequence_theme, timing_style_from_sequence_theme, ComponentStyleMode,
    GenericSkinParamValue, SkinParamSupport, LOCAL_SEQUENCE_THEME_CATALOG,
};
use puml::{
    normalize_family, parse, render_source_to_svg_for_family, DiagramFamily, NormalizedDocument,
};

#[test]
fn theme_projection_and_color_helpers_cover_additive_branches() {
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        let mixed_case = format!("  {}  ", name.to_ascii_uppercase());
        let preset = resolve_sequence_theme_preset(&mixed_case)
            .unwrap_or_else(|err| panic!("theme {name} should resolve: {err}"));
        assert_eq!(preset.name, *name);
    }

    assert_eq!(css3_color_to_hex("RebeccaPurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("darkgrey"), Some("#a9a9a9"));
    assert_eq!(css3_color_to_hex("not-a-css-color"), None);
    assert!(hex_color_is_dark("#000"));
    assert!(hex_color_is_dark("#112233"));
    assert!(!hex_color_is_dark("#fff"));
    assert!(!hex_color_is_dark("#ggg"));
    assert!(!hex_color_is_dark("not-hex"));

    let preset = resolve_sequence_theme_preset("carbon-gray").expect("theme should resolve");
    assert_eq!(preset.style.background_color.as_deref(), Some("#161616"));

    let class_style = class_style_from_sequence_theme(&preset.style);
    assert_eq!(
        class_style.background_color,
        preset.style.participant_background_color
    );
    assert_eq!(
        class_style.header_color,
        preset.style.group_background_color
    );
    assert_eq!(class_style.member_color, preset.style.arrow_color);

    let state_style = state_style_from_sequence_theme(&preset.style);
    assert_eq!(state_style.start_color, preset.style.arrow_color);
    assert_eq!(state_style.font_color, preset.style.arrow_color);

    let component_style = component_style_from_sequence_theme(&preset.style);
    assert_eq!(
        component_style.interface_color,
        preset.style.note_background_color
    );
    assert_eq!(
        component_style.component_style_mode,
        ComponentStyleMode::Uml2
    );

    let activity_style = activity_style_from_sequence_theme(&preset.style);
    assert_eq!(
        activity_style.diamond_color,
        preset.style.note_background_color
    );

    let timing_style = timing_style_from_sequence_theme(&preset.style);
    assert_eq!(timing_style.background_color, "#161616");
    assert_eq!(timing_style.grid_color, preset.style.lifeline_border_color);

    let chart_style = chart_style_from_sequence_theme(&preset.style);
    assert_eq!(chart_style.background_color, "#161616");
    assert_eq!(
        chart_style.pie_border_color,
        preset.style.group_border_color
    );

    let mindmap_style = mindmap_style_from_sequence_theme(&preset.style);
    assert_eq!(mindmap_style.depth_styles.len(), 3);
    assert_eq!(
        mindmap_style.depth_styles[&0].background_color.as_deref(),
        Some(preset.style.group_background_color.as_str())
    );
    assert_eq!(
        mindmap_style.depth_styles[&2].border_color.as_deref(),
        Some(preset.style.note_border_color.as_str())
    );
}

#[test]
fn generic_family_skinparam_classifiers_cover_value_and_noop_branches() {
    type GenericClassifier = fn(&str, &str) -> SkinParamSupport<GenericSkinParamValue>;

    let cases: [(&str, GenericClassifier); 9] = [
        ("gantt", classify_gantt_skinparam),
        ("mindmap", classify_mindmap_skinparam),
        ("wbs", classify_wbs_skinparam),
        ("timeline", classify_timeline_skinparam),
        ("nwdiag", classify_nwdiag_skinparam),
        ("archimate", classify_archimate_skinparam),
        ("sdl", classify_sdl_skinparam),
        ("ditaa", classify_ditaa_skinparam),
        ("salt", classify_salt_skinparam),
    ];

    for (prefix, classify) in cases {
        assert_eq!(
            classify(&format!("{prefix}BackgroundColor"), "cornsilk"),
            SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
                "#fff8dc".to_string()
            ))
        );
        assert_eq!(
            classify(&format!("{prefix}BorderColor"), "#123abc"),
            SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(
                "#123abc".to_string()
            ))
        );
        assert_eq!(
            classify(&format!("{prefix}FontColor"), "slategrey"),
            SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
                "#708090".to_string()
            ))
        );
        assert_eq!(
            classify("fontSize", "17"),
            SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(17))
        );
        assert_eq!(
            classify("unsupportedKey", "red"),
            SkinParamSupport::UnsupportedKey
        );
    }

    assert_eq!(
        classify_mindmap_skinparam("roundCorner", "8"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_wbs_skinparam("fontName", "Fira Code"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_timeline_skinparam("timelineArrowColor", "red"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_nwdiag_skinparam("networkColor", "red"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_archimate_skinparam("archimateStyle", "business"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_archimate_skinparam("archimateontSize", "19"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(19))
    );
    assert_eq!(
        classify_sdl_skinparam("sdlArrowColor", "red"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_ditaa_skinparam("ditaaShadowing", "true"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_salt_skinparam("saltRoundCorner", "8"),
        SkinParamSupport::SupportedNoop
    );
}

#[test]
fn family_normalization_applies_theme_and_skinparams_without_rendering_side_effects() {
    let src = "@startuml\n\
!theme spacelab-white\n\
skinparam classBackgroundColor<<Entity>> #ffeecc\n\
skinparam classBorderColor<<Entity>> #884400\n\
skinparam classHeaderBackgroundColor<<Entity>> #ffe0aa\n\
skinparam classFontColor<<Entity>> #332211\n\
class User <<Entity>> {\n  +id: UUID\n}\n\
class Customer\n\
Customer -[#green,dashed,thickness=3]-> User : owns\n\
@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let normalized = normalize_family(doc).expect("normalize should succeed");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected family document");
    };
    let Some(puml::model::FamilyStyle::Class(style)) = family.family_style else {
        panic!("expected class style");
    };
    assert_eq!(style.background_color, "#ffffff");
    assert_eq!(style.arrow_color, "#2f4f6f");
    let entity = style
        .stereotype_styles
        .get("entity")
        .expect("stereotype style should be recorded");
    assert_eq!(entity.background_color.as_deref(), Some("#ffeecc"));
    assert_eq!(entity.border_color.as_deref(), Some("#884400"));
    assert_eq!(entity.header_color.as_deref(), Some("#ffe0aa"));
    assert_eq!(entity.font_color.as_deref(), Some("#332211"));
    assert_eq!(family.relations.len(), 1);
    assert_eq!(family.relations[0].line_color.as_deref(), Some("#008000"));
    assert!(family.relations[0].dashed);
    assert_eq!(family.relations[0].thickness, Some(3));
}

#[test]
fn class_style_cascade_applies_theme_skinparam_style_and_stereotype_to_svg() {
    let src = include_str!("fixtures/styling/valid_style_cascade_class.puml");
    let doc = parse(src).expect("class style cascade should parse");
    let normalized = normalize_family(doc).expect("class style cascade should normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected family document");
    };
    let Some(puml::model::FamilyStyle::Class(style)) = &family.family_style else {
        panic!("expected class family style");
    };
    assert_eq!(style.background_color, "#dbeafe");
    assert_eq!(style.border_color, "#1d4ed8");
    let service = style
        .stereotype_styles
        .get("service")
        .expect("style block stereotype selector should be retained");
    assert_eq!(service.background_color.as_deref(), Some("#dcfce7"));
    assert_eq!(service.border_color.as_deref(), Some("#15803d"));
    assert_eq!(service.header_color.as_deref(), Some("#bbf7d0"));
    assert_eq!(service.font_color.as_deref(), Some("#14532d"));

    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class)
        .expect("class style cascade should render");
    assert!(
        svg.contains("#dbeafe"),
        "plain class style should reach SVG"
    );
    assert!(
        svg.contains("#dcfce7"),
        "stereotype-scoped class fill should reach SVG"
    );
    assert!(
        svg.contains("#bbf7d0"),
        "stereotype-scoped class header should reach SVG"
    );
    assert!(
        svg.contains("#14532d"),
        "stereotype-scoped class font should reach SVG"
    );
}

#[test]
fn deployment_style_cascade_applies_theme_skinparam_and_style_to_svg() {
    let src = include_str!("fixtures/styling/valid_style_cascade_deployment.puml");
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Deployment)
        .expect("deployment style cascade should render");
    assert!(svg.contains("#ecfeff"), "node style fill should reach SVG");
    assert!(
        svg.contains("#0891b2"),
        "node style border should reach SVG"
    );
    assert!(
        svg.contains("#0f766e"),
        "style-block arrow color should reach SVG marker/edge output"
    );
    assert!(
        !svg.contains("#fee2e2"),
        "style block should override earlier deployment skinparam"
    );
}

#[test]
fn graph_style_unsupported_rules_warn_deterministically_but_render() {
    let src = "@startuml\n\
<style>\n\
classDiagram {\n\
  class {\n\
    TotallyMadeUp #123456\n\
  }\n\
}\n\
</style>\n\
class A\n\
@enduml\n";
    let doc = parse(src).expect("unsupported graph style should parse");
    let normalized = normalize_family(doc).expect("unsupported graph style should normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected family document");
    };
    assert_eq!(family.warnings.len(), 1);
    assert_eq!(
        family.warnings[0].message,
        "[W_STYLE_UNSUPPORTED] unsupported style `TotallyMadeUp` in selector `class`"
    );
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class)
        .expect("unsupported graph style should still render");
    assert!(svg.contains(">A<"));
}

#[test]
fn family_renderers_expose_style_relation_and_marker_branches_deterministically() {
    let component = "@startuml\n\
skinparam componentStyle rectangle\n\
skinparam componentBackgroundColor #eef6ff\n\
skinparam componentBorderColor #123456\n\
skinparam componentFontColor #102030\n\
skinparam componentArrowColor #445566\n\
skinparam interfaceColor #fedcba\n\
component API\n\
interface Events\n\
port HTTPS\n\
component Store\n\
API -[#dc2626;line.dashed;line.thickness=4]right-> Store <<writes>> : persists\n\
Events -[#0f766e;line.thick]down-() API : mounted\n\
Store -[line.hidden]-> HTTPS : layout only\n\
@enduml\n";
    let first = render_source_to_svg_for_family(component, DiagramFamily::Component)
        .expect("component should render");
    let second = render_source_to_svg_for_family(component, DiagramFamily::Component)
        .expect("component should render deterministically");
    assert_eq!(first, second);
    assert!(first.contains("data-component-style=\"rectangle\""));
    assert!(first.contains("fill=\"#eef6ff\""));
    assert!(first.contains("stroke=\"#123456\""));
    assert!(first.contains("fill=\"#102030\""));
    assert!(first.contains("stroke=\"#dc2626\""));
    assert!(first.contains("data-uml-relation-style=\"color:#dc2626 dashed thickness:4\""));
    assert!(first.contains("visibility=\"hidden\""));
    assert!(first.contains("data-uml-label-role=\"edge\""));
    assert!(first.contains("arrow-open") || first.contains("arrow-ie"));

    let deployment = "@startuml\n\
skinparam monochrome reverse\n\
node Web\n\
queue Jobs\n\
cloud Edge\n\
Web -[#orange,dotted]-> Jobs : enqueues\n\
Edge --> Web : routes\n\
@enduml\n";
    let svg = render_source_to_svg_for_family(deployment, DiagramFamily::Deployment)
        .expect("deployment should render");
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("stroke=\"#ffa500\""));
    assert!(svg.contains("Web"));
    assert!(svg.contains("Jobs"));
    assert!(svg.contains("Edge"));
}
