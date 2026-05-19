/// Coverage wave 23 — exercises theme skinparam classification functions
/// that were previously uncovered: class, state, component, activity,
/// timing, and chart classifiers.
use puml::theme::{
    classify_activity_skinparam, classify_chart_skinparam, classify_class_skinparam,
    classify_component_skinparam, classify_state_skinparam, classify_timing_skinparam,
    ActivitySkinParamValue, ChartSkinParamValue, ClassSkinParamValue, ComponentSkinParamValue,
    SkinParamSupport, StateSkinParamValue, TimingSkinParamValue,
};

// ── class skinparam ───────────────────────────────────────────────────────────

#[test]
fn class_background_color_valid() {
    let result = classify_class_skinparam("BackgroundColor", "#aabbcc");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BackgroundColor(_))
    ));
}

#[test]
fn class_background_color_aliases() {
    // classbackgroundcolor, objectbackgroundcolor, etc.
    for key in &[
        "classbackgroundcolor",
        "objectbackgroundcolor",
        "usecasebackgroundcolor",
        "actorbackgroundcolor",
    ] {
        let result = classify_class_skinparam(key, "#001122");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BackgroundColor(_))
            ),
            "expected BackgroundColor for key {key}"
        );
    }
}

#[test]
fn class_border_color_valid() {
    let result = classify_class_skinparam("BorderColor", "#112233");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BorderColor(_))
    ));
}

#[test]
fn class_border_color_aliases() {
    for key in &[
        "classbordercolor",
        "objectbordercolor",
        "usecasebordercolor",
        "actorbordercolor",
    ] {
        let result = classify_class_skinparam(key, "red");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BorderColor(_))
            ),
            "expected BorderColor for key {key}"
        );
    }
}

#[test]
fn class_header_background_color() {
    let result = classify_class_skinparam("ClassHeaderBackgroundColor", "#ffffff");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::HeaderBackgroundColor(_))
    ));
}

#[test]
fn class_member_font_color() {
    for key in &[
        "classmemberfontcolor",
        "classattributefontcolor",
        "classmethodfontcolor",
    ] {
        let result = classify_class_skinparam(key, "#333333");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::MemberFontColor(_))
            ),
            "expected MemberFontColor for key {key}"
        );
    }
}

#[test]
fn class_font_color() {
    let result = classify_class_skinparam("FontColor", "blue");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontColor(_))
    ));
}

#[test]
fn class_arrow_color() {
    let result = classify_class_skinparam("ArrowColor", "green");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ArrowColor(_))
    ));
}

#[test]
fn class_font_size_valid_and_invalid() {
    let ok = classify_class_skinparam("FontSize", "14");
    assert!(matches!(
        ok,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontSize(14))
    ));

    let bad = classify_class_skinparam("FontSize", "notanumber");
    assert!(matches!(bad, SkinParamSupport::UnsupportedValue));
}

#[test]
fn class_font_name() {
    let result = classify_class_skinparam("ClassFontName", "Arial");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontName(_))
    ));
    // Empty font name → UnsupportedValue
    let empty = classify_class_skinparam("ClassFontName", "");
    assert!(matches!(empty, SkinParamSupport::UnsupportedValue));
}

#[test]
fn class_invalid_color_returns_unsupported_value() {
    let result = classify_class_skinparam("BackgroundColor", "not-a-color");
    assert!(matches!(result, SkinParamSupport::UnsupportedValue));
}

#[test]
fn class_unknown_key() {
    let result = classify_class_skinparam("completelyunknownkey", "value");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

// ── state skinparam ───────────────────────────────────────────────────────────

#[test]
fn state_background_color() {
    let result = classify_state_skinparam("BackgroundColor", "#aabbcc");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::BackgroundColor(_))
    ));
    // Alias
    let aliased = classify_state_skinparam("StateBackgroundColor", "#aabbcc");
    assert!(matches!(
        aliased,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::BackgroundColor(_))
    ));
}

