use crate::render::text_metrics::{chunk_text, default_monospace_width};
use crate::scene::{Label, LayoutOptions, TextOverflowPolicy};
use crate::theme::MessageAlign;

use super::{
    GROUP_BOTTOM_PADDING, GROUP_HEADER_BASELINE_Y, GROUP_REF_BODY_BASELINE_Y, GROUP_TEXT_INSET_X,
    METADATA_BLOCK_PADDING, METADATA_LINE_HEIGHT, TEXT_LINE_HEIGHT,
};

pub(super) fn metadata_label_block_height(label: Option<&Label>) -> i32 {
    label
        .map(|label| metadata_lines_block_height(Some(&label.lines)))
        .unwrap_or(0)
}

pub(super) fn message_label_top_clearance(
    label_lines: &[String],
    is_parallel: bool,
    response_message_below_arrow: bool,
    arrow: &str,
) -> i32 {
    if label_lines.is_empty()
        || is_parallel
        || (response_message_below_arrow && is_response_message_arrow(arrow))
    {
        return 0;
    }
    ((label_lines.len() as i32 - 1) * TEXT_LINE_HEIGHT) + 8
}

pub(super) fn is_response_message_arrow(arrow: &str) -> bool {
    arrow.contains("--")
}

pub(super) fn metadata_block_right_edge(label: &Option<Label>, margin: i32) -> i32 {
    label
        .as_ref()
        .map(|label| {
            label
                .lines
                .iter()
                .map(|line| label.x + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(super) fn metadata_lines_right_edge(lines: Option<&Vec<String>>, margin: i32) -> i32 {
    lines
        .map(|lines| {
            lines
                .iter()
                .map(|line| margin + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(super) fn metadata_label_x(
    align: crate::model::MetadataHAlign,
    width: i32,
    margin: i32,
) -> i32 {
    match align {
        crate::model::MetadataHAlign::Left => margin,
        crate::model::MetadataHAlign::Center => width / 2,
        crate::model::MetadataHAlign::Right => width - margin,
    }
}

pub(super) fn metadata_lines_block_height(lines: Option<&Vec<String>>) -> i32 {
    lines
        .map(|lines| (lines.len() as i32 * METADATA_LINE_HEIGHT) + METADATA_BLOCK_PADDING)
        .unwrap_or(0)
}

pub(super) fn normalize_label_lines(
    text: &str,
    max_chars: usize,
    policy: TextOverflowPolicy,
) -> Vec<String> {
    match policy {
        TextOverflowPolicy::EllipsisSingleLine => {
            let one_line = text.replace('\n', " ");
            vec![ellipsize(&one_line, max_chars)]
        }
        TextOverflowPolicy::WrapAndGrow => text
            .lines()
            .flat_map(|line| wrap_line(line, max_chars))
            .collect::<Vec<_>>(),
    }
}

/// Count the *visual* (display) characters in a word, stripping creole/HTML
/// markup tags so that `<color:red>`, `</color>`, `<size:18>`, `</size>`,
/// `<b>`, `</b>`, `<i>`, `</i>`, `<u>`, `</u>`, `<&icon>`, etc. do not
/// inflate the character count used for line-wrapping decisions.
///
/// Bare `&name` OpenIconic sprite references (e.g. `&cloud-upload`) are
/// counted as 2 visual characters so they stay atomic during word-wrapping
/// and are never split mid-token by `chunk_text`.
///
/// `[[url label]]` hyperlink tokens are counted as the visible label length
/// only so the wrap budget reflects what the user sees, not the raw URL.
/// A bare `[[url]]` with no label counts as 4 placeholder chars.
pub(super) fn visual_char_count(word: &str) -> usize {
    // Handle [[url]] and [[url label]] tokens atomically before the character loop.
    if let Some(inner) = word.strip_prefix("[[").and_then(|s| s.strip_suffix("]]")) {
        if let Some(space_pos) = inner.find(" ") {
            return inner[space_pos + 1..].chars().count().max(1);
        }
        return 4;
    }
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    let mut count = 0;
    let mut i = 0;
    while i < len {
        if chars[i] == '<' {
            // Try to skip a markup tag: collect up to '>'.
            let mut j = i + 1;
            // Allow longer sprite references such as `<$name{scale=2,color=#2563eb}>`
            // to stay atomic during wrapping; splitting inside them prevents render-time
            // sprite substitution.
            let tag_limit = if i + 1 < len && chars[i + 1] == '$' {
                96
            } else {
                32
            };
            // Allow at most tag_limit chars inside the tag to avoid consuming large
            // non-tag `<...` sequences (e.g. math operators).
            while j < len && j - i <= tag_limit && chars[j] != '>' {
                j += 1;
            }
            if j < len && chars[j] == '>' {
                if i + 1 < len && chars[i + 1] == '$' {
                    count += 2;
                }
                // Consumed a tag — skip it entirely (or as compact sprite width).
                i = j + 1;
                continue;
            }
            // No closing '>' found within limit — treat '<' as a visual char.
            count += 1;
            i += 1;
        } else if chars[i] == '&' {
            // Bare `&name` OpenIconic sprite reference: `&[a-zA-Z0-9_-]+`.
            // Count the whole token as 2 visual chars (like a `<$...>` sprite)
            // so that `wrap_line` never chunks the token mid-name.
            let mut j = i + 1;
            while j < len && (chars[j].is_ascii_alphanumeric() || matches!(chars[j], '-' | '_')) {
                j += 1;
            }
            if j > i + 1 {
                // At least one alphanumeric char follows '&' — treat as sprite token.
                count += 2;
                i = j;
            } else {
                count += 1;
                i += 1;
            }
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

/// Tokenise `line` for word-wrap, keeping `[[...]]` hyperlink tokens atomic.
/// Spaces inside `[[...]]` must not become word-break points — the creole
/// inline parser requires the full `[[url label]]` syntax on one line to
/// recognise it as a hyperlink.  Splitting produces raw `[[url` text on one
/// line and `label]]` on the next, causing `//` in the URL to be misread as
/// italic markup.
fn split_for_wrap(line: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Skip whitespace between tokens
        if bytes[i] == b' ' || bytes[i] == b'\t' {
            i += 1;
            continue;
        }
        // Detect start of a [[...]] hyperlink token
        if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i;
            i += 2;
            while i + 1 < len {
                if bytes[i] == b']' && bytes[i + 1] == b']' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            // If closing `]]` was never found, i walked to len — still push.
            tokens.push(line[start..i].to_string());
        } else {
            // Regular whitespace-delimited word
            let start = i;
            while i < len && bytes[i] != b' ' && bytes[i] != b'\t' {
                i += 1;
            }
            tokens.push(line[start..i].to_string());
        }
    }
    tokens
}

pub(super) fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let word_strings = split_for_wrap(line);
    if word_strings.is_empty() {
        return vec![String::new()];
    }
    let words: Vec<&str> = word_strings.iter().map(|s| s.as_str()).collect();

    let mut lines = Vec::new();
    let mut current = String::new();
    // Track the *visual* length of `current` separately from its raw length.
    let mut current_visual: usize = 0;
    for word in words {
        let word_visual = visual_char_count(word);
        if current.is_empty() {
            if word_visual <= max_chars {
                current.push_str(word);
                current_visual = word_visual;
            } else {
                // Word is visually longer than max_chars.  If it contains
                // markup (visual_len < raw len) keep it whole rather than
                // splitting mid-tag; otherwise chunk it the old way.
                let word_raw = word.chars().count();
                if word_visual < word_raw {
                    // Contains markup — don't chunk it.
                    current.push_str(word);
                    current_visual = word_visual;
                } else {
                    for chunk in chunk_text(word, max_chars) {
                        lines.push(chunk);
                    }
                }
            }
            continue;
        }

        let next_visual = current_visual + 1 + word_visual;
        if next_visual <= max_chars {
            current.push(' ');
            current.push_str(word);
            current_visual = next_visual;
        } else {
            lines.push(current);
            let word_raw = word.chars().count();
            if word_visual <= max_chars {
                current = word.to_string();
                current_visual = word_visual;
            } else if word_visual < word_raw {
                // Contains markup — keep whole.
                current = word.to_string();
                current_visual = word_visual;
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current_visual = visual_char_count(&tail);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    debug_assert!(!lines.is_empty());
    lines
}

pub(super) fn ellipsize(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars - 1) {
        out.push(ch);
    }
    out.push('…');
    out
}

pub(super) fn multiline_metrics(text: &str) -> (i32, i32) {
    let mut max_width = 0;
    let mut lines = 0;
    for line in text.split('\n') {
        max_width = max_width.max(estimate_text_px_width(line));
        lines += 1;
    }
    (max_width, lines)
}

pub(super) fn group_content_min_size(kind: &str, label: Option<&str>) -> (i32, i32) {
    if kind.eq_ignore_ascii_case("box") {
        let min_width = label
            .map(|label| estimate_text_px_width(label) + (GROUP_TEXT_INSET_X * 2))
            .unwrap_or(0);
        return (min_width, 0);
    }
    let Some(label) = label else {
        return (0, 0);
    };
    let mut lines = label.split('\n');
    let header = lines.next().unwrap_or("");
    let header_text = format!("{kind} {header}");
    let mut max_width = estimate_text_px_width(header_text.trim());
    let mut height = GROUP_HEADER_BASELINE_Y + GROUP_BOTTOM_PADDING;

    if kind.eq_ignore_ascii_case("ref") {
        // The first line of the ref label is the participant spec (`over A, B`);
        // it is NOT rendered as body text (PlantUML parity — only body lines
        // after the `over ...` line appear inside the ref box).  Height and
        // width are computed from body-text lines only.
        let mut body_lines = 0i32;
        for line in lines {
            let lower = line.trim().to_ascii_lowercase();
            if lower.starts_with("over ") || lower == "over" {
                continue;
            }
            max_width = max_width.max(estimate_text_px_width(line));
            body_lines += 1;
        }
        // GROUP_REF_BODY_BASELINE_Y reserves space for the header notch + one line.
        // Each additional body line adds TEXT_LINE_HEIGHT.
        height = GROUP_REF_BODY_BASELINE_Y
            + (body_lines.saturating_sub(1).max(0) * TEXT_LINE_HEIGHT)
            + GROUP_BOTTOM_PADDING;
    }

    (max_width + (GROUP_TEXT_INSET_X * 2), height)
}

pub(super) fn else_separator_label(label: Option<&str>) -> String {
    match label.map(str::trim).filter(|label| !label.is_empty()) {
        Some(label) => format!("else {label}"),
        None => "else".to_string(),
    }
}

pub(super) fn estimate_text_px_width(line: &str) -> i32 {
    default_monospace_width(line)
}

pub(super) fn message_label_bounds(
    x1: i32,
    x2: i32,
    text_width: i32,
    align: MessageAlign,
) -> (i32, i32) {
    let left = x1.min(x2);
    let right = x1.max(x2);
    match align {
        MessageAlign::Left => {
            let anchor = left + 8;
            (anchor, anchor + text_width)
        }
        MessageAlign::Center => {
            let anchor = ((x1 + x2) / 2) + 2;
            (anchor - (text_width / 2), anchor + ((text_width + 1) / 2))
        }
        MessageAlign::Right => {
            let anchor = right - 8;
            (anchor - text_width, anchor)
        }
    }
}

pub(super) fn legend_box_size(text: &str) -> (i32, i32) {
    let lines = text.lines().collect::<Vec<_>>();
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| estimate_text_px_width(line))
        .max()
        .unwrap_or(0);
    let width = (max_line_width + 16).max(200);
    let height = 24 + (line_count * 16);
    (width, height)
}

pub(super) fn message_label_lines(
    label: Option<&str>,
    x1: i32,
    x2: i32,
    sequence_message_span: bool,
    options: &LayoutOptions,
) -> Vec<String> {
    let Some(label) = label else {
        return Vec::new();
    };
    let min_span = (options.participant_spacing - 20).max(56);
    let span_px = if sequence_message_span {
        (options.participant_spacing * 2).max((x2 - x1).abs())
    } else {
        (x2 - x1).abs().max(min_span) - 16
    };
    let tx = ((x1 + x2) / 2) + 2;
    let max_chars_by_span = (span_px / 7).max(1) as usize;
    let max_chars_by_left_edge = ((tx * 2) / 7).max(1) as usize;
    let mut max_chars = max_chars_by_span.min(max_chars_by_left_edge);
    if starts_with_autonumber_prefix(label) {
        max_chars = max_chars.saturating_add(4);
    }
    normalize_label_lines(label, max_chars, options.text_overflow_policy)
}

pub(super) fn starts_with_autonumber_prefix(label: &str) -> bool {
    let Some(first) = label.split_whitespace().next() else {
        return false;
    };
    (first.contains('.')
        && first
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
        || (first.contains('-') && first.bytes().any(|b| b.is_ascii_digit()))
}

pub(super) fn row_units_for_height(height: i32, row_height: i32) -> i32 {
    if row_height <= 0 {
        return 1;
    }
    ((height + row_height - 1) / row_height).max(1)
}
