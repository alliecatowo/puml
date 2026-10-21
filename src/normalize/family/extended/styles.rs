use super::*;

pub(super) struct ExtendedFamilyStyles {
    graph_style: crate::theme::GraphStyleCascade,
    activity_style: ActivityStyle,
    timing_style: TimingStyle,
    activity_monochrome_mode: Option<crate::theme::MonochromeMode>,
    timing_monochrome_mode: Option<crate::theme::MonochromeMode>,
    sepia: bool,
    /// Phase B (#1404): accumulated `<style>` block rules for component/deployment.
    style_builder: crate::theme::StyleBuilder,
}

impl ExtendedFamilyStyles {
    pub(super) fn new(family_kind: DiagramKind) -> Self {
        let graph_family = match family_kind {
            DiagramKind::Deployment => crate::theme::GraphStyleFamily::Deployment,
            _ => crate::theme::GraphStyleFamily::Component,
        };
        Self {
            graph_style: crate::theme::GraphStyleCascade::new(graph_family),
            activity_style: ActivityStyle::default(),
            timing_style: TimingStyle::default(),
            activity_monochrome_mode: None,
            timing_monochrome_mode: None,
            sepia: false,
            style_builder: crate::theme::StyleBuilder::new(),
        }
    }

    /// Phase B (#1404): push all Regular-scheme rules from a parsed `<style>` block.
    /// Phase E (#1417): emit W_STYLE_UNKNOWN_TAG / W_STYLE_UNKNOWN_PROPERTY /
    /// E_STYLE_BAD_VALUE diagnostics via push_with_warnings.
    pub(super) fn push_style_block(
        &mut self,
        block: crate::ast::style::StyleBlock,
        warnings: &mut Vec<Diagnostic>,
    ) {
        for rule in block.rules {
            if rule.scheme == crate::ast::style::StyleScheme::Regular {
                self.style_builder.push_with_warnings(rule, warnings);
            }
        }
    }

    pub(super) fn sepia(&self) -> bool {
        self.sepia || self.graph_style.sepia()
    }

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
                        self.graph_style.apply_skinparam(key, value, span, warnings);
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
            self.graph_style.apply_skinparam(key, value, span, warnings);
            return;
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
                self.graph_style.apply_theme(value, span)?;
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
                let mut fs = self.graph_style.into_family_style();
                // Phase B (#1404): attach the StyleBuilder so the cascade resolver can
                // query `<style>` rules per element at render time.
                if !self.style_builder.is_empty() {
                    if let FamilyStyle::Component(ref mut cs) = fs {
                        cs.style_builder = Some(Box::new(self.style_builder));
                    }
                }
                Some(fs)
            }
            DiagramKind::Activity => {
                if let Some(mode) = self.activity_monochrome_mode {
                    apply_monochrome_to_activity_style(&mut self.activity_style, mode);
                }
                // Phase E (#1417): apply StyleBuilder rules to the flat ActivityStyle fields
                // so the renderer (which reads directly from these fields) picks up <style>
                // block colours.  The compat shim previously handled this translation.
                if !self.style_builder.is_empty() {
                    apply_style_builder_to_activity(&mut self.activity_style, &self.style_builder);
                    self.activity_style.style_builder = Some(Box::new(self.style_builder.clone()));
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

// ---------------------------------------------------------------------------
// StyleBlock → ActivityStyle bridge (Phase E, #1417)
// ---------------------------------------------------------------------------

/// Query `builder` for `<style>` rules that target activity diagram elements
/// and apply the resulting colours to the flat `ActivityStyle` fields.
///
/// This replaces the `StyleParam` compat shim path that previously translated
/// e.g. `activityDiagram { activity { BackgroundColor … } }` into
/// `ActivityBackgroundColor` skinparam triples.
fn apply_style_builder_to_activity(
    act: &mut crate::theme::ActivityStyle,
    builder: &crate::theme::StyleBuilder,
) {
    use crate::ast::style::{PName, SName};
    use crate::theme::style_builder::StyleQuery;

    let color = |query: &StyleQuery, pname: PName| -> Option<String> {
        builder.resolve(query).color(pname).map(str::to_string)
    };

    // activityDiagram { ArrowColor #... } (diagram-level; arrowcolor is an alias for linecolor)
    let diagram_q = StyleQuery::tags([SName::ActivityDiagram]);
    if let Some(c) = color(&diagram_q, PName::LineColor) {
        act.arrow_color = c;
    }

    // activityDiagram { activity { BackgroundColor / BorderColor / FontColor } }
    let act_q = StyleQuery::tags([SName::ActivityDiagram, SName::Activity]);
    if let Some(c) = color(&act_q, PName::BackgroundColor) {
        act.background_color = c;
    }
    if let Some(c) = color(&act_q, PName::LineColor) {
        act.border_color = c;
    }
    if let Some(c) = color(&act_q, PName::FontColor) {
        act.font_color = c;
    }

    // activityDiagram { diamond { BackgroundColor } }
    let diamond_q = StyleQuery::tags([SName::ActivityDiagram, SName::Diamond]);
    if let Some(c) = color(&diamond_q, PName::BackgroundColor) {
        act.diamond_color = c;
    }

    // activityDiagram { bar { BackgroundColor } } (also covers fork/start/stop)
    // The selector is `bar` → SName::Bar.
    let bar_q = StyleQuery::tags([SName::ActivityDiagram, SName::Bar]);
    if let Some(c) = color(&bar_q, PName::BackgroundColor) {
        act.fork_color = c;
    }
}
