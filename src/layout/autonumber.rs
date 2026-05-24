#[derive(Debug, Default)]
pub(super) struct AutonumberState {
    pub(super) enabled: bool,
    pub(super) next: AutonumberCounter,
    pub(super) step: u64,
    pub(super) format: Option<String>,
}

impl AutonumberState {
    pub(super) fn update(&mut self, raw: Option<&str>) {
        let value = raw.map(str::trim).unwrap_or("");
        if value.eq_ignore_ascii_case("stop") || value.eq_ignore_ascii_case("off") {
            self.enabled = false;
            return;
        }

        if value.is_empty() {
            if self.next.is_zero() {
                self.next = AutonumberCounter::from_number(1);
            }
            if self.step == 0 {
                self.step = 1;
            }
            self.enabled = true;
            return;
        }

        let parsed = parse_autonumber_command(value);
        if let Some(level) = parsed.increment_level {
            self.next.increment_level(level, self.step.max(1));
            self.enabled = true;
            return;
        }
        if parsed.resume_only {
            if self.next.is_zero() {
                self.next = AutonumberCounter::from_number(1);
            }
        } else {
            self.next = parsed
                .start
                .unwrap_or_else(|| AutonumberCounter::from_number(1));
        }
        if let Some(step) = parsed.step {
            self.step = step.max(1);
        } else if self.step == 0 {
            self.step = 1;
        }
        if let Some(fmt) = parsed.format {
            self.format = Some(fmt);
        }
        self.enabled = true;
    }

