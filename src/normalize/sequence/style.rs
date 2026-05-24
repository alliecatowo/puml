use super::state::SequenceNormalizeState;
use super::*;

pub(super) fn parse_teoz_pragma(lower: &str) -> Option<bool> {
    let mut parts = lower.split_whitespace();
    if parts.next()? != "teoz" {
        return None;
    }
    match parts.next() {
        None => Some(true),
        Some("true" | "on" | "yes") => Some(true),
        Some("false" | "off" | "no") => Some(false),
        Some(_) => Some(true),
    }
}

pub(super) fn apply_sequence_skinparam(
    key: &str,
    value: &str,
    span: crate::source::Span,
    style: &mut SequenceStyle,
    footbox_visible: &mut bool,
    monochrome_mode: &mut Option<crate::theme::MonochromeMode>,
    warnings: &mut Vec<Diagnostic>,
) {
    match classify_sequence_skinparam(key, value) {
        SequenceSkinParamSupport::SupportedNoop => {}
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::FootboxVisible(
            visible,
        )) => *footbox_visible = visible,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(color)) => {
            style.arrow_color = color
        }
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::LifelineBorderColor(color),
        ) => style.lifeline_border_color = color,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ParticipantBackgroundColor(color),
        ) => style.participant_background_color = color,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ParticipantBorderColor(color),
        ) => style.participant_border_color = color,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ParticipantFontColor(color),
        ) => style.participant_font_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::NoteBackgroundColor(color),
        ) => style.note_background_color = color,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::NoteBorderColor(
            color,
        )) => style.note_border_color = color,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::GroupBackgroundColor(color),
        ) => style.group_background_color = color,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::GroupBorderColor(
            color,
        )) => style.group_border_color = color,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::RoundCorner(n)) => {
            style.round_corner = n
        }
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Shadowing(
            enabled,
        )) => style.shadowing = enabled,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::DefaultFontName(
            name,
        )) => style.default_font_name = Some(name),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::DefaultFontSize(
            sz,
        )) => style.default_font_size = Some(sz),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BackgroundColor(
            color,
        )) => style.background_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::DefaultTextAlignment(align),
        ) => style.text_alignment = align,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ParticipantPadding(n),
        ) => style.participant_padding = Some(n),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BoxPadding(n)) => {
            style.box_padding = Some(n)
        }
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
            align,
        )) => style.message_align = align,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
        ) => style.response_message_below_arrow = enabled,
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::LifelineThickness(n),
        ) => style.lifeline_thickness = Some(n),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageLineColor(
            color,
        )) => style.message_line_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ReferenceBackgroundColor(color),
        ) => style.reference_background_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ReferenceBorderColor(color),
        ) => style.reference_border_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::GroupHeaderFontColor(color),
        ) => style.group_header_font_color = Some(color),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::GroupHeaderFontStyle(fs),
        ) => style.group_header_font_style = fs,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Monochrome(mode)) => {
            *monochrome_mode = Some(mode)
        }
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Handwritten(
            enabled,
        )) => style.hand_drawn = enabled,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::LifelineNoSolid(
            nosolid,
        )) => style.lifeline_nosolid = nosolid,
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Sepia(enabled)) => {
            style.sepia = enabled
        }
        SequenceSkinParamSupport::UnsupportedValue => {
            warnings.push(common::unsupported_skinparam_value_warning(
                key, value, span,
            ));
        }
        SequenceSkinParamSupport::UnsupportedKey => {
            warnings.push(common::unsupported_skinparam_warning(key, span));
        }
    }
}

impl SequenceNormalizeState {
    pub(super) fn handle_skinparam(
        &mut self,
        span: crate::source::Span,
        key: String,
        value: String,
    ) {
        groups::mark_group_content(&mut self.group_stack);
        self.skinparams.push((key.clone(), value.clone()));
        apply_sequence_skinparam(
            &key,
            &value,
            span,
            &mut self.style,
            &mut self.footbox_visible,
            &mut self.monochrome_mode,
            &mut self.warnings,
        );
    }

    pub(super) fn handle_theme(
        &mut self,
        span: crate::source::Span,
        name: String,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        let preset = resolve_sequence_theme_preset(&name)
            .map_err(|msg| Diagnostic::error(msg).with_span(span))?;
        self.style = preset.style;
        Ok(())
    }

    pub(super) fn handle_pragma(&mut self, span: crate::source::Span, value: String) {
        groups::mark_group_content(&mut self.group_stack);
        let trimmed = value.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("teoz ") || lower == "teoz" {
            self.teoz = parse_teoz_pragma(&lower).unwrap_or(true);
        } else if lower == "sequencemessagespan true" || lower == "sequence message span true" {
            self.style.sequence_message_span = true;
        } else if lower == "sequencemessagespan false" || lower == "sequence message span false" {
            self.style.sequence_message_span = false;
        } else {
            self.warnings
                .push(common::unsupported_pragma_warning(trimmed, span));
        }
    }
}
