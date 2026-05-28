//! Coverage uplift tests for `src/theme/styles.rs` and `src/theme/values.rs`.
//!
//! These tests target the branches that were identified as uncovered in the wave-8
//! coverage report, pushing the total line coverage past the 87% gate.
//!
//! Refs #89

// Test code mutates `SequenceStyle::default()` in many small focused tests; rewriting
// each as a full struct-literal expansion (with every other field defaulted) is far
// noisier than the mutation pattern itself, and adds no readability or safety in test
// code. The lint is meaningful in production code where partial init can hide bugs.
#![allow(clippy::field_reassign_with_default)]

use puml::theme::{
    hex_color_is_dark, salt_style_from_sequence_theme, GroupHeaderFontStyle, MessageAlign,
    MonochromeMode, SequenceStyle, StyleColor, StyleSource, TextAlignment,
};
use puml::theme::{EffectiveStyleValue, MindMapDepthStyle, MindMapStyle, SaltStyle, Theme};

// ---------------------------------------------------------------------------
// Theme struct (styles.rs lines 5-29)
// ---------------------------------------------------------------------------

#[test]
fn theme_default_has_footbox_invisible() {
    let t = Theme::default();
    assert!(!t.footbox_visible, "default theme should hide footbox");
}

#[test]
fn theme_new_has_footbox_visible() {
    let t = Theme::new();
    assert!(t.footbox_visible, "Theme::new() should show footbox");
}

#[test]
fn theme_default_and_new_have_same_skinparams() {
    let d = Theme::default();
    let n = Theme::new();
    assert!(d.skinparams.is_empty());
    assert!(n.skinparams.is_empty());
}

// ---------------------------------------------------------------------------
// TextAlignment (styles.rs lines 127-143)
// ---------------------------------------------------------------------------

#[test]
fn text_alignment_as_text_anchor_center() {
    assert_eq!(TextAlignment::Center.as_text_anchor(), "middle");
}

#[test]
fn text_alignment_as_text_anchor_left() {
    assert_eq!(TextAlignment::Left.as_text_anchor(), "start");
}

#[test]
fn text_alignment_as_text_anchor_right() {
    assert_eq!(TextAlignment::Right.as_text_anchor(), "end");
}

#[test]
fn text_alignment_default_is_center() {
    assert_eq!(TextAlignment::default(), TextAlignment::Center);
}

// ---------------------------------------------------------------------------
// MonochromeMode (styles.rs lines 105-125)
// Note: ink() and paper() are pub(crate); cover the type via Debug/Clone/PartialEq.
// ---------------------------------------------------------------------------

#[test]
fn monochrome_mode_variants_are_distinct() {
    assert_ne!(MonochromeMode::True, MonochromeMode::Reverse);
}

#[test]
fn monochrome_mode_clone_preserves_variant() {
    let m = MonochromeMode::True;
    assert_eq!(m, m.clone());
    let r = MonochromeMode::Reverse;
    assert_eq!(r, r.clone());
}

#[test]
fn monochrome_mode_debug_contains_variant_name() {
    assert!(format!("{:?}", MonochromeMode::True).contains("True"));
    assert!(format!("{:?}", MonochromeMode::Reverse).contains("Reverse"));
}

// ---------------------------------------------------------------------------
// MessageAlign (styles.rs lines 88-94)
// ---------------------------------------------------------------------------

#[test]
fn message_align_default_is_left() {
    assert_eq!(MessageAlign::default(), MessageAlign::Left);
}

#[test]
fn message_align_variants_are_distinct() {
    assert_ne!(MessageAlign::Left, MessageAlign::Center);
    assert_ne!(MessageAlign::Left, MessageAlign::Right);
    assert_ne!(MessageAlign::Center, MessageAlign::Right);
}

// ---------------------------------------------------------------------------
// GroupHeaderFontStyle (styles.rs lines 97-103)
// ---------------------------------------------------------------------------

#[test]
fn group_header_font_style_default_is_normal() {
    assert_eq!(
        GroupHeaderFontStyle::default(),
        GroupHeaderFontStyle::Normal
    );
}