    pub(super) fn apply(&mut self, label: Option<String>) -> Option<String> {
        if !self.enabled {
            return label;
        }
        if self.next.is_zero() {
            self.next = AutonumberCounter::from_number(1);
        }
        if self.step == 0 {
            self.step = 1;
        }

        let number = format_autonumber(&self.next, self.format.as_deref());
        self.next.advance(self.step);
        match label {
            Some(text) if text.contains("%autonumber%") => {
                Some(text.replace("%autonumber%", &number))
            }
            Some(text) if !text.is_empty() => Some(format!("{number} {text}")),
            _ => Some(number.to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AutonumberCounter {
    pub(super) prefix: Vec<String>,
    pub(super) separators: Vec<char>,
    pub(super) current: u64,
    pub(super) width: usize,
}

impl AutonumberCounter {
    pub(super) fn from_number(value: u64) -> Self {
        Self {
            prefix: Vec::new(),
            separators: Vec::new(),
            current: value,
            width: 0,
        }
    }

    pub(super) fn from_token(token: &str) -> Option<Self> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        let mut separators = Vec::new();
        let mut current_part = String::new();
        for ch in trimmed.chars() {
            if matches!(ch, '.' | ';' | ',' | ':') {
                if current_part.is_empty() {
                    return None;
                }
                parts.push(current_part);
                separators.push(ch);
                current_part = String::new();
            } else if ch.is_ascii_digit() {
                current_part.push(ch);
            } else {
                return None;
            }
        }
        if !current_part.is_empty() {
            parts.push(current_part);
        }
        if parts.is_empty()
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        let last = parts.last()?;
        let current = last.parse::<u64>().ok()?;
        let width = if last.len() > 1 { last.len() } else { 0 };
        Some(Self {
            prefix: parts[..parts.len().saturating_sub(1)]
                .iter()
                .map(|part| (*part).to_string())
                .collect(),
            separators,
            current,
            width,
        })
    }

    pub(super) fn is_zero(&self) -> bool {
        self.prefix.is_empty() && self.current == 0
    }

    pub(super) fn advance(&mut self, step: u64) {
        self.current = self.current.saturating_add(step.max(1));
    }

    pub(super) fn increment_level(&mut self, level: usize, step: u64) {
        if level == 0 {
            return;
        }
        if level <= self.prefix.len() {
            if let Some(part) = self.prefix.get_mut(level - 1) {
                let width = part.len();
                let next = part.parse::<u64>().unwrap_or(0).saturating_add(step.max(1));
                *part = if width > 1 {
                    format!("{:0width$}", next, width = width)
                } else {
                    next.to_string()
                };
            }
        } else {
            self.advance(step);
        }
    }

    pub(super) fn render(&self) -> String {
        let tail = if self.width > 0 {
            format!("{:0width$}", self.current, width = self.width)
        } else {
            self.current.to_string()
        };
        if self.prefix.is_empty() {
            tail
        } else {
            let mut out = String::new();
            for (idx, part) in self.prefix.iter().enumerate() {
                out.push_str(part);
                out.push(*self.separators.get(idx).unwrap_or(&'.'));
            }
            out.push_str(&tail);
            out
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct ParsedAutonumber {
    pub(super) resume_only: bool,
    pub(super) start: Option<AutonumberCounter>,
    pub(super) step: Option<u64>,
    pub(super) format: Option<String>,
    pub(super) increment_level: Option<usize>,
}

pub(super) fn parse_autonumber_command(raw: &str) -> ParsedAutonumber {
    let mut parsed = ParsedAutonumber::default();
    let mut rest = raw.trim();

    if rest.eq_ignore_ascii_case("resume") {
        parsed.resume_only = true;
        return parsed;
    }

    if rest
        .get(..4)
        .is_some_and(|head| head.eq_ignore_ascii_case("inc "))
    {
        let level = &rest[4..];
        parsed.increment_level = autonumber_increment_level(level.trim());
        return parsed;
    }

    if let Some(tail) = rest.strip_prefix("resume ") {
        parsed.resume_only = true;
        rest = tail.trim_start();
    }

    if let Some((format, before)) = trailing_quoted_format(rest) {
        parsed.format = Some(format);
        rest = before.trim_end();
    }

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut idx = 0usize;
    if parsed.resume_only {
        if let Some(token) = tokens.get(idx) {
            if let Ok(step) = token.parse::<u64>() {
                parsed.step = Some(step);
                idx += 1;
            }
        }
    } else {
        if let Some(token) = tokens.get(idx) {
            if let Some(counter) = AutonumberCounter::from_token(token) {
                parsed.start = Some(counter);
                idx += 1;
            }
        }
        if let Some(token) = tokens.get(idx) {
            if let Ok(step) = token.parse::<u64>() {
                parsed.step = Some(step);
                idx += 1;
            }
        }
    }

    if parsed.format.is_none() {
        parsed.format = tokens.get(idx).map(|part| (*part).to_string());
    }

    parsed
}

pub(super) fn autonumber_increment_level(raw: &str) -> Option<usize> {
    let ch = raw.trim().chars().next()?;
    if !ch.is_ascii_alphabetic() {
        return None;
    }
    Some((ch.to_ascii_uppercase() as u8 - b'A' + 1) as usize)
}

pub(super) fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

pub(super) fn format_autonumber(counter: &AutonumberCounter, format: Option<&str>) -> String {
    let Some(format) = format else {
        return counter.render();
    };
    let fmt = format.trim();
    if fmt.is_empty() {
        return counter.render();
    }

    if fmt.contains('#') {
        return replace_hash_runs(fmt, counter.current);
    }

    let mut longest_zero_run = 0usize;
    let mut run_start = 0usize;
    let bytes = fmt.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'0' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'0' {
                i += 1;
            }
            let len = i - start;
            if len > longest_zero_run {
                longest_zero_run = len;
                run_start = start;
            }
            continue;
        }
        i += 1;
    }

    if longest_zero_run == 0 {
        return format!("{fmt}{}", counter.current);
    }

    let padded = format!("{:0width$}", counter.current, width = longest_zero_run);
    let prefix = &fmt[..run_start];
    let suffix = &fmt[run_start + longest_zero_run..];
    format!("{prefix}{padded}{suffix}")
}

pub(super) fn replace_hash_runs(format: &str, value: u64) -> String {
    let mut out = String::with_capacity(format.len() + 8);
    let bytes = format.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'#' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i] == b'#' {
            i += 1;
        }
        let width = i - start;
        if width > 1 {
            out.push_str(&format!("{:0width$}", value, width = width));
        } else {
            out.push_str(&value.to_string());
        }
    }
    out
}