#[test]
fn state_border_color() {
    let result = classify_state_skinparam("BorderColor", "#123456");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::BorderColor(_))
    ));
}

#[test]
fn state_arrow_color() {
    let result = classify_state_skinparam("ArrowColor", "orange");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::ArrowColor(_))
    ));
}

#[test]
fn state_start_color() {
    let result = classify_state_skinparam("StateStartColor", "#000000");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::StartColor(_))
    ));
}

#[test]
fn state_font_color() {
    let result = classify_state_skinparam("FontColor", "black");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontColor(_))
    ));
}

#[test]
fn state_font_size() {
    let ok = classify_state_skinparam("FontSize", "12");
    assert!(matches!(
        ok,
        SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontSize(12))
    ));

    let bad = classify_state_skinparam("FontSize", "abc");
    assert!(matches!(bad, SkinParamSupport::UnsupportedValue));
}

#[test]
fn state_noop_keys() {
    for key in &[
        "statefontname",
        "statestereotypefontcolor",
        "stateattributefontsize",
    ] {
        let result = classify_state_skinparam(key, "anything");
        assert!(
            matches!(result, SkinParamSupport::SupportedNoop),
            "expected Noop for {key}"
        );
    }
}

#[test]
fn state_unknown_key() {
    let result = classify_state_skinparam("nonexistentkey", "val");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

// ── component skinparam ───────────────────────────────────────────────────────

#[test]
fn component_background_color_and_aliases() {
    for key in &[
        "BackgroundColor",
        "ComponentBackgroundColor",
        "DeploymentBackgroundColor",
        "NodeBackgroundColor",
        "ArtifactBackgroundColor",
        "DatabaseBackgroundColor",
    ] {
        let result = classify_component_skinparam(key, "#aaaaaa");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BackgroundColor(_))
            ),
            "expected BackgroundColor for {key}"
        );
    }
}

#[test]
fn component_border_color_and_aliases() {
    for key in &[
        "BorderColor",
        "ComponentBorderColor",
        "NodeBorderColor",
        "ArtifactBorderColor",
        "DatabaseBorderColor",
    ] {
        let result = classify_component_skinparam(key, "blue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BorderColor(_))
            ),
            "expected BorderColor for {key}"
        );
    }
}

#[test]
fn component_interface_color() {
    for key in &[
        "InterfaceBackgroundColor",
        "InterfaceColor",
        "InterfaceCircleBackgroundColor",
    ] {
        let result = classify_component_skinparam(key, "white");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::InterfaceColor(_))
            ),
            "expected InterfaceColor for {key}"
        );
    }
}

#[test]
fn component_port_color() {
    let result = classify_component_skinparam("PortBackgroundColor", "green");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::InterfaceColor(_))
    ));
}

#[test]
fn component_font_color_and_aliases() {
    for key in &[
        "FontColor",
        "ComponentFontColor",
        "DeploymentFontColor",
        "NodeFontColor",
        "ArtifactFontColor",
        "DatabaseFontColor",
        "PortFontColor",
        "InterfaceFontColor",
    ] {
        let result = classify_component_skinparam(key, "black");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::FontColor(_))
            ),
            "expected FontColor for {key}"
        );
    }
}

#[test]
fn component_arrow_color() {
    let result = classify_component_skinparam("ArrowColor", "red");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::ArrowColor(_))
    ));
}

#[test]
fn component_noop_keys() {
    for key in &[
        "ComponentFontSize",
        "DeploymentFontName",
        "PackageStyle",
        "PackageBorderColor",
        "PackageBackgroundColor",
    ] {
        let result = classify_component_skinparam(key, "anything");
        assert!(
            matches!(result, SkinParamSupport::SupportedNoop),
            "expected Noop for {key}"
        );
    }
}