#[test]
fn group_header_font_style_variants_are_distinct() {
    assert_ne!(GroupHeaderFontStyle::Normal, GroupHeaderFontStyle::Bold);
    assert_ne!(GroupHeaderFontStyle::Normal, GroupHeaderFontStyle::Italic);
    assert_ne!(GroupHeaderFontStyle::Bold, GroupHeaderFontStyle::Italic);
}

// ---------------------------------------------------------------------------
// MindMapStyle and MindMapDepthStyle (styles.rs lines 145-155)
// ---------------------------------------------------------------------------

#[test]
fn mindmap_style_default_has_empty_depth_styles() {
    let s = MindMapStyle::default();
    assert!(s.depth_styles.is_empty());
}

#[test]
fn mindmap_depth_style_default_all_none() {
    let d = MindMapDepthStyle::default();
    assert!(d.background_color.is_none());
    assert!(d.font_color.is_none());
    assert!(d.border_color.is_none());
}

#[test]
fn mindmap_depth_style_equality() {
    let a = MindMapDepthStyle {
        background_color: Some("#ff0000".to_string()),
        font_color: None,
        border_color: None,
    };
    let b = MindMapDepthStyle {
        background_color: Some("#ff0000".to_string()),
        font_color: None,
        border_color: None,
    };
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// SequenceStyle defaults and participant_font_color_resolved (styles.rs 326-405)
// ---------------------------------------------------------------------------

#[test]
fn sequence_style_default_values_are_correct() {
    let s = SequenceStyle::default();
    assert_eq!(s.arrow_color, "#111");
    assert_eq!(s.lifeline_border_color, "#555");
    assert_eq!(s.participant_background_color, "#f6f6f6");
    assert_eq!(s.participant_border_color, "#111");
    assert!(s.participant_font_color.is_none());
    assert_eq!(s.note_background_color, "#fff8c4");
    assert_eq!(s.note_border_color, "#111");
    assert_eq!(s.group_background_color, "#fafafa");
    assert_eq!(s.group_border_color, "#666");
    assert_eq!(s.round_corner, 4);
    assert!(!s.shadowing);
    assert!(!s.hand_drawn);
    assert!(!s.lifeline_nosolid);
    assert!(!s.sepia);
    assert!(!s.response_message_below_arrow);
    assert!(!s.sequence_message_span);
    assert_eq!(s.message_align, MessageAlign::Left);
    assert_eq!(s.group_header_font_style, GroupHeaderFontStyle::Normal);
}

#[test]
fn participant_font_color_resolved_uses_explicit_color_when_set() {
    let mut s = SequenceStyle::default();
    s.participant_font_color = Some("#abcdef".to_string());
    assert_eq!(s.participant_font_color_resolved(), "#abcdef");
}

#[test]
fn participant_font_color_resolved_auto_detects_dark_background() {
    let mut s = SequenceStyle::default();
    s.participant_font_color = None;
    s.participant_background_color = "#000000".to_string(); // dark
    assert_eq!(s.participant_font_color_resolved(), "#ffffff");
}

#[test]
fn participant_font_color_resolved_auto_detects_light_background() {
    let mut s = SequenceStyle::default();
    s.participant_font_color = None;
    s.participant_background_color = "#f6f6f6".to_string(); // light (default)
    assert_eq!(s.participant_font_color_resolved(), "#111111");
}

// ---------------------------------------------------------------------------
// hex_color_is_dark (styles.rs 408-439)
// ---------------------------------------------------------------------------

#[test]
fn hex_color_is_dark_pure_black_6() {
    assert!(hex_color_is_dark("#000000"));
}

#[test]
fn hex_color_is_dark_pure_white_6() {
    assert!(!hex_color_is_dark("#ffffff"));
}

#[test]
fn hex_color_is_dark_pure_black_3() {
    assert!(hex_color_is_dark("#000"));
}

#[test]
fn hex_color_is_dark_pure_white_3() {
    assert!(!hex_color_is_dark("#fff"));
}

#[test]
fn hex_color_is_dark_navy_blue_is_dark() {
    assert!(hex_color_is_dark("#000080"));
}

#[test]
fn hex_color_is_dark_yellow_is_light() {
    assert!(!hex_color_is_dark("#ffff00"));
}

#[test]
fn hex_color_is_dark_midgray_is_light() {
    // #808080 luminance ≈ 0.216 which is > 0.179
    assert!(!hex_color_is_dark("#808080"));
}

#[test]
fn hex_color_is_dark_invalid_hex_char_returns_false() {
    // #ggg has invalid hex digits — should return false (not dark)
    assert!(!hex_color_is_dark("#ggg"));
}

#[test]
fn hex_color_is_dark_wrong_length_returns_false() {
    // lengths 1, 2, 4, 5, 7+ are all treated as unknown → false
    assert!(!hex_color_is_dark("#12"));
    assert!(!hex_color_is_dark("#1234"));
    assert!(!hex_color_is_dark("#1234567"));
}

#[test]
fn hex_color_is_dark_no_hash_prefix_parses_as_6_digit() {
    // Without '#', trim_start_matches('#') is a no-op; "000000" is 6 chars
    // and parses as r=0,g=0,b=0 which is dark. This is the actual behavior.
    assert!(hex_color_is_dark("000000"));
    // A string without '#' that isn't 3 or 6 chars returns false
    assert!(!hex_color_is_dark("red"));
}

#[test]
fn hex_color_is_dark_dark_red_is_dark() {
    assert!(hex_color_is_dark("#8b0000"));
}

#[test]
fn hex_color_is_dark_dark_shorthand_3() {
    // #222 → R=0x22, G=0x22, B=0x22 — very dark
    assert!(hex_color_is_dark("#222"));
}

#[test]
fn hex_color_is_dark_bright_shorthand_3() {
    // #eee → R=0xee, G=0xee, B=0xee — light
    assert!(!hex_color_is_dark("#eee"));
}

// ---------------------------------------------------------------------------
// salt_style_from_sequence_theme (styles.rs 362-390)
// ---------------------------------------------------------------------------

#[test]
fn salt_style_from_sequence_theme_derives_fields_correctly() {
    let seq = SequenceStyle::default();
    let salt = salt_style_from_sequence_theme(&seq);

    // canvas_fill: background_color is None, falls back to group_background_color
    assert_eq!(salt.canvas_fill, seq.group_background_color);
    assert_eq!(salt.panel_fill, seq.group_background_color);
    assert_eq!(salt.header_fill, seq.participant_background_color);
    assert_eq!(salt.input_fill, seq.note_background_color);
    assert_eq!(salt.button_fill, seq.participant_background_color);
    assert_eq!(salt.menu_fill, seq.group_background_color);
    assert_eq!(salt.tab_fill, seq.note_background_color);
    assert_eq!(salt.scroll_fill, seq.group_border_color);
    assert_eq!(salt.checkbox_fill, seq.note_background_color);
    assert_eq!(salt.radio_fill, seq.note_background_color);
    assert_eq!(salt.accent_fill, seq.note_background_color);
    assert_eq!(salt.border_color, seq.participant_border_color);
    assert_eq!(salt.grid_color, seq.group_border_color);
    assert_eq!(salt.text_color, seq.arrow_color);
    assert_eq!(salt.muted_text_color, seq.group_border_color);
    // No explicit font name → monospace default
    assert_eq!(salt.font_family, "monospace");
}

#[test]
fn salt_style_from_sequence_theme_uses_background_color_for_canvas_when_set() {
    let mut seq = SequenceStyle::default();
    seq.background_color = Some("#161616".to_string());
    let salt = salt_style_from_sequence_theme(&seq);
    assert_eq!(salt.canvas_fill, "#161616");
}

#[test]
fn salt_style_from_sequence_theme_uses_explicit_font_name() {
    let mut seq = SequenceStyle::default();
    seq.default_font_name = Some("Roboto".to_string());
    let salt = salt_style_from_sequence_theme(&seq);
    assert_eq!(salt.font_family, "Roboto");
}

#[test]
fn salt_style_from_sequence_theme_header_text_color_from_participant_font_color_resolved() {
    let mut seq = SequenceStyle::default();
    seq.participant_font_color = Some("#cc0000".to_string());
    let salt = salt_style_from_sequence_theme(&seq);
    assert_eq!(salt.header_text_color, "#cc0000");
    assert_eq!(salt.button_text_color, "#cc0000");
}

// ---------------------------------------------------------------------------
// SaltStyle::apply_key (styles.rs 240-323)
// ---------------------------------------------------------------------------

#[test]
fn salt_apply_key_background_color_aliases() {
    for key in ["backgroundcolor", "saltbackgroundcolor", "canvascolor"] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#aabbcc"));
        assert_eq!(s.canvas_fill, "#aabbcc");
    }
}

