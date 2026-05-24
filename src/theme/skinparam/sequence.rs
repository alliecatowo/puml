use super::helpers::{parse_bool_value, parse_monochrome_value};
use crate::theme::color::parse_color_value;
use crate::theme::styles::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamValue {
    FootboxVisible(bool),
    ArrowColor(String),
    LifelineBorderColor(String),
    ParticipantBackgroundColor(String),
    ParticipantBorderColor(String),
    ParticipantFontColor(String),
    NoteBackgroundColor(String),
    NoteBorderColor(String),
    GroupBackgroundColor(String),
    GroupBorderColor(String),
    RoundCorner(i32),
    Shadowing(bool),
    DefaultFontName(String),
    DefaultFontSize(u32),
    BackgroundColor(String),
    DefaultTextAlignment(TextAlignment),
    // --- Extended skinparams (#182 wishlist) ---
    ParticipantPadding(i32),
    BoxPadding(i32),
    MessageAlign(MessageAlign),
    ResponseMessageBelowArrow(bool),
    LifelineThickness(i32),
    MessageLineColor(String),
    ReferenceBackgroundColor(String),
    ReferenceBorderColor(String),
    GroupHeaderFontColor(String),
    GroupHeaderFontStyle(GroupHeaderFontStyle),
    Monochrome(MonochromeMode),
    Handwritten(bool),
    /// `skinparam lifelineStrategy nosolid|solid`
    LifelineNoSolid(bool),
    Sepia(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamSupport {
    SupportedNoop,
    SupportedWithValue(SequenceSkinParamValue),
    UnsupportedKey,
    UnsupportedValue,
}

pub fn classify_sequence_skinparam(key: &str, value: &str) -> SequenceSkinParamSupport {
    let normalized_key = key.trim().to_ascii_lowercase();
    match normalized_key.as_str() {
        "maxmessagesize" => SequenceSkinParamSupport::SupportedNoop,
        "footbox" | "sequencefootbox" => parse_footbox_value(value)
            .map(SequenceSkinParamSupport::SupportedWithValue)
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "arrowcolor" | "sequencearrowcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
                    color,
                ))
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "lifelinebordercolor" | "sequencelifelinebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
            parse_color_value(value)
                .map(|color| {
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    )
                })
                .unwrap_or(SequenceSkinParamSupport::UnsupportedValue)
        }
        "participantbordercolor" | "sequenceparticipantbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "participantfontcolor" | "sequenceparticipantfontcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantFontColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebackgroundcolor" | "sequencenotebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebordercolor" | "sequencenotebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbackgroundcolor" | "sequencegroupbackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbordercolor" | "sequencegroupbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "roundcorner" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::RoundCorner(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "shadowing" => {
            let Some(enabled) = parse_bool_value(value) else {
                return SequenceSkinParamSupport::UnsupportedValue;
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Shadowing(enabled))
        }
        "monochrome" => match parse_monochrome_value(value) {
            Some(Some(mode)) => SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::Monochrome(mode),
            ),
            Some(None) => SequenceSkinParamSupport::SupportedNoop,
            None => SequenceSkinParamSupport::UnsupportedValue,
        },
        "handwritten" => parse_bool_value(value)
            .map(|enabled| {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Handwritten(
                    enabled,
                ))
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sepia" => parse_bool_value(value)
            .map(|enabled| {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Sepia(enabled))
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "defaultfontname" | "participantfontname" | "sequenceparticipantfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SequenceSkinParamSupport::UnsupportedValue
            } else {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontName(name.to_string()),
                )
            }
        }
        "defaultfontsize" | "participantfontsize" | "sequenceparticipantfontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontSize(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "backgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::BackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "defaulttextalignment" => {
            let lower = value.trim().to_ascii_lowercase();
            let alignment = match lower.as_str() {
                "center" => TextAlignment::Center,
                "left" => TextAlignment::Left,
                "right" => TextAlignment::Right,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::DefaultTextAlignment(alignment),
            )
        }
        // --- Extended skinparams (#182 wishlist) ---
        "participantpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantPadding(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "boxpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BoxPadding(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "sequencemessagealign" => {
            let lower = value.trim().to_ascii_lowercase();
            let align = match lower.as_str() {
                "left" | "direction" => MessageAlign::Left,
                "center" => MessageAlign::Center,
                "right" | "reversedirection" | "reverse_direction" | "reverse-direction" => {
                    MessageAlign::Right
                }
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
                align,
            ))
        }
        "sequencereferencealign" => {
            let lower = value.trim().to_ascii_lowercase();
            match lower.as_str() {
                "left" | "center" | "right" | "direction" | "reversedirection"
                | "reverse_direction" | "reverse-direction" => {
                    SequenceSkinParamSupport::SupportedNoop
                }
                _ => SequenceSkinParamSupport::UnsupportedValue,
            }
        }
        "responsemessagebelowarrow" | "sequenceresponsemessagebelowarrow" => {
            let lower = value.trim().to_ascii_lowercase();
            let enabled = match lower.as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
            )
        }
        "sequencelifelinethickness" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineThickness(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "messagelinecolor" | "sequencemessagelinecolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::MessageLineColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebackgroundcolor" | "sequencereferencebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebordercolor" | "sequencereferencebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupHeaderFontColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontstyle" => {
            let lower = value.trim().to_ascii_lowercase();
            let style = match lower.as_str() {
                "normal" => GroupHeaderFontStyle::Normal,
                "bold" => GroupHeaderFontStyle::Bold,
                "italic" => GroupHeaderFontStyle::Italic,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::GroupHeaderFontStyle(style),
            )
        }
        "lifelinestrategy" => {
            let lower = value.trim().to_ascii_lowercase();
            match lower.as_str() {
                "nosolid" => SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineNoSolid(true),
                ),
                "solid" | "" => SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineNoSolid(false),
                ),
                _ => SequenceSkinParamSupport::UnsupportedValue,
            }
        }
        _ => SequenceSkinParamSupport::UnsupportedKey,
    }
}

fn parse_footbox_value(value: &str) -> Option<SequenceSkinParamValue> {
    let normalized = value.trim().to_ascii_lowercase();
    let visible = match normalized.as_str() {
        "show" | "true" | "yes" | "on" => true,
        "hide" | "false" | "no" | "off" => false,
        _ => return None,
    };
    Some(SequenceSkinParamValue::FootboxVisible(visible))
}
