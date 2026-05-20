use crate::common::*;

#[test]
fn theme_classifies_sequence_skinparam_subset() {
    assert_eq!(
        classify_sequence_skinparam("maxMessageSize", "120"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceFootbox", "hide"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::FootboxVisible(false))
    );
    assert_eq!(
        classify_sequence_skinparam("footbox", "show"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::FootboxVisible(true))
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceFootbox", "bogus"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    // "red" is now resolved to its CSS3 hex value by parse_color_value.
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "red"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "#AaBbCC"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "\"/><script"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("SequenceResponseMessageBelowArrow", "true"),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ResponseMessageBelowArrow(true)
        )
    );
    assert_eq!(
        classify_sequence_skinparam("MessageLineColor", "blue"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageLineColor(
            "#0000ff".to_string()
        ))
    );
}

#[test]
fn css3_color_names_are_resolved_to_hex_in_skinparams() {
    let src = fs::read_to_string(fixture("styling/valid_css3_color_message_arrow.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    // "rebeccapurple" -> "#663399"
    assert_eq!(model.style.arrow_color, "#663399");
    // "aliceblue" -> "#f0f8ff"
    assert_eq!(model.style.participant_background_color, "#f0f8ff");
    // "navy" -> "#000080"
    assert_eq!(model.style.participant_border_color, "#000080");
    assert!(model.warnings.is_empty());
}

#[test]
fn new_skinparams_round_shadow_font_background_alignment_are_accepted() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_round_shadow.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert_eq!(model.style.round_corner, 12);
    assert!(model.style.shadowing);
    assert_eq!(model.style.default_font_name.as_deref(), Some("Arial"));
    assert_eq!(model.style.default_font_size, Some(14));
    // "cornsilk" -> "#fff8dc"
    assert_eq!(model.style.background_color.as_deref(), Some("#fff8dc"));
    use puml::theme::TextAlignment;
    assert_eq!(model.style.text_alignment, TextAlignment::Left);
    assert!(model.warnings.is_empty());
}

#[test]
fn scale_directive_factor_is_parsed_and_stored() {
    let src = fs::read_to_string(fixture("styling/valid_scale_directive.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert!(
        matches!(model.scale, Some(ScaleSpec::Factor(f)) if (f - 1.5).abs() < 0.001),
        "expected scale factor 1.5, got {:?}",
        model.scale
    );
}

#[test]
fn scale_directive_fixed_size_is_parsed() {
    let src = "@startuml\nscale 800*600\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert_eq!(
        model.scale,
        Some(ScaleSpec::Fixed {
            width: 800,
            height: 600
        })
    );
}

#[test]
fn scale_directive_max_is_parsed() {
    let src = "@startuml\nscale max 500\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert_eq!(model.scale, Some(ScaleSpec::Max(500)));
}

#[test]
fn css3_color_to_hex_covers_full_set() {
    use puml::theme::css3_color_to_hex;

    // Check a representative sample of all CSS3 named colors.
    assert_eq!(css3_color_to_hex("rebeccapurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("RebeccaPurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("aliceblue"), Some("#f0f8ff"));
    assert_eq!(css3_color_to_hex("yellowgreen"), Some("#9acd32"));
    assert_eq!(css3_color_to_hex("midnightblue"), Some("#191970"));
    assert_eq!(css3_color_to_hex("notacolor"), None);
}

// ─── Tests: classify_*_skinparam for previously-missing families (#442) ───────

#[test]
fn theme_classifies_gantt_skinparam() {
    use puml::theme::{classify_gantt_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_gantt_skinparam("BackgroundColor", "red"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_gantt_skinparam("GanttFontColor", "#aabbcc"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_gantt_skinparam("FontSize", "14"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(14))
    );
    assert_eq!(
        classify_gantt_skinparam("TodayColor", "blue"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_gantt_skinparam("completelymadeupkey", "val"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_mindmap_skinparam() {
    use puml::theme::{classify_mindmap_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_mindmap_skinparam("BackgroundColor", "#123456"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#123456".to_string()
        ))
    );
    assert_eq!(
        classify_mindmap_skinparam("NodeFontColor", "green"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#008000".to_string()
        ))
    );
    assert_eq!(
        classify_mindmap_skinparam("RoundCorner", "10"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_mindmap_skinparam("unknownmindmapkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_wbs_skinparam() {
    use puml::theme::{classify_wbs_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_wbs_skinparam("BorderColor", "navy"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(
            "#000080".to_string()
        ))
    );
    assert_eq!(
        classify_wbs_skinparam("FontSize", "12"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(12))
    );
    assert_eq!(
        classify_wbs_skinparam("RoundCorner", "5"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_wbs_skinparam("unknownwbskey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_timeline_skinparam() {
    use puml::theme::{classify_timeline_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_timeline_skinparam("BackgroundColor", "white"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#ffffff".to_string()
        ))
    );
    assert_eq!(
        classify_timeline_skinparam("TimelineFontColor", "#010203"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#010203".to_string()
        ))
    );
    assert_eq!(
        classify_timeline_skinparam("ArrowColor", "black"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_timeline_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_nwdiag_skinparam() {
    use puml::theme::{classify_nwdiag_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_nwdiag_skinparam("FontColor", "red"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_nwdiag_skinparam("FontSize", "10"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(10))
    );
    assert_eq!(
        classify_nwdiag_skinparam("NetworkColor", "blue"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_nwdiag_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_archimate_skinparam() {
    use puml::theme::{classify_archimate_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_archimate_skinparam("BackgroundColor", "#aabbcc"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_archimate_skinparam("BorderColor", "teal"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(
            "#008080".to_string()
        ))
    );
    assert_eq!(
        classify_archimate_skinparam("ArchiMateStyle", "sketch"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_archimate_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_sdl_skinparam() {
    use puml::theme::{classify_sdl_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_sdl_skinparam("BackgroundColor", "#112233"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#112233".to_string()
        ))
    );
    assert_eq!(
        classify_sdl_skinparam("FontSize", "16"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(16))
    );
    assert_eq!(
        classify_sdl_skinparam("FontName", "Courier"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sdl_skinparam("inventedsdlkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_ditaa_skinparam() {
    use puml::theme::{classify_ditaa_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_ditaa_skinparam("BackgroundColor", "silver"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#c0c0c0".to_string()
        ))
    );
    assert_eq!(
        classify_ditaa_skinparam("FontColor", "#ff0000"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_ditaa_skinparam("Shadowing", "true"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_ditaa_skinparam("inventedditaakey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_salt_skinparam() {
    use puml::theme::{classify_salt_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_salt_skinparam("BackgroundColor", "ivory"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#fffff0".to_string()
        ))
    );
    assert_eq!(
        classify_salt_skinparam("FontSize", "11"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(11))
    );
    assert_eq!(
        classify_salt_skinparam("RoundCorner", "4"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_salt_skinparam("inventedsaltkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}
