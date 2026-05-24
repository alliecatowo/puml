#[derive(Clone)]
enum SimpleRegexAtom {
    Any,
    Literal(char),
    Whitespace,
    Digit,
    Word,
    Class(Vec<(char, char)>, bool),
}

#[derive(Clone)]
struct SimpleRegexPart {
    atom: SimpleRegexAtom,
    min: usize,
    max: Option<usize>,
}

pub(super) fn split_preprocessor_regex(s: &str, pattern: &str) -> Vec<String> {
    if pattern.is_empty() {
        return vec![s.to_string()];
    }
    let Some(parts) = parse_simple_regex(pattern) else {
        return s.split(pattern).map(str::to_string).collect();
    };
    let chars = s.chars().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if let Some(len) = match_simple_regex_at(&chars, i, &parts, 0) {
            if len > 0 {
                fields.push(chars[start..i].iter().collect());
                i += len;
                start = i;
                continue;
            }
        }
        i += 1;
    }
    fields.push(chars[start..].iter().collect());
    fields
}

fn parse_simple_regex(pattern: &str) -> Option<Vec<SimpleRegexPart>> {
    let chars = pattern.chars().collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let atom = match chars[i] {
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    return None;
                }
                match chars[i] {
                    's' => SimpleRegexAtom::Whitespace,
                    'd' => SimpleRegexAtom::Digit,
                    'w' => SimpleRegexAtom::Word,
                    other => SimpleRegexAtom::Literal(other),
                }
            }
            '[' => {
                let (atom, next) = parse_simple_regex_class(&chars, i + 1)?;
                i = next;
                atom
            }
            '.' => SimpleRegexAtom::Any,
            '|' | '(' | ')' | '{' | '}' => return None,
            other => SimpleRegexAtom::Literal(other),
        };
        i += 1;
        let (min, max) = if i < chars.len() {
            match chars[i] {
                '+' => {
                    i += 1;
                    (1, None)
                }
                '*' => {
                    i += 1;
                    (0, None)
                }
                '?' => {
                    i += 1;
                    (0, Some(1))
                }
                _ => (1, Some(1)),
            }
        } else {
            (1, Some(1))
        };
        parts.push(SimpleRegexPart { atom, min, max });
    }
    Some(parts)
}

fn parse_simple_regex_class(chars: &[char], mut i: usize) -> Option<(SimpleRegexAtom, usize)> {
    let mut negated = false;
    if i < chars.len() && chars[i] == '^' {
        negated = true;
        i += 1;
    }
    let mut ranges = Vec::new();
    while i < chars.len() && chars[i] != ']' {
        let start = if chars[i] == '\\' {
            i += 1;
            if i >= chars.len() {
                return None;
            }
            chars[i]
        } else {
            chars[i]
        };
        if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
            let end = chars[i + 2];
            ranges.push((start, end));
            i += 3;
        } else {
            ranges.push((start, start));
            i += 1;
        }
    }
    if i >= chars.len() || chars[i] != ']' {
        return None;
    }
    Some((SimpleRegexAtom::Class(ranges, negated), i))
}

fn match_simple_regex_at(
    chars: &[char],
    pos: usize,
    parts: &[SimpleRegexPart],
    part_idx: usize,
) -> Option<usize> {
    if part_idx >= parts.len() {
        return Some(0);
    }
    let part = &parts[part_idx];
    let mut max_count = 0usize;
    while pos + max_count < chars.len()
        && part.max.map(|max| max_count < max).unwrap_or(true)
        && simple_regex_atom_matches(&part.atom, chars[pos + max_count])
    {
        max_count += 1;
    }
    if max_count < part.min {
        return None;
    }
    for count in (part.min..=max_count).rev() {
        if let Some(rest) = match_simple_regex_at(chars, pos + count, parts, part_idx + 1) {
            return Some(count + rest);
        }
    }
    None
}

fn simple_regex_atom_matches(atom: &SimpleRegexAtom, ch: char) -> bool {
    match atom {
        SimpleRegexAtom::Any => true,
        SimpleRegexAtom::Literal(lit) => *lit == ch,
        SimpleRegexAtom::Whitespace => ch.is_whitespace(),
        SimpleRegexAtom::Digit => ch.is_ascii_digit(),
        SimpleRegexAtom::Word => ch.is_ascii_alphanumeric() || ch == '_',
        SimpleRegexAtom::Class(ranges, negated) => {
            let matched = ranges.iter().any(|(start, end)| *start <= ch && ch <= *end);
            matched ^ *negated
        }
    }
}
