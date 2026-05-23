use puml::sprites::{
    builtin_sprite, encode_pixels, normalize_sprite_name, parse_hex_grid_sprite,
    parse_packed_sprite, parse_sprite_header_spec, parse_sprite_ref_at, parse_svg_sprite,
    render_sprite, SpriteKind, SpriteRef,
};
use puml::theme::{
    activity_style_from_sequence_theme, apply_monochrome_to_activity_style,
    apply_monochrome_to_chart_style, apply_monochrome_to_class_style,
    apply_monochrome_to_component_style, apply_monochrome_to_sequence_style,
    apply_monochrome_to_state_style, apply_monochrome_to_timing_style,
    chart_style_from_sequence_theme, class_style_from_sequence_theme, classify_activity_skinparam,
    classify_archimate_skinparam, classify_chart_skinparam, classify_class_skinparam,
    classify_component_skinparam, classify_ditaa_skinparam, classify_gantt_skinparam,
    classify_mindmap_skinparam, classify_nwdiag_skinparam, classify_salt_skinparam,
    classify_sdl_skinparam, classify_sequence_skinparam, classify_state_skinparam,
    classify_timeline_skinparam, classify_timing_skinparam, classify_wbs_skinparam,
    component_style_from_sequence_theme, css3_color_to_hex, hex_color_is_dark,
    mindmap_style_from_sequence_theme, resolve_sequence_theme_preset,
    state_style_from_sequence_theme, timing_style_from_sequence_theme, ActivitySkinParamValue,
    ActorStyle, ChartSkinParamValue, ClassSkinParamValue, ComponentSkinParamValue, ComponentStyle,
    ComponentStyleMode, GenericSkinParamValue, GroupHeaderFontStyle, MessageAlign, MonochromeMode,
    SequenceSkinParamSupport, SequenceSkinParamValue, SequenceStyle, SkinParamSupport,
    StateSkinParamValue, TextAlignment, TimingSkinParamValue, LOCAL_SEQUENCE_THEME_CATALOG,
};
use puml::{
    normalize_family, parse, render_source_to_svg_for_family, DiagramFamily, NormalizedDocument,
};

