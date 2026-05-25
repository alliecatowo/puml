use super::*;
use crate::model::TimelineDatePrecision;

struct ChronologyDateSpan {
    start_day: i32,
    end_day: i32,
    precision: TimelineDatePrecision,
}

pub(super) fn normalize_chronology_event(
    subject: String,
    when: String,
    end: Option<String>,
    color: Option<String>,
    bracket: bool,
) -> TimelineChronologyEvent {
    let start_span = parse_chronology_date_span(&when);
    let end_span = end.as_deref().and_then(parse_chronology_date_span);
    TimelineChronologyEvent {
        subject,
        when,
        end,
        color,
        bracket,
        start_day: start_span.as_ref().map(|span| span.start_day),
        end_day: end_span.as_ref().map(|span| span.end_day),
        date_precision: start_span.as_ref().map(|span| span.precision),
        end_date_precision: end_span.as_ref().map(|span| span.precision),
    }
}

fn parse_chronology_date_span(raw: &str) -> Option<ChronologyDateSpan> {
    let cleaned = raw
        .trim()
        .trim_matches('"')
        .trim_matches('[')
        .trim_matches(']')
        .trim();
    if cleaned.is_empty() {
        return None;
    }
    let lower = cleaned.to_ascii_lowercase();
    if let Some(span) = parse_century_span(&lower) {
        return Some(span);
    }
    if let Some(span) = parse_decade_span(&lower) {
        return Some(span);
    }

    let date_token = cleaned
        .split_whitespace()
        .next()
        .unwrap_or("")
        .replace('/', "-");
    let mut parts = date_token.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    if year < 0 {
        return None;
    }
    let Some(month_raw) = parts.next() else {
        return year_span(year, TimelineDatePrecision::Year);
    };
    let month = month_raw.parse::<u32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    let Some(day_raw) = parts.next() else {
        return month_span(year, month);
    };
    let day = day_raw.parse::<u32>().ok()?;
    if parts.next().is_some() || !(1..=last_day_of_month(year, month)).contains(&day) {
        return None;
    }
    let day_number = date_day(year, month, day)?;
    Some(ChronologyDateSpan {
        start_day: day_number,
        end_day: day_number,
        precision: TimelineDatePrecision::Day,
    })
}

fn parse_decade_span(lower: &str) -> Option<ChronologyDateSpan> {
    let token = lower.split_whitespace().next()?;
    let year_raw = token.strip_suffix('s')?;
    if year_raw.len() != 4 {
        return None;
    }
    let start_year = year_raw.parse::<i32>().ok()?;
    if start_year % 10 != 0 {
        return None;
    }
    year_range_span(
        start_year,
        start_year.saturating_add(9),
        TimelineDatePrecision::Decade,
    )
}

fn parse_century_span(lower: &str) -> Option<ChronologyDateSpan> {
    let prefix = lower.strip_suffix(" century")?.trim();
    let century = parse_ordinal_number(prefix)?;
    if century == 0 {
        return None;
    }
    let start_year = (century as i32 - 1).saturating_mul(100).saturating_add(1);
    let end_year = (century as i32).saturating_mul(100);
    year_range_span(start_year, end_year, TimelineDatePrecision::Century)
}

fn parse_ordinal_number(raw: &str) -> Option<u32> {
    let digits = raw
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_alphabetic());
    digits.parse::<u32>().ok()
}

fn year_span(year: i32, precision: TimelineDatePrecision) -> Option<ChronologyDateSpan> {
    year_range_span(year, year, precision)
}

fn year_range_span(
    start_year: i32,
    end_year: i32,
    precision: TimelineDatePrecision,
) -> Option<ChronologyDateSpan> {
    Some(ChronologyDateSpan {
        start_day: date_day(start_year, 1, 1)?,
        end_day: date_day(end_year, 12, 31)?,
        precision,
    })
}

fn month_span(year: i32, month: u32) -> Option<ChronologyDateSpan> {
    Some(ChronologyDateSpan {
        start_day: date_day(year, month, 1)?,
        end_day: date_day(year, month, last_day_of_month(year, month))?,
        precision: TimelineDatePrecision::Month,
    })
}

fn date_day(year: i32, month: u32, day: u32) -> Option<i32> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = i64::from(year);
    let month = i64::from(month);
    let day = i64::from(day);
    let year_adj = year - if month <= 2 { 1 } else { 0 };
    let era = if year_adj >= 0 {
        year_adj
    } else {
        year_adj - 399
    } / 400;
    let year_of_era = year_adj - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146097 + day_of_era - 719468;
    i32::try_from(days).ok()
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
