pub(super) fn normalize_timing_time(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let trimmed = raw.trim().trim_start_matches('@');
    if let Some(anchor_expr) = trimmed.strip_prefix(':') {
        return normalize_timing_anchor_expr(anchor_expr, current, anchors);
    }
    if let Some((clock_name, multiplier)) = trimmed.split_once('*') {
        if let Some((period, offset)) = clocks.get(clock_name.trim()) {
            if let Ok(n) = multiplier.trim().parse::<i64>() {
                return period.saturating_mul(n).saturating_add(*offset).to_string();
            }
        }
    }
    if let Some(delta) = trimmed
        .strip_prefix('+')
        .and_then(|v| v.parse::<i64>().ok())
    {
        let base = current.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        return base.saturating_add(delta).to_string();
    }
    if let Some(delta) = trimmed
        .strip_prefix('-')
        .and_then(|v| v.parse::<i64>().ok())
    {
        let base = current.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        return base.saturating_sub(delta).to_string();
    }
    trimmed.to_string()
}

pub(super) fn normalize_timing_anchor_expr(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
) -> String {
    let trimmed = raw.trim();
    let split_idx = trimmed
        .char_indices()
        .skip(1)
        .find(|(_, ch)| *ch == '+' || *ch == '-')
        .map(|(idx, _)| idx);
    let (name, offset) = match split_idx {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
        None => (trimmed, None),
    };
    let base = anchors
        .get(name)
        .cloned()
        .unwrap_or_else(|| current.unwrap_or_default().to_string());
    let Some(offset) = offset else {
        return base;
    };
    let base_num = base.parse::<i64>().unwrap_or(0);
    let delta = offset.parse::<i64>().unwrap_or(0);
    base_num.saturating_add(delta).to_string()
}

pub(super) fn normalize_timing_endpoint(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let trimmed = raw.trim();
    let Some((signal, time)) = trimmed.split_once('@') else {
        return current
            .filter(|time| !time.is_empty())
            .map(|time| format!("{trimmed}@{time}"))
            .unwrap_or_else(|| trimmed.to_string());
    };
    let normalized_time = normalize_timing_time(time, current, anchors, clocks);
    format!("{}@{}", signal.trim(), normalized_time)
}

pub(super) fn normalize_timing_range_note(
    note: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let Some(rest) = note.strip_prefix("range:") else {
        return note.to_string();
    };
    let (end, label) = rest.split_once(':').unwrap_or((rest, ""));
    let normalized_end = normalize_timing_time(end, current, anchors, clocks);
    if label.is_empty() {
        format!("range:{normalized_end}")
    } else {
        format!("range:{normalized_end}:{label}")
    }
}