#[test]
fn component_unknown_key() {
    let result = classify_component_skinparam("weirdunknownkey", "val");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

// ── activity skinparam ────────────────────────────────────────────────────────

#[test]
fn activity_background_color_and_aliases() {
    for key in &[
        "BackgroundColor",
        "ActivityBackgroundColor",
        "ActivityPartitionBackgroundColor",
    ] {
        let result = classify_activity_skinparam(key, "#eefff0");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BackgroundColor(_))
            ),
            "expected BackgroundColor for {key}"
        );
    }
}

#[test]
fn activity_border_color_and_aliases() {
    for key in &[
        "BorderColor",
        "ActivityBorderColor",
        "ActivityPartitionBorderColor",
        "SwimlaneBorderColor",
    ] {
        let result = classify_activity_skinparam(key, "navy");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BorderColor(_))
            ),
            "expected BorderColor for {key}"
        );
    }
}

#[test]
fn activity_diamond_background_color() {
    for key in &["ActivityDiamondBackgroundColor", "ActivityDiamondColor"] {
        let result = classify_activity_skinparam(key, "yellow");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(
                    ActivitySkinParamValue::DiamondBackgroundColor(_)
                )
            ),
            "expected DiamondBackgroundColor for {key}"
        );
    }
}

#[test]
fn activity_bar_color() {
    for key in &["ActivityBarColor", "ActivityStartColor", "ActivityEndColor"] {
        let result = classify_activity_skinparam(key, "black");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BarColor(_))
            ),
            "expected BarColor for {key}"
        );
    }
}

#[test]
fn activity_font_color() {
    for key in &["FontColor", "ActivityFontColor", "SwimlaneFontColor"] {
        let result = classify_activity_skinparam(key, "darkblue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::FontColor(_))
            ),
            "expected FontColor for {key}"
        );
    }
}

#[test]
fn activity_arrow_color() {
    for key in &["ArrowColor", "ActivityArrowColor"] {
        let result = classify_activity_skinparam(key, "gray");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::ArrowColor(_))
            ),
            "expected ArrowColor for {key}"
        );
    }
}

#[test]
fn activity_noop_keys() {
    for key in &[
        "ActivityFontSize",
        "ActivityFontName",
        "ActivityBorderThickness",
        "SwimlaneFontSize",
    ] {
        let result = classify_activity_skinparam(key, "val");
        assert!(
            matches!(result, SkinParamSupport::SupportedNoop),
            "expected Noop for {key}"
        );
    }
}

#[test]
fn activity_unknown_key() {
    let result = classify_activity_skinparam("unknownunknown", "val");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

// ── timing skinparam ──────────────────────────────────────────────────────────

#[test]
fn timing_background_color_and_aliases() {
    for key in &[
        "BackgroundColor",
        "TimingBackgroundColor",
        "TimingDiagramBackgroundColor",
    ] {
        let result = classify_timing_skinparam(key, "#ffffff");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::BackgroundColor(_))
            ),
            "expected BackgroundColor for {key}"
        );
    }
}

#[test]
fn timing_axis_color() {
    let result = classify_timing_skinparam("TimingAxisColor", "gray");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(TimingSkinParamValue::AxisColor(_))
    ));
}

#[test]
fn timing_grid_color() {
    let result = classify_timing_skinparam("TimingGridColor", "lightgray");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(TimingSkinParamValue::GridColor(_))
    ));
}

#[test]
fn timing_signal_background_color() {
    for key in &[
        "TimingSignalBackgroundColor",
        "TimingParticipantBackgroundColor",
    ] {
        let result = classify_timing_skinparam(key, "white");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::SignalBackgroundColor(
                    _
                ))
            ),
            "expected SignalBackgroundColor for {key}"
        );
    }
}

#[test]
fn timing_signal_border_color() {
    let result = classify_timing_skinparam("TimingSignalBorderColor", "black");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(TimingSkinParamValue::SignalBorderColor(_))
    ));
}

