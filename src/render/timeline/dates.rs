pub(super) fn parse_relative_day(raw: &str) -> Option<u32> {
    let t = raw.trim();
    let rest = t.strip_prefix("D+").or_else(|| t.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

pub(super) fn parse_iso_date_tuple(raw: &str) -> Option<(i32, i32, i32)> {
    let normalized = raw.trim().replace('/', "-");
    let mut parts = normalized.split('-');
    let y = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<i32>().ok()?;
    let d = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((y, m, d))
}

pub(super) fn parse_iso_date_day_number(raw: &str) -> Option<u32> {
    let (y, m, d) = parse_iso_date_tuple(raw)?;
    if y < 0 || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = i64::from(y);
    let m = i64::from(m);
    let d = i64::from(d);
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    if days < 0 {
        return None;
    }
    u32::try_from(days).ok()
}

pub(super) fn day_number_to_iso(day: u32) -> Option<String> {
    let z = i64::from(day) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    Some(format!("{y:04}-{m:02}-{d:02}"))
}
