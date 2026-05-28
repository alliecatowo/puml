/// Creole inline text formatting for PlantUML labels.
///
/// Supports: **bold**, //italic//, ""mono"", __underline__, --strikethrough--,
/// ~~wave underline~~, [[url label]] hyperlinks, <color:X>text</color>,
/// <size:N>text</size>, legacy HTML-style tags, \n line breaks, basic block
/// Creole line forms (lists, headings, horizontal rules, tilde escape), and
/// <&icon> placeholders.
mod inline;
mod inline_helpers;
mod parser;
mod svg;
#[cfg(test)]
mod tests;

pub use crate::text_markup::decode_unicode_escapes;
pub use svg::{render_creole_line_to_tspans, render_creole_to_svg_tspans};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreoleSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub underline: bool,
    pub strike: bool,
    pub wave: bool,
    /// Set to `true` for a horizontal-rule sentinel span.  The SVG renderer
    /// emits an `<line>` element instead of a `<tspan>` for these spans.
    pub is_hr: bool,
    pub color: Option<String>,
    pub background: Option<String>,
    pub size: Option<u32>,
    pub font: Option<String>,
    pub baseline_shift: Option<String>,
    pub decoration_color: Option<String>,
    pub link: Option<String>,
    pub link_tooltip: Option<String>,
}

/// A line is a list of spans. Multiple lines come from \n splits.
pub type CreoleLine = Vec<CreoleSpan>;

/// Tokenize `text` into lines of spans.
///
/// Line breaks come from:
///   - literal `\n` characters in the string
///   - the two-character sequence `\\n` (backslash + n) in the source
///   - `<br>` / `<br/>` tags
pub fn tokenize_creole(text: &str) -> Vec<CreoleLine> {
    // First normalize line-break representations into real '\n'.
    let normalized = parser::normalize_line_breaks(text);

    let mut all_lines: Vec<CreoleLine> = Vec::new();
    for raw_line in normalized.split('\n') {
        all_lines.push(tokenize_creole_line(raw_line));
    }
    all_lines
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn tokenize_creole_line(raw_line: &str) -> CreoleLine {
    parser::parse_block_line(raw_line)
}