#[test]
fn salt_apply_key_panel_color_aliases() {
    for key in ["saltpanelcolor", "panelcolor", "saltfillcolor"] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#112233"));
        assert_eq!(s.panel_fill, "#112233");
    }
}

#[test]
fn salt_apply_key_header_color_aliases() {
    for key in ["saltheadercolor", "headercolor", "tableheadercolor"] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#334455"));
        assert_eq!(s.header_fill, "#334455");
    }
}

#[test]
fn salt_apply_key_input_color_aliases() {
    for key in [
        "saltinputcolor",
        "saltinputbackgroundcolor",
        "inputbackgroundcolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#556677"));
        assert_eq!(s.input_fill, "#556677");
    }
}

#[test]
fn salt_apply_key_button_color_aliases() {
    for key in [
        "saltbuttoncolor",
        "saltbuttonbackgroundcolor",
        "buttonbackgroundcolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#778899"));
        assert_eq!(s.button_fill, "#778899");
    }
}

#[test]
fn salt_apply_key_menu_color_aliases() {
    for key in [
        "saltmenucolor",
        "saltmenubackgroundcolor",
        "menubackgroundcolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#99aabb"));
        assert_eq!(s.menu_fill, "#99aabb");
    }
}

#[test]
fn salt_apply_key_tab_color_aliases() {
    for key in [
        "salttabcolor",
        "salttabbackgroundcolor",
        "tabbackgroundcolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#aabbcc"));
        assert_eq!(s.tab_fill, "#aabbcc");
    }
}

