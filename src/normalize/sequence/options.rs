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

/// Strip the LEGEND_POS prefix from a packed legend value, returning just the text.
pub(crate) fn strip_legend_pos_prefix(v: &str) -> String {
    if let Some(rest) = v.strip_prefix("LEGEND_POS:") {
        if let Some(nl) = rest.find('\n') {
            return rest[nl + 1..].to_string();
        }
        return String::new();
    }
    v.to_string()
}

/// Parse a scale body (everything after "scale ").
/// Supports:
///   "1.5"          → Factor(1.5)
///   "800*600"      → Fixed { width: 800, height: 600 }
///   "max 800"      → Max(800)
pub(super) fn parse_scale_spec(body: &str) -> Option<ScaleSpec> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("max ") {
        let n: u32 = rest.trim().parse().ok()?;
        return Some(ScaleSpec::Max(n));
    }
    if let Some(idx) = trimmed.find('*') {
        let w: u32 = trimmed[..idx].trim().parse().ok()?;
        let h: u32 = trimmed[idx + 1..].trim().parse().ok()?;
        return Some(ScaleSpec::Fixed {
            width: w,
            height: h,
        });
    }
    let f: f64 = trimmed.parse().ok()?;
    if f > 0.0 {
        Some(ScaleSpec::Factor(f))
    } else {
        None
    }
}

pub(super) fn unsupported_family_diagnostic(kind: DiagramKind) -> Diagnostic {
    let (code, family) = match kind {
        DiagramKind::Component => ("E_FAMILY_COMPONENT_UNSUPPORTED", "component"),
        DiagramKind::Deployment => ("E_FAMILY_DEPLOYMENT_UNSUPPORTED", "deployment"),
        DiagramKind::State => ("E_FAMILY_STATE_UNSUPPORTED", "state"),
        DiagramKind::Activity => ("E_FAMILY_ACTIVITY_UNSUPPORTED", "activity"),
        DiagramKind::Timing => ("E_FAMILY_TIMING_UNSUPPORTED", "timing"),
        DiagramKind::Gantt => ("E_FAMILY_GANTT_UNSUPPORTED", "gantt"),
        DiagramKind::Chronology => ("E_FAMILY_CHRONOLOGY_UNSUPPORTED", "chronology"),
        DiagramKind::Salt => ("E_FAMILY_SALT_UNSUPPORTED", "salt"),
        _ => ("E_FAMILY_UNSUPPORTED", "unknown"),
    };

    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{family}` is not implemented yet; sequence is currently supported"
        ),
    )
}
