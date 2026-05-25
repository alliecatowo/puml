use super::skinparam::*;
use super::styles::*;
use super::StyleSource;
use std::collections::BTreeMap;

pub fn class_style_from_sequence_theme(style: &SequenceStyle) -> ClassStyle {
    ClassStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        header_color: style.group_background_color.clone(),
        member_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
        font_size: style.default_font_size,
        font_name: style.default_font_name.clone(),
        actor_style: ActorStyle::Stick,
        attribute_icons: true,
        stereotype_styles: BTreeMap::new(),
        sources: ClassStyleSources {
            background_color: StyleSource::ThemePreset,
            border_color: StyleSource::ThemePreset,
            header_color: StyleSource::ThemePreset,
            member_color: StyleSource::ThemePreset,
            font_color: StyleSource::ThemePreset,
            arrow_color: StyleSource::ThemePreset,
            font_size: style
                .default_font_size
                .map(|_| StyleSource::ThemePreset)
                .unwrap_or_default(),
            font_name: style
                .default_font_name
                .as_ref()
                .map(|_| StyleSource::ThemePreset)
                .unwrap_or_default(),
        },
    }
}

pub fn state_style_from_sequence_theme(style: &SequenceStyle) -> StateStyle {
    StateStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        arrow_color: style.arrow_color.clone(),
        start_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        font_size: style.default_font_size,
    }
}

pub fn component_style_from_sequence_theme(style: &SequenceStyle) -> ComponentStyle {
    ComponentStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        interface_color: style.note_background_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
        component_style_mode: ComponentStyleMode::Uml2,
        target_styles: BTreeMap::new(),
        stereotype_styles: BTreeMap::new(),
        sources: ComponentStyleSources {
            background_color: StyleSource::ThemePreset,
            border_color: StyleSource::ThemePreset,
            interface_color: StyleSource::ThemePreset,
            font_color: StyleSource::ThemePreset,
            arrow_color: StyleSource::ThemePreset,
        },
    }
}

pub fn activity_style_from_sequence_theme(style: &SequenceStyle) -> ActivityStyle {
    ActivityStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        diamond_color: style.note_background_color.clone(),
        fork_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
    }
}

pub fn timing_style_from_sequence_theme(style: &SequenceStyle) -> TimingStyle {
    TimingStyle {
        background_color: style
            .background_color
            .clone()
            .unwrap_or_else(|| "#ffffff".to_string()),
        axis_color: style.arrow_color.clone(),
        grid_color: style.lifeline_border_color.clone(),
        signal_background_color: style.participant_background_color.clone(),
        signal_border_color: style.participant_border_color.clone(),
        arrow_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
    }
}

pub fn chart_style_from_sequence_theme(style: &SequenceStyle) -> ChartStyle {
    ChartStyle {
        background_color: style
            .background_color
            .clone()
            .unwrap_or_else(|| "#ffffff".to_string()),
        axis_color: style.arrow_color.clone(),
        grid_color: style.lifeline_border_color.clone(),
        series_color: style.arrow_color.clone(),
        bar_color: style.participant_border_color.clone(),
        line_color: style.arrow_color.clone(),
        pie_border_color: style.group_border_color.clone(),
        font_color: style.arrow_color.clone(),
    }
}

pub fn mindmap_style_from_sequence_theme(style: &SequenceStyle) -> MindMapStyle {
    let mut depth_styles = BTreeMap::new();
    depth_styles.insert(
        0,
        MindMapDepthStyle {
            background_color: Some(style.group_background_color.clone()),
            font_color: Some(style.arrow_color.clone()),
            border_color: Some(style.group_border_color.clone()),
        },
    );
    depth_styles.insert(
        1,
        MindMapDepthStyle {
            background_color: Some(style.participant_background_color.clone()),
            font_color: Some(style.arrow_color.clone()),
            border_color: Some(style.participant_border_color.clone()),
        },
    );
    depth_styles.insert(
        2,
        MindMapDepthStyle {
            background_color: Some(style.note_background_color.clone()),
            font_color: Some(style.arrow_color.clone()),
            border_color: Some(style.note_border_color.clone()),
        },
    );
    MindMapStyle { depth_styles }
}

pub fn apply_monochrome_to_sequence_style(style: &mut SequenceStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.arrow_color = ink.clone();
    style.lifeline_border_color = ink.clone();
    style.participant_background_color = paper.clone();
    style.participant_border_color = ink.clone();
    style.participant_font_color = Some(ink.clone());
    style.note_background_color = paper.clone();
    style.note_border_color = ink.clone();
    style.group_background_color = paper.clone();
    style.group_border_color = ink.clone();
    style.background_color = Some(paper.clone());
    style.message_line_color = Some(ink.clone());
    style.reference_background_color = Some(paper.clone());
    style.reference_border_color = Some(ink.clone());
    style.group_header_font_color = Some(ink);
    style.shadowing = false;
}

pub fn apply_monochrome_to_class_style(style: &mut ClassStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper.clone();
    style.border_color = ink.clone();
    style.header_color = paper.clone();
    style.member_color = ink.clone();
    style.font_color = ink.clone();
    style.arrow_color = ink;
    style.stereotype_styles.clear();
}

pub fn apply_monochrome_to_state_style(style: &mut StateStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper;
    style.border_color = ink.clone();
    style.arrow_color = ink.clone();
    style.start_color = ink.clone();
    style.font_color = ink;
}

pub fn apply_monochrome_to_component_style(style: &mut ComponentStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper.clone();
    style.border_color = ink.clone();
    style.interface_color = paper;
    style.font_color = ink.clone();
    style.arrow_color = ink;
    style.target_styles.clear();
    style.stereotype_styles.clear();
}

pub fn apply_monochrome_to_activity_style(style: &mut ActivityStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper.clone();
    style.border_color = ink.clone();
    style.diamond_color = paper;
    style.fork_color = ink.clone();
    style.font_color = ink.clone();
    style.arrow_color = ink;
}

pub fn apply_monochrome_to_timing_style(style: &mut TimingStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper.clone();
    style.axis_color = ink.clone();
    style.grid_color = ink.clone();
    style.signal_background_color = paper.clone();
    style.signal_border_color = ink.clone();
    style.arrow_color = ink.clone();
    style.font_color = ink;
}

pub fn apply_monochrome_to_chart_style(style: &mut ChartStyle, mode: MonochromeMode) {
    let ink = mode.ink().to_string();
    let paper = mode.paper().to_string();
    style.background_color = paper;
    style.axis_color = ink.clone();
    style.grid_color = ink.clone();
    style.series_color = ink.clone();
    style.bar_color = ink.clone();
    style.line_color = ink.clone();
    style.pie_border_color = ink.clone();
    style.font_color = ink;
}