#[test]
fn salt_apply_key_scrollbar_color_aliases() {
    for key in [
        "saltscrollbarcolor",
        "scrollbarcolor",
        "scrollbarbackgroundcolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#bbccdd"));
        assert_eq!(s.scroll_fill, "#bbccdd");
    }
}

#[test]
fn salt_apply_key_checkbox_radio_accent() {
    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltcheckboxcolor", "#ccddee"));
    assert_eq!(s.checkbox_fill, "#ccddee");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("checkboxbackgroundcolor", "#ccddee"));
    assert_eq!(s.checkbox_fill, "#ccddee");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltradiocolor", "#ddeeff"));
    assert_eq!(s.radio_fill, "#ddeeff");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("radiobackgroundcolor", "#ddeeff"));
    assert_eq!(s.radio_fill, "#ddeeff");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltaccentcolor", "#eeffaa"));
    assert_eq!(s.accent_fill, "#eeffaa");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("accentcolor", "#eeffaa"));
    assert_eq!(s.accent_fill, "#eeffaa");
}

#[test]
fn salt_apply_key_border_and_line_color() {
    for key in [
        "bordercolor",
        "linecolor",
        "saltbordercolor",
        "saltlinecolor",
    ] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#010203"));
        assert_eq!(s.border_color, "#010203");
    }
}

#[test]
fn salt_apply_key_grid_color() {
    for key in ["saltgridcolor", "gridcolor"] {
        let mut s = SaltStyle::default();
        assert!(s.apply_key(key, "#030405"));
        assert_eq!(s.grid_color, "#030405");
    }
}

#[test]
fn salt_apply_key_font_colors() {
    let mut s = SaltStyle::default();
    assert!(s.apply_key("fontcolor", "#111111"));
    assert_eq!(s.text_color, "#111111");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltfontcolor", "#222222"));
    assert_eq!(s.text_color, "#222222");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltheaderfontcolor", "#333333"));
    assert_eq!(s.header_text_color, "#333333");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("headerfontcolor", "#444444"));
    assert_eq!(s.header_text_color, "#444444");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltinputfontcolor", "#555555"));
    assert_eq!(s.input_text_color, "#555555");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("inputfontcolor", "#666666"));
    assert_eq!(s.input_text_color, "#666666");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltbuttonfontcolor", "#777777"));
    assert_eq!(s.button_text_color, "#777777");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("buttonfontcolor", "#888888"));
    assert_eq!(s.button_text_color, "#888888");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("saltmutedfontcolor", "#999999"));
    assert_eq!(s.muted_text_color, "#999999");

    let mut s = SaltStyle::default();
    assert!(s.apply_key("mutedfontcolor", "#aaaaaa"));
    assert_eq!(s.muted_text_color, "#aaaaaa");
}