#[test]
fn theme_preset_projection_helpers_are_deterministic_across_family_styles() {
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        let mixed_case = format!("  {}  ", name.to_ascii_uppercase());
        let preset = resolve_sequence_theme_preset(&mixed_case)
            .unwrap_or_else(|err| panic!("theme {name} should resolve: {err}"));
        assert_eq!(preset.name, *name);
    }

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
fn monochrome_and_color_resolution_helpers_cover_true_reverse_and_invalid_paths() {
    assert_eq!(css3_color_to_hex("RebeccaPurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("darkgrey"), Some("#a9a9a9"));
    assert_eq!(css3_color_to_hex("not-a-css-color"), None);
    assert!(hex_color_is_dark("#000"));
    assert!(hex_color_is_dark("#112233"));
    assert!(!hex_color_is_dark("#fff"));
    assert!(!hex_color_is_dark("#ggg"));
    assert!(!hex_color_is_dark("not-hex"));

    let mut sequence = SequenceStyle {
        shadowing: true,
        ..SequenceStyle::default()
    };
    apply_monochrome_to_sequence_style(&mut sequence, MonochromeMode::Reverse);
    assert_eq!(sequence.arrow_color, "#ffffff");
    assert_eq!(sequence.background_color.as_deref(), Some("#000000"));
    assert_eq!(sequence.message_line_color.as_deref(), Some("#ffffff"));
    assert!(!sequence.shadowing);
    assert_eq!(sequence.participant_font_color_resolved(), "#ffffff");

    let mut class_style = class_style_from_sequence_theme(&sequence);
    class_style
        .stereotype_styles
        .insert("entity".to_string(), Default::default());
    apply_monochrome_to_class_style(&mut class_style, MonochromeMode::True);
    assert_eq!(class_style.background_color, "#ffffff");
    assert_eq!(class_style.arrow_color, "#000000");
    assert!(class_style.stereotype_styles.is_empty());

    let mut state = state_style_from_sequence_theme(&sequence);
    apply_monochrome_to_state_style(&mut state, MonochromeMode::Reverse);
    assert_eq!(state.background_color, "#000000");
    assert_eq!(state.start_color, "#ffffff");

    let mut component = ComponentStyle::default();
    apply_monochrome_to_component_style(&mut component, MonochromeMode::True);
    assert_eq!(component.interface_color, "#ffffff");
    assert_eq!(component.font_color, "#000000");

    let mut activity = activity_style_from_sequence_theme(&sequence);
    apply_monochrome_to_activity_style(&mut activity, MonochromeMode::Reverse);
    assert_eq!(activity.diamond_color, "#000000");
    assert_eq!(activity.arrow_color, "#ffffff");

    let mut timing = timing_style_from_sequence_theme(&sequence);
    apply_monochrome_to_timing_style(&mut timing, MonochromeMode::True);
    assert_eq!(timing.grid_color, "#000000");
    assert_eq!(timing.signal_background_color, "#ffffff");

    let mut chart = chart_style_from_sequence_theme(&sequence);
    apply_monochrome_to_chart_style(&mut chart, MonochromeMode::Reverse);
    assert_eq!(chart.background_color, "#000000");
    assert_eq!(chart.series_color, "#ffffff");
}

#[test]
fn skinparam_classifiers_cover_family_value_noop_and_error_branches() {
    assert_eq!(TextAlignment::Center.as_text_anchor(), "middle");
    assert_eq!(TextAlignment::Left.as_text_anchor(), "start");
    assert_eq!(TextAlignment::Right.as_text_anchor(), "end");
    assert!(resolve_sequence_theme_preset("plain extra")
        .expect_err("extra tokens should reject local theme names")
        .contains("E_THEME_INVALID"));

    assert_eq!(
        classify_sequence_skinparam("sequenceMessageAlign", "reverse-direction"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
            MessageAlign::Right
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceReferenceAlign", "direction"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceGroupHeaderFontStyle", "italic"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::GroupHeaderFontStyle(
            GroupHeaderFontStyle::Italic
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("defaultTextAlignment", "left"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::DefaultTextAlignment(
            TextAlignment::Left
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("lifelineStrategy", "solid"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::LifelineNoSolid(
            false
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("arrowColor", "#abcd"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "#abcd".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("arrowColor", "#xyz"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("roundCorner", "nope"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("shadowing", "maybe"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("monochrome", "off"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("monochrome", "sepia"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("defaultFontName", ""),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("defaultFontSize", "large"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("defaultTextAlignment", "right"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::DefaultTextAlignment(
            TextAlignment::Right
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("defaultTextAlignment", "diagonal"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("participantPadding", "wide"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("boxPadding", "wide"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceMessageAlign", "diagonal"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceReferenceAlign", "diagonal"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceResponseMessageBelowArrow", "no"),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ResponseMessageBelowArrow(false)
        )
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceResponseMessageBelowArrow", "maybe"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceLifelineThickness", "thick"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceGroupHeaderFontStyle", "oblique"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("lifelineStrategy", "dotted"),
        SequenceSkinParamSupport::UnsupportedValue
    );

    assert_eq!(
        classify_class_skinparam("classBackgroundColor<<Entity>>", "aliceblue"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeBackgroundColor(
            "entity".to_string(),
            "#f0f8ff".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("actorStyle", "awesome"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(ActorStyle::Awesome))
    );
    assert_eq!(
        classify_class_skinparam("actorStyle", "hollow"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(ActorStyle::Hollow))
    );
    assert_eq!(
        classify_class_skinparam("shadowing", "yes"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("fontName", ""),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("classFontSize", "big"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("fontName", "Fira Code"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontName(
            "Fira Code".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("monochrome", "reverse"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Monochrome(
            MonochromeMode::Reverse
        ))
    );
    assert_eq!(
        classify_class_skinparam("monochrome", "off"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("monochrome", "sepia"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("handwritten", "maybe"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("shadowing<<Entity>>", "maybe"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("classFontSize<<Entity>>", "12"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("unknown<<Entity>>", "red"),
        SkinParamSupport::UnsupportedKey
    );

    assert_eq!(
        classify_state_skinparam("stateStartColor", "navy"),
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::StartColor(
            "#000080".to_string()
        ))
    );
    assert_eq!(
        classify_state_skinparam("stateFontSize", "large"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_state_skinparam("stateFontName", "Fira Code"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_component_skinparam("componentStyle", "rectangle"),
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::StyleMode(
            ComponentStyleMode::Rectangle
        ))
    );
    assert_eq!(
        classify_component_skinparam("componentStyle", "uml1"),
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::StyleMode(
            ComponentStyleMode::Uml1
        ))
    );
    assert_eq!(
        classify_component_skinparam("componentStyle", "cloud"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_component_skinparam("componentFontName", "Fira Code"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_component_skinparam("interfaceColor", "bad color"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_activity_skinparam("activityBarColor", "#12345678"),
        SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BarColor(
            "#12345678".to_string()
        ))
    );
    assert_eq!(
        classify_activity_skinparam("activityFontName", "Fira Code"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_activity_skinparam("activityArrowColor", "bad color"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_timing_skinparam("timingGridColor", "lightgrey"),
        SkinParamSupport::SupportedWithValue(TimingSkinParamValue::GridColor(
            "#d3d3d3".to_string()
        ))
    );
    assert_eq!(
        classify_timing_skinparam("timingFontName", "Fira Code"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_chart_skinparam("chartPieBorderColor", "orange"),
        SkinParamSupport::SupportedWithValue(ChartSkinParamValue::PieBorderColor(
            "#ffa500".to_string()
        ))
    );
    assert_eq!(
        classify_chart_skinparam("legendFontSize", "14"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_gantt_skinparam("ganttFontSize", "18"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(18))
    );
    assert_eq!(
        classify_gantt_skinparam("todayColor", "ignored"),
        SkinParamSupport::SupportedNoop
    );
}

#[test]
fn generic_family_skinparam_classifiers_cover_value_error_and_noop_branches() {
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
            classify("fontSize", "large"),
            SkinParamSupport::UnsupportedValue
        );
        assert_eq!(
            classify(&format!("{prefix}BackgroundColor"), "#nothex"),
            SkinParamSupport::UnsupportedValue
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

#[test]
fn sprite_helpers_round_trip_uncompressed_and_compressed_pixels() {
    assert_eq!(normalize_sprite_name("  \"$icon\"  "), "$icon");
    assert_eq!(parse_sprite_header_spec("[2x3/4]"), Some((2, 3, 4, false)));
    assert_eq!(parse_sprite_header_spec("[2X2/8z]"), Some((2, 2, 8, true)));
    assert_eq!(parse_sprite_header_spec("[2x2/5]"), None);
    assert_eq!(parse_sprite_header_spec("2x2/4"), None);

    let pixels = [0, 5, 10, 15, 2, 7, 12, 15];
    for (levels, height) in [(16, 4), (8, 4), (4, 4)] {
        let encoded =
            encode_pixels("$roundtrip", 2, height, levels, false, &pixels).expect("encode");
        assert!(encoded.contains(&format!("[2x{height}/{levels}]")));
        let body = encoded
            .split_once("{\n")
            .and_then(|(_, rest)| rest.strip_suffix("\n}"))
            .expect("uncompressed sprite body");
        let decoded =
            parse_packed_sprite("$roundtrip", 2, height, levels, false, body).expect("decode");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, height);
        let SpriteKind::Monochrome { pixels: decoded } = decoded.kind else {
            panic!("expected monochrome pixels");
        };
        assert_eq!(decoded.len(), (2 * height) as usize);
        assert!(decoded.iter().any(|value| *value > 0));
    }

    let compressed = encode_pixels("$zip", 2, 4, 8, true, &pixels).expect("compressed encode");
    assert!(compressed.starts_with("sprite $zip [2x4/8z] "));
    let payload = compressed
        .strip_prefix("sprite $zip [2x4/8z] ")
        .expect("compressed payload");
    let decoded = parse_packed_sprite("$zip", 2, 4, 8, true, payload).expect("compressed decode");
    let SpriteKind::Monochrome { pixels: decoded } = decoded.kind else {
        panic!("expected compressed monochrome pixels");
    };
    assert_eq!(decoded.len(), 8);
    assert!(decoded.iter().all(|value| *value < 8));

    let mismatch = encode_pixels("$bad", 2, 2, 16, false, &[1, 2, 3])
        .expect_err("mismatched dimensions should fail");
    assert!(mismatch.message.contains("E_SPRITE_ENCODE_INVALID"));
}

#[test]
fn sprite_parsers_and_rendering_cover_svg_escape_and_error_branches() {
    let grid = parse_hex_grid_sprite(
        "$grid",
        Some(3),
        Some(2),
        4,
        &["0F7".to_string(), "123".to_string()],
    )
    .expect("hex grid should parse");
    let SpriteKind::Monochrome { pixels } = &grid.kind else {
        panic!("expected monochrome grid");
    };
    assert_eq!(pixels.len(), 6);
    assert!(pixels.iter().all(|value| *value < 4));

    assert!(parse_hex_grid_sprite("$empty", None, None, 16, &[])
        .expect_err("empty grid should fail")
        .message
        .contains("at least one row"));
    assert!(
        parse_hex_grid_sprite("$short", Some(4), Some(1), 16, &["123".to_string()])
            .expect_err("bad width should fail")
            .message
            .contains("row width")
    );
    assert!(
        parse_hex_grid_sprite("$bad", Some(1), Some(1), 16, &["x".to_string()])
            .expect_err("bad hex should fail")
            .message
            .contains("non-hex")
    );

    let svg = parse_svg_sprite(
        "$svg&icon",
        "<svg viewBox=\"0 0 7.2 3.1\"><path d=\"M0 0h7\"/></svg>",
    )
    .expect("viewBox dimensions should parse");
    assert_eq!(svg.width, 8);
    assert_eq!(svg.height, 4);
    let rendered_svg = render_sprite(
        &svg,
        1.25,
        2.5,
        &SpriteRef {
            name: "ignored".to_string(),
            scale: 1.5,
            color: None,
        },
    );
    assert!(rendered_svg.contains("data-sprite=\"svg&amp;icon\""));
    assert!(rendered_svg.contains("translate(1.25,2.50) scale(1.500)"));

    let mono = builtin_sprite("$<&seed", "stable");
    let rendered_mono = render_sprite(
        &mono,
        0.0,
        0.0,
        &SpriteRef {
            name: mono.name.clone(),
            scale: 0.1,
            color: Some("\"<&>".to_string()),
        },
    );
    assert!(rendered_mono.contains("data-sprite=\"&lt;&amp;seed\""));
    assert!(rendered_mono.contains("fill=\"&quot;&lt;&amp;&gt;\""));
    assert!(rendered_mono.contains("data-sprite-gray-levels=\"16\""));

    let (star_ref, consumed) = parse_sprite_ref_at("<$icon*99> tail").expect("star ref");
    assert_eq!(consumed, 10);
    assert_eq!(star_ref.name, "icon");
    assert_eq!(star_ref.scale, 32.0);
    let (plain_ref, consumed) = parse_sprite_ref_at("<$plain>").expect("plain ref");
    assert_eq!(consumed, 8);
    assert_eq!(plain_ref.scale, 1.0);
    let (fraction_ref, _) = parse_sprite_ref_at("<$icon,2.5>").expect("fraction ref");
    assert_eq!(fraction_ref.scale, 2.5);
    let (invalid_star_ref, _) = parse_sprite_ref_at("<$icon*nan>").expect("invalid star ref");
    assert_eq!(invalid_star_ref.scale, 1.0);
    let (brace_ref, _) =
        parse_sprite_ref_at("<$icon{scale=0.001, colour=navy, ignored=yes}>").expect("brace ref");
    assert_eq!(brace_ref.scale, 0.05);
    assert_eq!(brace_ref.color.as_deref(), Some("navy"));
    assert!(parse_sprite_ref_at("<$>").is_none());
    assert!(parse_sprite_ref_at("<$   >").is_none());

    assert!(
        parse_hex_grid_sprite("$zero", Some(0), Some(1), 16, &["".to_string()])
            .expect_err("zero width should fail")
            .message
            .contains("E_SPRITE_INVALID")
    );
    assert!(
        parse_hex_grid_sprite("$height", Some(1), Some(2), 16, &["0".to_string()])
            .expect_err("height mismatch should fail")
            .message
            .contains("height")
    );
    assert!(parse_packed_sprite("$zero", 0, 1, 16, false, "")
        .expect_err("zero packed width should fail")
        .message
        .contains("E_SPRITE_INVALID"));
    assert!(parse_packed_sprite("$short", 2, 3, 4, false, "0")
        .expect_err("short payload should fail")
        .message
        .contains("E_SPRITE_INVALID"));
    assert!(parse_packed_sprite("$bad", 1, 1, 4, false, "!")
        .expect_err("bad six-bit payload should fail")
        .message
        .contains("E_SPRITE_INVALID"));
    assert!(parse_packed_sprite("$badz", 1, 1, 8, true, "!!!!")
        .expect_err("bad compressed payload should fail")
        .message
        .contains("E_SPRITE_INVALID"));
    assert!(parse_svg_sprite("", "<svg/>")
        .expect_err("empty svg sprite name should fail")
        .message
        .contains("E_SPRITE_INVALID"));
}
