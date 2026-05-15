use crate::scene::TextOverflowPolicy;

#[derive(Debug, Clone)]
pub struct Theme {
    pub skinparams: Vec<(String, String)>,
    pub footbox_visible: bool,
    pub text_overflow_policy: TextOverflowPolicy,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: false,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: true,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceSkinParamValue {
    FootboxVisible(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