#[test]
fn timing_arrow_color() {
    for key in &["ArrowColor", "TimingArrowColor"] {
        let result = classify_timing_skinparam(key, "blue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::ArrowColor(_))
            ),
            "expected ArrowColor for {key}"
        );
    }
}

#[test]
fn timing_font_color() {
    for key in &["FontColor", "TimingFontColor"] {
        let result = classify_timing_skinparam(key, "darkblue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::FontColor(_))
            ),
            "expected FontColor for {key}"
        );
    }
}

#[test]
fn timing_noop_keys() {
    for key in &["TimingFontSize", "TimingFontName"] {
        let result = classify_timing_skinparam(key, "val");
        assert!(
            matches!(result, SkinParamSupport::SupportedNoop),
            "expected Noop for {key}"
        );
    }
}

#[test]
fn timing_unknown_key() {
    let result = classify_timing_skinparam("notreal", "val");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

// ── chart skinparam ───────────────────────────────────────────────────────────

#[test]
fn chart_background_color() {
    let result = classify_chart_skinparam("BackgroundColor", "#ffffff");
    assert!(matches!(
        result,
        SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BackgroundColor(_))
    ));
}

#[test]
fn chart_axis_color() {
    for key in &["AxisColor", "ChartAxisColor", "ChartAxisLineColor"] {
        let result = classify_chart_skinparam(key, "black");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::AxisColor(_))
            ),
            "expected AxisColor for {key}"
        );
    }
}

#[test]
fn chart_grid_color() {
    for key in &["GridColor", "ChartGridColor", "ChartGridLineColor"] {
        let result = classify_chart_skinparam(key, "lightgray");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::GridColor(_))
            ),
            "expected GridColor for {key}"
        );
    }
}

#[test]
fn chart_series_color() {
    for key in &["ChartSeriesColor", "SeriesColor"] {
        let result = classify_chart_skinparam(key, "blue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::SeriesColor(_))
            ),
            "expected SeriesColor for {key}"
        );
    }
}

#[test]
fn chart_bar_color() {
    for key in &["ChartBarColor", "BarColor"] {
        let result = classify_chart_skinparam(key, "green");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BarColor(_))
            ),
            "expected BarColor for {key}"
        );
    }
}

#[test]
fn chart_line_color() {
    for key in &["ChartLineColor", "LineColor"] {
        let result = classify_chart_skinparam(key, "red");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::LineColor(_))
            ),
            "expected LineColor for {key}"
        );
    }
}

#[test]
fn chart_pie_border_color() {
    for key in &["ChartPieBorderColor", "PieBorderColor"] {
        let result = classify_chart_skinparam(key, "black");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::PieBorderColor(_))
            ),
            "expected PieBorderColor for {key}"
        );
    }
}

#[test]
fn chart_font_color() {
    for key in &["FontColor", "ChartFontColor", "ChartLabelFontColor"] {
        let result = classify_chart_skinparam(key, "darkblue");
        assert!(
            matches!(
                result,
                SkinParamSupport::SupportedWithValue(ChartSkinParamValue::FontColor(_))
            ),
            "expected FontColor for {key}"
        );
    }
}

#[test]
fn chart_noop_keys() {
    for key in &[
        "ChartFontSize",
        "ChartFontName",
        "LegendFontColor",
        "LegendFontSize",
    ] {
        let result = classify_chart_skinparam(key, "val");
        assert!(
            matches!(result, SkinParamSupport::SupportedNoop),
            "expected Noop for {key}"
        );
    }
}

#[test]
fn chart_unknown_key() {
    let result = classify_chart_skinparam("totallyunknown", "val");
    assert!(matches!(result, SkinParamSupport::UnsupportedKey));
}

#[test]
fn chart_invalid_color_returns_unsupported_value() {
    let result = classify_chart_skinparam("BackgroundColor", "notacolor!!");
    assert!(matches!(result, SkinParamSupport::UnsupportedValue));
}
