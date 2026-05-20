use super::color::parse_color_value;
use super::families::SkinParamSupport;

// ─── Generic skinparam value type used by families that don't yet wire style ──

/// Shared value type for classify functions that recognize common skinparam
/// keys but haven't wired them to per-family style structs yet.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    FontColor(String),
    FontSize(u32),
}

// ─── Gantt ────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Gantt diagrams.
pub fn classify_gantt_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "ganttbackgroundcolor" | "ganttdiagrambackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor" | "ganttbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "ganttfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "ganttfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "ganttarrowcolor"
        | "ganttlinecolor"
        | "todaycolor"
        | "closedcolor"
        | "opencolor"
        | "milestonestyle"
        | "ganttdiagramarrowcolor"
        | "fontname"
        | "ganttfontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── MindMap ──────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for MindMap diagrams.
pub fn classify_mindmap_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "mindmapbackgroundcolor" | "nodebordercolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor" | "mindmapbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "mindmapfontcolor" | "nodefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "mindmapfontsize" | "nodefontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "mindmaparrowcolor" | "nodefontname" | "mindmapfontname" | "roundcorner"
        | "mindmaproundcorner" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── WBS ──────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for WBS (Work Breakdown Structure) diagrams.
pub fn classify_wbs_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "wbsbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "wbsbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "wbsfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "wbsfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "wbsarrowcolor" | "wbsfontname" | "fontname" | "roundcorner" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Timeline (Chronology) ────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Timeline/Chronology diagrams.
pub fn classify_timeline_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "timelinebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "timelinebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "timelinefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "timelinefontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "timelinearrowcolor" | "timelinefontname" | "fontname" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── NwDiag ───────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for NwDiag (network diagram) diagrams.
pub fn classify_nwdiag_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "nwdiagbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "nwdiagbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "nwdiagfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "nwdiagfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "nwdiagarrowcolor" | "nwdiagfontname" | "fontname" | "networkcolor"
        | "nwdiagnetworkcolor" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Archimate ────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Archimate diagrams.
pub fn classify_archimate_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "archimatebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "archimatebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "archimatefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "archimateontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "archimatearrowcolor"
        | "archimatefontname"
        | "fontname"
        | "roundcorner"
        | "archimatestyle" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── SDL ──────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for SDL (Specification and Description Language) diagrams.
pub fn classify_sdl_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "sdlbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "sdlbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "sdlfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "sdlfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "sdlarrowcolor" | "sdlfontname" | "fontname" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Ditaa ────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Ditaa diagrams.
pub fn classify_ditaa_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "ditaabackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "ditaabordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "ditaafontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "ditaafontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontname" | "ditaafontname" | "shadowing" | "ditaashadowing" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Salt ─────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Salt (wireframe/UI mockup) diagrams.
pub fn classify_salt_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "saltbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "saltbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "saltfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "saltfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontname" | "saltfontname" | "roundcorner" | "saltroundcorner" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}