#[test]
fn salt_apply_key_handwritten_true_switches_font_family() {
    let mut s = SaltStyle::default();
    assert!(s.apply_key("handwritten", "true"));
    assert_eq!(s.font_family, "Comic Sans MS, cursive");
}

#[test]
fn salt_apply_key_handwritten_false_is_noop() {
    // "handwritten false" doesn't match the `"true"` arm, falls through to false
    let mut s = SaltStyle::default();
    let original_font = s.font_family.clone();
    assert!(!s.apply_key("handwritten", "false"));
    assert_eq!(s.font_family, original_font);
}

#[test]
fn salt_apply_key_unknown_key_returns_false() {
    let mut s = SaltStyle::default();
    assert!(!s.apply_key("unknownKey", "#ff0000"));
}

#[test]
fn salt_apply_key_invalid_color_returns_false() {
    let mut s = SaltStyle::default();
    // resolve_css3_color_or_original preserves unknown strings as-is but
    // returns Some, so only truly empty input drives a false return.
    // An empty string is the only guaranteed false path.
    assert!(!s.apply_key("backgroundcolor", ""));
}

// ---------------------------------------------------------------------------
// SaltStyle::apply_property (styles.rs 207-238) — scope-based dispatch
// ---------------------------------------------------------------------------

#[test]
fn salt_apply_property_no_scope_delegates_to_apply_key() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(None, "canvascolor", "#111111"));
    assert_eq!(s.canvas_fill, "#111111");
}

#[test]
fn salt_apply_property_saltdiagram_scope_delegates_to_apply_key() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("saltDiagram"), "backgroundcolor", "#222222"));
    assert_eq!(s.canvas_fill, "#222222");
}

#[test]
fn salt_apply_property_salt_scope_delegates_to_apply_key() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("salt"), "canvascolor", "#333333"));
    assert_eq!(s.canvas_fill, "#333333");
}

#[test]
fn salt_apply_property_button_backgroundcolor_maps_to_salt_key() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("button"), "backgroundcolor", "#444444"));
    assert_eq!(s.button_fill, "#444444");
}

#[test]
fn salt_apply_property_button_fontcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("button"), "fontcolor", "#550011"));
    assert_eq!(s.button_text_color, "#550011");
}

#[test]
fn salt_apply_property_input_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("input"), "backgroundcolor", "#660022"));
    assert_eq!(s.input_fill, "#660022");
}

#[test]
fn salt_apply_property_textfield_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("textfield"), "backgroundcolor", "#770033"));
    assert_eq!(s.input_fill, "#770033");
}

#[test]
fn salt_apply_property_textarea_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("textarea"), "backgroundcolor", "#880044"));
    assert_eq!(s.input_fill, "#880044");
}

#[test]
fn salt_apply_property_input_fontcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("input"), "fontcolor", "#990055"));
    assert_eq!(s.input_text_color, "#990055");
}

#[test]
fn salt_apply_property_header_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("header"), "backgroundcolor", "#aabb00"));
    assert_eq!(s.header_fill, "#aabb00");
}

#[test]
fn salt_apply_property_header_fontcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("header"), "fontcolor", "#bbcc00"));
    assert_eq!(s.header_text_color, "#bbcc00");
}

#[test]
fn salt_apply_property_menu_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("menu"), "backgroundcolor", "#ccdd00"));
    assert_eq!(s.menu_fill, "#ccdd00");
}

#[test]
fn salt_apply_property_tab_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("tab"), "backgroundcolor", "#ddee00"));
    assert_eq!(s.tab_fill, "#ddee00");
}

#[test]
fn salt_apply_property_scrollbar_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("scrollbar"), "backgroundcolor", "#eeff00"));
    assert_eq!(s.scroll_fill, "#eeff00");
}

