use super::*;

pub(super) fn state_node_kind_name(kind: &StateNodeKind) -> &'static str {
    match kind {
        StateNodeKind::Normal => "normal",
        StateNodeKind::StartEnd => "start-end",
        StateNodeKind::HistoryShallow => "history-shallow",
        StateNodeKind::HistoryDeep => "history-deep",
        StateNodeKind::Fork => "fork",
        StateNodeKind::Join => "join",
        StateNodeKind::Choice => "choice",
        StateNodeKind::End => "end",
        StateNodeKind::EntryPoint => "entry-point",
        StateNodeKind::ExitPoint => "exit-point",
        StateNodeKind::InputPin => "input-pin",
        StateNodeKind::OutputPin => "output-pin",
        StateNodeKind::ExpansionInput => "expansion-input",
        StateNodeKind::ExpansionOutput => "expansion-output",
        StateNodeKind::Note => "note",
        StateNodeKind::JsonProjection => "json-projection",
    }
}

pub(super) fn state_dash_attr(dashed: bool) -> &'static str {
    if dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

pub(super) fn state_hidden_attr(hidden: bool) -> &'static str {
    if hidden {
        " visibility=\"hidden\""
    } else {
        ""
    }
}

pub(super) fn state_direction_attr(direction: Option<&str>) -> String {
    direction
        .map(|d| format!(" data-state-direction=\"{}\"", escape_text(d)))
        .unwrap_or_default()
}

pub(super) fn state_node_fill(node: &StateNode, state_style: &crate::theme::StateStyle) -> String {
    escape_text(
        node.style
            .fill_color
            .as_deref()
            .unwrap_or(&state_style.background_color),
    )
}

pub(super) fn state_node_border(
    node: &StateNode,
    state_style: &crate::theme::StateStyle,
) -> String {
    escape_text(
        node.style
            .border_color
            .as_deref()
            .unwrap_or(&state_style.border_color),
    )
}

pub(super) fn state_node_text(node: &StateNode, state_style: &crate::theme::StateStyle) -> String {
    escape_text(
        node.style
            .text_color
            .as_deref()
            .unwrap_or(&state_style.font_color),
    )
}

pub(super) fn state_node_font_size(state_style: &crate::theme::StateStyle) -> u32 {
    state_style.font_size.unwrap_or(13)
}

pub(super) fn state_node_border_dash(node: &StateNode) -> &'static str {
    if node.style.border_dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

pub(super) fn state_node_stroke_width(node: &StateNode, fallback: f32) -> String {
    node.style
        .border_thickness
        .map(|value| value.clamp(1, 8).to_string())
        .unwrap_or_else(|| {
            if fallback.fract() == 0.0 {
                format!("{}", fallback as i32)
            } else {
                fallback.to_string()
            }
        })
}

pub(super) fn render_state_note(
    out: &mut String,
    node: &StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let fold = 12;
    out.push_str(&format!(
        "<path class=\"state-note\" d=\"M {x} {y} H {} L {} {} V {} H {x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + w - fold,
        x + w,
        y + fold,
        y + h,
        STATE_NOTE_FILL,
        STATE_NOTE_BORDER
    ));
    out.push_str(&format!(
        "<path class=\"state-note-fold\" d=\"M {} {y} V {} H {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + w - fold,
        y + fold,
        x + w,
        STATE_NOTE_BORDER
    ));
    for (idx, line) in node_display_lines(node).iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#111111\">{}</text>",
            x + STATE_NOTE_PAD_X,
            y + STATE_NOTE_PAD_Y + 11 + idx as i32 * STATE_LABEL_LINE_H,
            escape_text(line)
        ));
    }
}
