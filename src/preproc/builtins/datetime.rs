use crate::preproc::PreprocState;

pub(super) fn deterministic_preproc_now_seconds(state: &PreprocState) -> i64 {
    state
        .vars
        .get("PUML_NOW")
        .or_else(|| state.vars.get("PUML_NOW_EPOCH"))
        .and_then(|value| crate::preproc::includes::eval_int_expr(value))
        .unwrap_or(0)
}

pub(super) fn format_preprocessor_date(
    format_arg: Option<&str>,
    seconds_arg: Option<&str>,
    state: &PreprocState,
) -> String {
    let format = format_arg
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("yyyy-MM-dd");
    let seconds = seconds_arg
        .and_then(crate::preproc::includes::eval_int_expr)
        .unwrap_or_else(|| deterministic_preproc_now_seconds(state));
    let (date, time) = unix_seconds_to_utc_parts(seconds);
    format_preprocessor_datetime(format, date, time)
}

pub(super) fn format_preprocessor_time(format_arg: Option<&str>, state: &PreprocState) -> String {
    let format = format_arg
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("HH:mm:ss");
    let (date, time) = unix_seconds_to_utc_parts(deterministic_preproc_now_seconds(state));
    format_preprocessor_datetime(format, date, time)
}

fn unix_seconds_to_utc_parts(seconds: i64) -> ((i32, u32, u32), (u32, u32, u32)) {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400) as u32;
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    (civil_from_unix_days(days), (hour, minute, second))
}

fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn format_preprocessor_datetime(
    format: &str,
    date: (i32, u32, u32),
    time: (u32, u32, u32),
) -> String {
    let (year, month, day) = date;
    let (hour, minute, second) = time;
    let mut out = String::with_capacity(format.len() + 8);
    let mut i = 0usize;
    while i < format.len() {
        let rest = &format[i..];
        if rest.starts_with("yyyy") || rest.starts_with("YYYY") {
            out.push_str(&format!("{year:04}"));
            i += 4;
        } else if rest.starts_with("yy") || rest.starts_with("YY") {
            out.push_str(&format!("{:02}", year.rem_euclid(100)));
            i += 2;
        } else if rest.starts_with("MM") {
            out.push_str(&format!("{month:02}"));
            i += 2;
        } else if rest.starts_with('M') {
            out.push_str(&month.to_string());
            i += 1;
        } else if rest.starts_with("dd") || rest.starts_with("DD") {
            out.push_str(&format!("{day:02}"));
            i += 2;
        } else if rest.starts_with('d') || rest.starts_with('D') {
            out.push_str(&day.to_string());
            i += 1;
        } else if rest.starts_with("HH") {
            out.push_str(&format!("{hour:02}"));
            i += 2;
        } else if rest.starts_with("mm") {
            out.push_str(&format!("{minute:02}"));
            i += 2;
        } else if rest.starts_with("ss") {
            out.push_str(&format!("{second:02}"));
            i += 2;
        } else {
            let ch = rest.chars().next().unwrap_or_default();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}