#[test]
fn salt_apply_property_checkbox_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("checkbox"), "backgroundcolor", "#001122"));
    assert_eq!(s.checkbox_fill, "#001122");
}

#[test]
fn salt_apply_property_radio_backgroundcolor_maps() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("radio"), "backgroundcolor", "#002233"));
    assert_eq!(s.radio_fill, "#002233");
}

#[test]
fn salt_apply_property_unknown_scope_linecolor_maps_to_border() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("somewidget"), "linecolor", "#ff1234"));
    assert_eq!(s.border_color, "#ff1234");
}

#[test]
fn salt_apply_property_unknown_scope_bordercolor_maps_to_border() {
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("somewidget"), "bordercolor", "#ff5678"));
    assert_eq!(s.border_color, "#ff5678");
}

#[test]
fn salt_apply_property_unknown_scope_unknown_key_fallthrough_to_apply_key() {
    // Falls through to apply_key which returns false for unknown keys
    let mut s = SaltStyle::default();
    assert!(!s.apply_property(Some("somewidget"), "unknownProp", "#ffffff"));
}

#[test]
fn salt_apply_property_scope_with_braces_and_whitespace_is_trimmed() {
    // Scope can arrive as "{ button" from a style block context
    let mut s = SaltStyle::default();
    assert!(s.apply_property(Some("{ button"), "backgroundcolor", "#003344"));
    assert_eq!(s.button_fill, "#003344");
}

// ---------------------------------------------------------------------------
// values.rs — StyleSource, StyleColor, EffectiveStyleValue
// ---------------------------------------------------------------------------

#[test]
fn style_source_default_is_default_variant() {
    assert_eq!(StyleSource::default(), StyleSource::Default);
}

#[test]
fn style_source_all_variants_are_distinct() {
    let variants = [
        StyleSource::Default,
        StyleSource::ThemePreset,
        StyleSource::SkinParam,
        StyleSource::StyleBlock,
        StyleSource::Stereotype,
        StyleSource::Inline,
    ];
    for i in 0..variants.len() {
        for j in 0..variants.len() {
            if i == j {
                assert_eq!(variants[i], variants[j]);
            } else {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }
}

#[test]
fn style_color_parse_valid_hex() {
    let c = StyleColor::parse("#ff0000");
    assert!(c.is_some());
    assert_eq!(c.unwrap().as_str(), "#ff0000");
}

#[test]
fn style_color_parse_valid_named_color() {
    let c = StyleColor::parse("red");
    assert!(c.is_some());
    assert_eq!(c.unwrap().as_str(), "#ff0000");
}

#[test]
fn style_color_parse_invalid_returns_none() {
    let c = StyleColor::parse("not-a-color!!");
    assert!(c.is_none());
}

#[test]
fn style_color_trusted_wraps_value_as_is() {
    let c = StyleColor::trusted("anything-goes");
    assert_eq!(c.as_str(), "anything-goes");
}

#[test]
fn style_color_display_uses_inner_string() {
    let c = StyleColor::trusted("#abcdef");
    assert_eq!(format!("{}", c), "#abcdef");
}

#[test]
fn style_color_equality() {
    let a = StyleColor::trusted("#aabbcc");
    let b = StyleColor::trusted("#aabbcc");
    let c = StyleColor::trusted("#000000");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn effective_style_value_new_round_trips() {
    let v = EffectiveStyleValue::new(42u32, StyleSource::SkinParam);
    assert_eq!(*v.value(), 42u32);
    assert_eq!(v.source(), StyleSource::SkinParam);
}

#[test]
fn effective_style_value_color_shortcut() {
    let v = EffectiveStyleValue::color("#334455", StyleSource::StyleBlock);
    assert_eq!(v.as_str(), "#334455");
    assert_eq!(v.source(), StyleSource::StyleBlock);
}

#[test]
fn effective_style_value_all_sources_round_trip() {
    for src in [
        StyleSource::Default,
        StyleSource::ThemePreset,
        StyleSource::SkinParam,
        StyleSource::StyleBlock,
        StyleSource::Stereotype,
        StyleSource::Inline,
    ] {
        let v = EffectiveStyleValue::new("test", src);
        assert_eq!(v.source(), src);
    }
}
