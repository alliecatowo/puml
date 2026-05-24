use super::*;

pub(super) struct ExtendedFamilyStyles {
    component_style: ComponentStyle,
    activity_style: ActivityStyle,
    timing_style: TimingStyle,
    component_monochrome_mode: Option<crate::theme::MonochromeMode>,
    activity_monochrome_mode: Option<crate::theme::MonochromeMode>,
    timing_monochrome_mode: Option<crate::theme::MonochromeMode>,
    pub(super) sepia: bool,
}

impl Default for ExtendedFamilyStyles {
    fn default() -> Self {
        Self {
            component_style: ComponentStyle::default(),
            activity_style: ActivityStyle::default(),
            timing_style: TimingStyle::default(),
            component_monochrome_mode: None,
            activity_monochrome_mode: None,
            timing_monochrome_mode: None,
            sepia: false,
        }
    }
}

impl ExtendedFamilyStyles {
    pub(super) fn handle_skinparam(
        &mut self,
        family_kind: DiagramKind,
        key: &str,
        value: &str,
        span: crate::source::Span,
        warnings: &mut Vec<Diagnostic>,
    ) {
        let mut handled = false;
        if key.trim().eq_ignore_ascii_case("monochrome") {
            handled = true;
            match classify_sequence_skinparam(key, value) {
                SequenceSkinParamSupport::SupportedNoop => {}
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::Monochrome(mode),
                ) => match family_kind {
                    DiagramKind::Component | DiagramKind::Deployment => {
                        self.component_monochrome_mode = Some(mode);
                    }
                    DiagramKind::Activity => {
                        self.activity_monochrome_mode = Some(mode);
                    }
                    DiagramKind::Timing => {
                        self.timing_monochrome_mode = Some(mode);
                    }
                    _ => {}
                },
                _ => warnings.push(unsupported_value_warning(key, value).with_span(span)),
            }
        } else if key.trim().eq_ignore_ascii_case("handwritten") {
            handled = true;
            match classify_sequence_skinparam(key, value) {
                SequenceSkinParamSupport::SupportedNoop
                | SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::Handwritten(_),
                ) => {}
                _ => warnings.push(unsupported_value_warning(key, value).with_span(span)),
            }
        } else if key.trim().eq_ignore_ascii_case("sepia") {
            handled = true;
            if let SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Sepia(
                enabled,
            )) = classify_sequence_skinparam(key, value)
            {
                self.sepia = enabled;
            }
        }
        if matches!(
            family_kind,
            DiagramKind::Component | DiagramKind::Deployment
        ) {
            use crate::theme::ComponentSkinParamValue;
            match classify_component_skinparam(key, value) {
                SkinParamSupport::SupportedNoop => {
                    handled = true;
                }
                SkinParamSupport::SupportedWithValue(v) => {
                    handled = true;
                    match v {
                        ComponentSkinParamValue::BackgroundColor(c) => {
                            self.component_style.background_color = c;
                        }
                        ComponentSkinParamValue::BorderColor(c) => {
                            self.component_style.border_color = c;
                        }
                        ComponentSkinParamValue::InterfaceColor(c) => {
                            self.component_style.interface_color = c;
                        }
                        ComponentSkinParamValue::FontColor(c) => {
                            self.component_style.font_color = c;
                        }
                        ComponentSkinParamValue::ArrowColor(c) => {
                            self.component_style.arrow_color = c;
                        }
                        ComponentSkinParamValue::StyleMode(mode) => {
                            self.component_style.component_style_mode = mode;
                        }
                    }
                }
                SkinParamSupport::UnsupportedKey => {}
                SkinParamSupport::UnsupportedValue => {
                    handled = true;
                    warnings.push(unsupported_value_warning(key, value).with_span(span));
                }
            }
        }
        if !handled && matches!(family_kind, DiagramKind::Activity) {
            use crate::theme::ActivitySkinParamValue;
            match classify_activity_skinparam(key, value) {
                SkinParamSupport::SupportedNoop => {
                    handled = true;
                }
                SkinParamSupport::SupportedWithValue(v) => {
                    handled = true;
                    match v {
                        ActivitySkinParamValue::BackgroundColor(c) => {
                            self.activity_style.background_color = c;
                        }
                        ActivitySkinParamValue::BorderColor(c) => {
                            self.activity_style.border_color = c;
                        }
                        ActivitySkinParamValue::DiamondBackgroundColor(c) => {
                            self.activity_style.diamond_color = c;
                        }
                        ActivitySkinParamValue::BarColor(c) => {
                            self.activity_style.fork_color = c;
                        }
                        ActivitySkinParamValue::FontColor(c) => {
                            self.activity_style.font_color = c;
                        }
                        ActivitySkinParamValue::ArrowColor(c) => {
                            self.activity_style.arrow_color = c;
                        }
                    }
                }
                SkinParamSupport::UnsupportedKey => {}
                SkinParamSupport::UnsupportedValue => {
                    handled = true;
                    warnings.push(unsupported_value_warning(key, value).with_span(span));
                }
            }
        }
        if !handled && matches!(family_kind, DiagramKind::Timing) {
            use crate::theme::TimingSkinParamValue;
            match classify_timing_skinparam(key, value) {
                SkinParamSupport::SupportedNoop => {
                    handled = true;
                }
                SkinParamSupport::SupportedWithValue(v) => {
                    handled = true;
                    match v {
                        TimingSkinParamValue::BackgroundColor(c) => {
                            self.timing_style.background_color = c;
                        }
                        TimingSkinParamValue::AxisColor(c) => {
                            self.timing_style.axis_color = c;
                        }
                        TimingSkinParamValue::GridColor(c) => {
                            self.timing_style.grid_color = c;
                        }
                        TimingSkinParamValue::SignalBackgroundColor(c) => {
                            self.timing_style.signal_background_color = c;
                        }
                        TimingSkinParamValue::SignalBorderColor(c) => {
                            self.timing_style.signal_border_color = c;
                        }
                        TimingSkinParamValue::ArrowColor(c) => {
                            self.timing_style.arrow_color = c;
                        }
                        TimingSkinParamValue::FontColor(c) => {
                            self.timing_style.font_color = c;
                        }
                    }
                }
                SkinParamSupport::UnsupportedKey => {}
                SkinParamSupport::UnsupportedValue => {
                    handled = true;
                    warnings.push(unsupported_value_warning(key, value).with_span(span));
                }
            }
        }
        if !handled {
            warnings.push(
                Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                    key
                ))
                .with_span(span),
            );
        }
    }

    pub(super) fn apply_theme(
        &mut self,
        family_kind: DiagramKind,
        value: &str,
        span: crate::source::Span,
    ) -> Result<(), Diagnostic> {
        let style = resolve_sequence_theme_preset(value)
            .map_err(|msg| Diagnostic::error(msg).with_span(span))?
            .style;
        match family_kind {
            DiagramKind::Component | DiagramKind::Deployment => {
                self.component_style = component_style_from_sequence_theme(&style);
            }
            DiagramKind::Activity => {
                self.activity_style = activity_style_from_sequence_theme(&style);
            }
            DiagramKind::Timing => {
                self.timing_style = timing_style_from_sequence_theme(&style);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn into_family_style(mut self, family_kind: DiagramKind) -> Option<FamilyStyle> {
        match family_kind {
            DiagramKind::Component | DiagramKind::Deployment => {
                if let Some(mode) = self.component_monochrome_mode {
                    apply_monochrome_to_component_style(&mut self.component_style, mode);
                }
                Some(FamilyStyle::Component(self.component_style))
            }
            DiagramKind::Activity => {
                if let Some(mode) = self.activity_monochrome_mode {
                    apply_monochrome_to_activity_style(&mut self.activity_style, mode);
                }
                Some(FamilyStyle::Activity(self.activity_style))
            }
            DiagramKind::Timing => {
                if let Some(mode) = self.timing_monochrome_mode {
                    apply_monochrome_to_timing_style(&mut self.timing_style, mode);
                }
                Some(FamilyStyle::Timing(self.timing_style))
            }
            _ => None,
        }
    }
}

fn unsupported_value_warning(key: &str, value: &str) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
        value, key
    ))
}
