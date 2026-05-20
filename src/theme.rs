mod color;
mod core;
mod families;
mod generic;
mod presets;
mod sequence;

pub use color::{css3_color_to_hex, hex_color_is_dark};
pub use core::Theme;
pub use families::{
    classify_activity_skinparam, classify_chart_skinparam, classify_class_skinparam,
    classify_component_skinparam, classify_state_skinparam, classify_timing_skinparam,
    ActivitySkinParamValue, ActivityStyle, ChartSkinParamValue, ChartStyle, ClassSkinParamValue,
    ClassStyle, ComponentSkinParamValue, ComponentStyle, SkinParamSupport, StateSkinParamValue,
    StateStyle, TimingSkinParamValue, TimingStyle,
};
pub use generic::{
    classify_archimate_skinparam, classify_ditaa_skinparam, classify_gantt_skinparam,
    classify_mindmap_skinparam, classify_nwdiag_skinparam, classify_salt_skinparam,
    classify_sdl_skinparam, classify_timeline_skinparam, classify_wbs_skinparam,
    GenericSkinParamValue,
};
pub use presets::{
    activity_style_from_sequence_theme, chart_style_from_sequence_theme,
    class_style_from_sequence_theme, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, state_style_from_sequence_theme,
    timing_style_from_sequence_theme, SequenceThemePreset, LOCAL_SEQUENCE_THEME_CATALOG,
};
pub use sequence::{
    classify_sequence_skinparam, GroupHeaderFontStyle, MessageAlign, SequenceSkinParamSupport,
    SequenceSkinParamValue, SequenceStyle, TextAlignment,
};
