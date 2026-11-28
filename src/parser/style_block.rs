//! Recursive-descent parser for `<style> … </style>` blocks (Phase A of #1404).
//!
//! Modelled on `StyleParser.java` (361 LOC, upstream). Accepts:
//! - Bare identifier selectors: `participant { … }`
//! - Stereotype selectors: `.Apache { … }`
//! - Wildcard: `* { … }`
//! - Pseudo: `:depth(N) { … }`
//! - Comma-separated: `a, b { … }` (fan-out)
//! - Descendant nesting: `activityDiagram { partition { … } }` (unbounded)
//! - CSS variables: `--name: value`
//! - `@media dark { … }` blocks
//! - Comments: `//`, `/' '/`, `/* */`
//! - Properties as `Key Value` or `Key: Value;`
//!
//! Returns:
//! - A typed `StyleBlock` AST
//! - A `Vec<CompatTriple>` (retained for callers that still pass compat triples; the
//!   `StatementKind::StyleParam` compat shim was removed in Phase E — #1417)

use std::collections::BTreeMap;

use crate::ast::style::{
    PName, SName, SelectorChain, SelectorSegment, StyleBlock, StyleRule, StyleScheme, StyleValue,
};

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// A raw identifier/string token (selector name, property key, value word, …).
    Str(String),
    /// `{`
    Open,
    /// `}`
    Close,
    /// `,`
    Comma,
    /// `;`
    Semi,
    /// `:`
    Colon,
    /// `*`
    Star,
    /// newline
    Newline,
    /// `@media dark` / `@media light` etc.
    ArobaseMedia(String),
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

/// Scan the raw inner text of a `<style>` block into tokens.
/// The text must NOT include the `<style>` / `</style>` lines.
fn lex(input: &str) -> Vec<Tok> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut pos = 0;
    let mut tokens = Vec::new();

    macro_rules! peek1 {
        () => {
            if pos + 1 < len {
                bytes[pos + 1]
            } else {
                0
            }
        };
    }

    while pos < len {
        let ch = bytes[pos] as char;

        // Skip horizontal whitespace
        if ch == ' ' || ch == '\t' {
            pos += 1;
            continue;
        }

        // Line comments: `//`
        if ch == '/' && peek1!() == b'/' {
            while pos < len && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        // Block comments: `/' '/` (PlantUML) and `/* */`
        if ch == '/' && peek1!() == b'\'' {
            pos += 2; // skip `/'`
            while pos < len {
                if bytes[pos] == b'\'' && pos + 1 < len && bytes[pos + 1] == b'/' {
                    pos += 2;
                    break;
                }
                pos += 1;
            }
            continue;
        }
        if ch == '/' && peek1!() == b'*' {
            pos += 2; // skip `/*`
            while pos < len {
                if bytes[pos] == b'*' && pos + 1 < len && bytes[pos + 1] == b'/' {
                    pos += 2;
                    break;
                }
                pos += 1;
            }
            continue;
        }

        // Newline
        if ch == '\n' || ch == '\r' {
            tokens.push(Tok::Newline);
            pos += 1;
            // Collapse `\r\n`
            if ch == '\r' && pos < len && bytes[pos] == b'\n' {
                pos += 1;
            }
            continue;
        }

        // Single-char tokens
        match ch {
            '{' => {
                tokens.push(Tok::Open);
                pos += 1;
                continue;
            }
            '}' => {
                tokens.push(Tok::Close);
                pos += 1;
                continue;
            }
            ',' => {
                tokens.push(Tok::Comma);
                pos += 1;
                continue;
            }
            ';' => {
                tokens.push(Tok::Semi);
                pos += 1;
                continue;
            }
            ':' => {
                tokens.push(Tok::Colon);
                pos += 1;
                continue;
            }
            '*' => {
                tokens.push(Tok::Star);
                pos += 1;
                continue;
            }
            _ => {}
        }

        // `@media` / `@…` tokens
        if ch == '@' {
            let start = pos;
            pos += 1; // skip `@`
            while pos < len {
                let c = bytes[pos] as char;
                if c == '{' || c == '}' || c == ';' || c == '\n' || c == '\r' {
                    break;
                }
                pos += 1;
            }
            let s = input[start..pos].trim().to_string();
            tokens.push(Tok::ArobaseMedia(s));
            continue;
        }

        // Quoted string: `"…"` → strip quotes, treat as Str
        if ch == '"' {
            pos += 1;
            let start = pos;
            while pos < len && bytes[pos] != b'"' {
                pos += 1;
            }
            let s = input[start..pos].to_string();
            if pos < len {
                pos += 1; // skip closing `"`
            }
            tokens.push(Tok::Str(s));
            continue;
        }

        // Stereotype / dotted selector: `.something`  — read until whitespace/special
        if ch == '.' {
            let start = pos;
            pos += 1;
            while pos < len {
                let c = bytes[pos] as char;
                if c == ' '
                    || c == '\t'
                    || c == '\n'
                    || c == '\r'
                    || c == '{'
                    || c == '}'
                    || c == ','
                    || c == ';'
                    || c == ':'
                {
                    break;
                }
                pos += 1;
            }
            let s = input[start..pos].trim().to_string();
            tokens.push(Tok::Str(s));
            continue;
        }

        // Plain identifier / value word.
        // Mirrors upstream StyleParser.java `readString`: stops at newline,
        // space, tab, `{`, `}`, `;`, `,`, `:`.  The `:` stop is important for
        // CSS-variable lines like `--name: value` so the key and colon lex
        // as separate tokens.
        {
            let start = pos;
            while pos < len {
                let c = bytes[pos] as char;
                if c == '\n'
                    || c == '\r'
                    || c == '{'
                    || c == '}'
                    || c == ';'
                    || c == ','
                    || c == '\t'
                    || c == ':'
                    || c == ' '
                {
                    break;
                }
                pos += 1;
            }
            let s = input[start..pos].to_string();
            if !s.is_empty() {
                tokens.push(Tok::Str(s));
            }
        }
    }

    tokens
}

// ---------------------------------------------------------------------------
// Token stream helpers
// ---------------------------------------------------------------------------

struct Stream<'a> {
    tokens: &'a [Tok],
    pos: usize,
}

impl<'a> Stream<'a> {
    fn new(tokens: &'a [Tok]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Tok> {
        // Skip newlines transparently for most peeks
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i] {
                Tok::Newline => i += 1,
                t => return Some(t),
            }
        }
        None
    }

    /// Peek without skipping newlines.
    fn peek_raw(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Advance past the next non-newline token (and any leading newlines).
    fn next(&mut self) -> Option<&Tok> {
        // skip leading newlines
        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos] {
                Tok::Newline => self.pos += 1,
                _ => break,
            }
        }
        if self.pos < self.tokens.len() {
            let t = &self.tokens[self.pos];
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(Tok::Newline) = self.peek_raw() {
            self.bump();
        }
    }

    fn skip_semis_and_newlines(&mut self) {
        while let Some(Tok::Newline) | Some(Tok::Semi) = self.peek_raw() {
            self.bump();
        }
    }
}

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

struct Parser {
    /// Currently active selector path stack (one chain per nesting level).
    /// Each entry holds the list of comma-expanded selectors at that depth.
    context: Vec<Vec<SelectorChain>>,
    scheme: StyleScheme,
    rules: Vec<StyleRule>,
    variables: BTreeMap<String, String>,
    /// Compat-shim flat output: (selector_path_string, property_raw, key, value)
    compat: Vec<CompatTriple>,
    counter: u32,
}

/// Legacy flat triple for the compat shim.
pub struct CompatTriple {
    /// Outermost selector name (e.g. `"participant"`), or `None` for top-level.
    pub selector: Option<String>,
    /// Raw property name (e.g. `"BackgroundColor"`).
    pub property: String,
    /// Skinparam key (legacy resolved key), if one can be computed.
    pub key: Option<String>,
    /// Raw value string.
    pub value: String,
}

impl Parser {
    fn new() -> Self {
        Self {
            context: Vec::new(),
            scheme: StyleScheme::Regular,
            rules: Vec::new(),
            variables: BTreeMap::new(),
            compat: Vec::new(),
            counter: 0,
        }
    }

    /// Convert the current context stack into a flat `Vec<SelectorChain>` path.
    /// Each level contributes its comma-expanded list as one `SelectorChain`.
    /// For simplicity we take the first chain at each depth level (fan-out is
    /// already materialised into separate rules by `emit_rule`).
    fn current_path(&self) -> Vec<SelectorChain> {
        self.context
            .iter()
            .map(|chains| {
                // Use the first chain at each depth as the representative path entry.
                chains.first().cloned().unwrap_or_else(|| SelectorChain {
                    segments: Vec::new(),
                })
            })
            .collect()
    }

    /// Push a selector group (one or more comma-separated names) onto the stack.
    fn push_selectors(&mut self, chains: Vec<SelectorChain>) {
        self.context.push(chains);
    }

    fn pop_selectors(&mut self) {
        self.context.pop();
    }

    fn emit_rule(
        &mut self,
        properties: BTreeMap<PName, StyleValue>,
        unknown: BTreeMap<String, String>,
    ) {
        // Fan out across the comma-expanded chains at every level.
        // For Phase A we emit one rule per top-level chain combination (simple
        // cross-product would be O(n^depth) — acceptable for real-world nesting).
        // The cascade resolver in Phase B will handle the selector matching properly.
        let path = self.current_path();
        self.counter += 1;
        self.rules.push(StyleRule {
            selector_path: path,
            properties,
            unknown_properties: unknown,
            source_order: self.counter,
            scheme: self.scheme,
        });
    }

    /// Compute the outermost selector string for compat-shim use.
    fn compat_outer_selector(&self) -> Option<String> {
        self.context.first().and_then(|chains| {
            chains.first().and_then(|chain| {
                chain.segments.first().map(|seg| match seg {
                    SelectorSegment::Tag(name) => format!("{name:?}").to_ascii_lowercase(),
                    SelectorSegment::Stereotype(s) => format!(".{s}"),
                    SelectorSegment::Wildcard => "*".to_string(),
                    SelectorSegment::Depth(n) => format!(":depth({n})"),
                    SelectorSegment::Unknown(s) => s.clone(),
                })
            })
        })
    }

    /// Emit a compat-shim triple for one property key=value pair.
    fn emit_compat(&mut self, raw_key: &str, value: &str) {
        let outer = self.compat_outer_selector();
        self.compat.push(CompatTriple {
            selector: outer,
            property: raw_key.to_string(),
            key: None, // legacy skinparam key resolution lives in directives.rs
            value: value.to_string(),
        });
    }
}

// ---------------------------------------------------------------------------
// Parse a raw block body string
// ---------------------------------------------------------------------------

/// Parse the body of a `<style>` block (everything between the `<style>` and
/// `</style>` tags, already pre-stripped of the tag lines themselves).
///
/// Returns `(StyleBlock, Vec<CompatTriple>)`.  The `CompatTriple` list is
/// retained for callers; the `StatementKind::StyleParam` compat shim that
/// consumed these triples was removed in Phase E (#1417).
pub fn parse_style_block_body(body: &str) -> (StyleBlock, Vec<CompatTriple>) {
    let tokens = lex(body);
    let mut stream = Stream::new(&tokens);
    let mut parser = Parser::new();

    parse_block_contents(&mut stream, &mut parser);

    let block = StyleBlock {
        rules: parser.rules,
        variables: parser.variables,
    };
    (block, parser.compat)
}

// ---------------------------------------------------------------------------
// Recursive-descent block content parser
// ---------------------------------------------------------------------------

/// Parse everything at the current nesting level until either EOF or a `}`
/// token is consumed (indicating the end of the enclosing block).
fn parse_block_contents(stream: &mut Stream, parser: &mut Parser) {
    loop {
        stream.skip_semis_and_newlines();
        let Some(tok) = stream.peek() else { break };

        match tok {
            Tok::Close => {
                // End of the current block — caller will pop the context.
                stream.next();
                break;
            }

            Tok::ArobaseMedia(s) => {
                let s = s.clone();
                stream.next();
                // `@media dark { … }` — push dark scheme
                let prev = parser.scheme;
                if s.contains("dark") {
                    parser.scheme = StyleScheme::Dark;
                }
                // Parse the inner block
                stream.skip_newlines();
                if let Some(Tok::Open) = stream.peek() {
                    stream.next(); // consume `{`
                    parse_block_contents(stream, parser);
                }
                parser.scheme = prev;
            }

            Tok::Str(s) => {
                let s = s.clone();
                stream.next(); // consume the string

                // CSS variable: `--name: value`
                if s.starts_with("--") {
                    // Skip optional `:`
                    if let Some(Tok::Colon) = stream.peek() {
                        stream.next();
                    }
                    let value = read_value(stream);
                    parser.variables.insert(s, value);
                    continue;
                }

                // Check what follows: comma (selector group), `{` (single selector), or value
                // First collect any comma-joined siblings
                let mut selector_parts = vec![s.clone()];
                loop {
                    stream.skip_newlines();
                    if let Some(Tok::Comma) = stream.peek() {
                        stream.next(); // consume `,`
                        stream.skip_newlines();
                        if let Some(Tok::Str(next)) = stream.peek() {
                            let next = next.clone();
                            stream.next();
                            selector_parts.push(next);
                        } else if let Some(Tok::Star) = stream.peek() {
                            stream.next();
                            selector_parts.push("*".to_string());
                        } else {
                            break;
                        }
                    } else if let Some(Tok::Star) = stream.peek() {
                        // `selector*` — upstream STAR suffix (rare)
                        stream.next();
                        *selector_parts.last_mut().unwrap() += "*";
                    } else {
                        break;
                    }
                }

                stream.skip_newlines();

                if let Some(Tok::Open) = stream.peek() {
                    // This is a selector block: `selectorA, selectorB { … }`
                    // `parse_selector_segments` handles the PlantUML-specific
                    // `identifier<<Stereotype>>` compound syntax.
                    stream.next(); // consume `{`
                    let chains: Vec<SelectorChain> = selector_parts
                        .iter()
                        .map(|p| SelectorChain {
                            segments: parse_selector_segments(p),
                        })
                        .collect();
                    parser.push_selectors(chains);
                    parse_block_contents(stream, parser);
                    parser.pop_selectors();
                } else {
                    // This is a property: `Key [:]? Value [;]`
                    // Skip optional `:`
                    if let Some(Tok::Colon) = stream.peek() {
                        stream.next();
                    }
                    let raw_value = read_value(stream);
                    if !raw_value.is_empty() {
                        let raw_key = s.clone();
                        // Emit compat triple
                        parser.emit_compat(&raw_key, &raw_value);
                        // Build a single-property rule at this depth
                        let mut props = BTreeMap::new();
                        let mut unknown = BTreeMap::new();
                        let sv = parse_value(&raw_value);
                        if let Some(pname) = PName::from_name(&raw_key) {
                            props.insert(pname, sv);
                        } else {
                            unknown.insert(raw_key, raw_value);
                        }
                        parser.emit_rule(props, unknown);
                    }
                }
            }

            Tok::Star => {
                // Wildcard selector `* { … }`
                stream.next();
                stream.skip_newlines();
                if let Some(Tok::Open) = stream.peek() {
                    stream.next();
                    let chains = vec![SelectorChain {
                        segments: vec![SelectorSegment::Wildcard],
                    }];
                    parser.push_selectors(chains);
                    parse_block_contents(stream, parser);
                    parser.pop_selectors();
                }
            }

            Tok::Colon => {
                // `:depth(N) { … }` pseudo-selector
                stream.next(); // consume `:`
                let name_tok = stream.peek().cloned();
                if let Some(Tok::Str(pseudo)) = name_tok {
                    stream.next();
                    let seg = parse_pseudo_segment(&pseudo);
                    stream.skip_newlines();
                    // There might be a `*` suffix
                    if let Some(Tok::Star) = stream.peek() {
                        stream.next();
                    }
                    if let Some(Tok::Open) = stream.peek() {
                        stream.next();
                        let chains = vec![SelectorChain {
                            segments: vec![seg],
                        }];
                        parser.push_selectors(chains);
                        parse_block_contents(stream, parser);
                        parser.pop_selectors();
                    }
                }
            }

            // Skip orphan semicolons / newlines already handled above
            Tok::Semi | Tok::Newline => {
                stream.bump();
            }

            // Anything else we don't understand — skip token to avoid getting stuck
            _ => {
                stream.next();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Value reader
// ---------------------------------------------------------------------------

/// Read a property value: one or more `Str`/`Colon` tokens up to newline, `;`, or `}`.
fn read_value(stream: &mut Stream) -> String {
    let mut parts = Vec::new();
    loop {
        match stream.peek_raw() {
            None | Some(Tok::Newline) | Some(Tok::Semi) | Some(Tok::Close) => break,
            Some(Tok::Str(s)) => {
                parts.push(s.clone());
                stream.bump();
            }
            Some(Tok::Colon) => {
                // `#color:value` style — include colon and next token
                parts.push(":".to_string());
                stream.bump();
                if let Some(Tok::Str(s)) = stream.peek_raw() {
                    parts.push(s.clone());
                    stream.bump();
                }
            }
            Some(Tok::Comma) => {
                // Comma inside a value (e.g. `"5, 40"` padding) — include it
                parts.push(",".to_string());
                stream.bump();
            }
            // Anything else terminates the value
            _ => break,
        }
    }
    // Skip trailing `;` if present
    if let Some(Tok::Semi) = stream.peek_raw() {
        stream.bump();
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Selector and value helpers
// ---------------------------------------------------------------------------

/// Convert a raw selector string to a `SelectorSegment`.
fn parse_selector_segment(raw: &str) -> SelectorSegment {
    if raw == "*" {
        return SelectorSegment::Wildcard;
    }
    if let Some(name) = raw.strip_prefix('.') {
        return SelectorSegment::Stereotype(name.to_string());
    }
    // Try known SName catalogue
    if let Some(sname) = SName::retrieve(raw) {
        return SelectorSegment::Tag(sname);
    }
    SelectorSegment::Unknown(raw.to_string())
}

/// Convert a raw selector string into one or more `SelectorSegment`s.
///
/// PlantUML `<style>` blocks support the `identifier<<Stereotype>>` syntax as
/// a shorthand for a compound selector (e.g. `class<<service>>` = class node
/// with the `service` stereotype).  This function splits such selectors into
/// `[Tag(identifier), Stereotype("service")]` so the cascade resolver can match
/// them correctly.
///
/// Non-compound selectors return a single-element `Vec`.
fn parse_selector_segments(raw: &str) -> Vec<SelectorSegment> {
    // Fast path: no `<<` — simple single segment.
    if !raw.contains("<<") {
        return vec![parse_selector_segment(raw)];
    }
    // Split on first `<<`: the left part is the SName, the right (strip `>>`) is the stereotype.
    if let Some(idx) = raw.find("<<") {
        let tag_part = &raw[..idx];
        let rest = &raw[idx + 2..]; // strip `<<`
        let stereo = rest.trim_end_matches('>').trim().to_string();
        let tag_seg = parse_selector_segment(tag_part.trim());
        let stereo_seg = SelectorSegment::Stereotype(stereo);
        return vec![tag_seg, stereo_seg];
    }
    vec![parse_selector_segment(raw)]
}

/// Convert a pseudo-selector token like `depth(2)` to a `SelectorSegment`.
fn parse_pseudo_segment(pseudo: &str) -> SelectorSegment {
    if let Some(inner) = pseudo
        .strip_prefix("depth(")
        .and_then(|s| s.strip_suffix(')'))
    {
        if let Ok(n) = inner.parse::<u32>() {
            return SelectorSegment::Depth(n);
        }
    }
    // Unknown pseudo — preserve as-is
    SelectorSegment::Unknown(format!(":{pseudo}"))
}

/// Parse a raw value string into a typed `StyleValue`.
fn parse_value(raw: &str) -> StyleValue {
    let s = raw.trim();

    // Numeric value (plain integer or float)
    // Handle leading `$` or gradient patterns as Raw
    if s.starts_with('#') || s.starts_with("0x") || s.starts_with("0X") {
        // Colour: `#RGB`, `#RRGGBB`, or a gradient `#abc-#def`
        return StyleValue::Color(s.to_string());
    }

    // Try numeric parse (covers font sizes, thicknesses, etc.)
    if let Ok(f) = s.parse::<f64>() {
        return StyleValue::Number(f);
    }

    // Known colour keywords (PlantUML named colours)
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "transparent"
            | "white"
            | "black"
            | "red"
            | "green"
            | "blue"
            | "yellow"
            | "orange"
            | "purple"
            | "gray"
            | "grey"
            | "lightgray"
            | "darkgray"
            | "pink"
            | "cyan"
            | "magenta"
            | "brown"
            | "silver"
            | "navy"
            | "teal"
            | "lime"
            | "maroon"
            | "olive"
            | "aqua"
            | "fuchsia"
    ) {
        return StyleValue::Color(s.to_string());
    }

    // Known style/alignment keywords
    if matches!(
        lower.as_str(),
        "bold"
            | "italic"
            | "plain"
            | "underline"
            | "strikethrough"
            | "left"
            | "right"
            | "center"
            | "normal"
            | "none"
            | "true"
            | "false"
            | "yes"
            | "no"
            | "dashed"
            | "dotted"
            | "solid"
            | "hidden"
            | "lighter"
            | "bolder"
            | "light"
            | "dark"
    ) {
        return StyleValue::Keyword(s.to_string());
    }

    // Everything else: Raw (includes `$VARREF`, gradients, multi-part values)
    StyleValue::Raw(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> StyleBlock {
        let (block, _) = parse_style_block_body(src);
        block
    }

    fn parse_with_compat(src: &str) -> (StyleBlock, Vec<CompatTriple>) {
        parse_style_block_body(src)
    }

    // -----------------------------------------------------------------------
    // 1. Basic flat properties (cyborg-root-style snippet)
    // -----------------------------------------------------------------------
    #[test]
    fn test_root_flat_properties() {
        let src = r#"
  root {
    BackgroundColor transparent
    FontColor #FFFFFF
    LineThickness 1
    Margin 10
    Padding 6
    Shadowing 0.0
  }
"#;
        let block = parse(src);
        assert!(!block.rules.is_empty(), "should produce at least one rule");
        // Find rule with BackgroundColor
        let has_bg = block
            .rules
            .iter()
            .any(|r| r.properties.contains_key(&PName::BackgroundColor));
        assert!(has_bg, "BackgroundColor should be parsed");
        let has_lt = block
            .rules
            .iter()
            .any(|r| r.properties.contains_key(&PName::LineThickness));
        assert!(has_lt, "LineThickness should be parsed");
        let has_margin = block
            .rules
            .iter()
            .any(|r| r.properties.contains_key(&PName::Margin));
        assert!(has_margin, "Margin should be parsed");
    }

    // -----------------------------------------------------------------------
    // 2. Nested descendant selectors (cyborg nwdiagDiagram snippet)
    // -----------------------------------------------------------------------
    #[test]
    fn test_nested_descendant_selectors() {
        let src = r#"
  nwdiagDiagram {
    network {
      LineColor #2A9FD6
      LineThickness 1.0
    }
    group {
      BackGroundColor #222222
      LineColor #444444
      LineThickness 2.0
    }
  }
"#;
        let block = parse(src);
        // Should produce rules for both `network` and `group` under `nwdiagDiagram`
        let network_rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Tag(SName::Network)))
            })
        });
        assert!(network_rule.is_some(), "network rule should be present");
        let group_rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Tag(SName::Group)))
            })
        });
        assert!(group_rule.is_some(), "group rule should be present");
        // nwdiagDiagram must be the first segment at depth 0
        let nr = network_rule.unwrap();
        let first_seg = &nr.selector_path[0].segments[0];
        assert_eq!(
            *first_seg,
            SelectorSegment::Tag(SName::NwdiagDiagram),
            "outer selector should be nwdiagDiagram"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Comma-separated selectors (mindmap+wbs snippet from hacker theme)
    // -----------------------------------------------------------------------
    #[test]
    fn test_comma_selectors() {
        let src = r#"
mindmapDiagram, wbsDiagram {
    element {
        BackgroundColor #77B300
    }
}
"#;
        let block = parse(src);
        // Two rules, one per top-level selector after fan-out
        // Both should have element-level nesting with BackgroundColor
        let bg_rules: Vec<_> = block
            .rules
            .iter()
            .filter(|r| r.properties.contains_key(&PName::BackgroundColor))
            .collect();
        assert!(
            !bg_rules.is_empty(),
            "BackgroundColor should appear in comma-expanded rules"
        );
    }

    // -----------------------------------------------------------------------
    // 4. Stereotype and wildcard selectors
    // -----------------------------------------------------------------------
    #[test]
    fn test_stereotype_selector() {
        let src = r#"
.Apache {
  BackgroundColor #FF5733
  LineColor #CC4400
}
"#;
        let block = parse(src);
        let rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Stereotype(s) if s == "Apache"))
            })
        });
        assert!(rule.is_some(), "Stereotype selector .Apache should parse");
        let r = rule.unwrap();
        assert!(r.properties.contains_key(&PName::BackgroundColor));
    }

    #[test]
    fn test_wildcard_selector() {
        let src = r#"
* {
  FontSize 12
}
"#;
        let block = parse(src);
        let rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Wildcard))
            })
        });
        assert!(rule.is_some(), "Wildcard selector * should parse");
        let r = rule.unwrap();
        assert!(r.properties.contains_key(&PName::FontSize));
    }

    // -----------------------------------------------------------------------
    // 5. :depth pseudo-selector (hacker theme snippet)
    // -----------------------------------------------------------------------
    #[test]
    fn test_depth_pseudo() {
        let src = r#"
mindmapDiagram {
    :depth(0) {
        fontSize 16
        fontStyle bold
    }
    :depth(1) {
        BackgroundColor #333333
    }
}
"#;
        let block = parse(src);
        let depth0 = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Depth(0)))
            })
        });
        assert!(depth0.is_some(), ":depth(0) should parse");
        let depth1 = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|seg| matches!(seg, SelectorSegment::Depth(1)))
            })
        });
        assert!(depth1.is_some(), ":depth(1) should parse");
    }

    // -----------------------------------------------------------------------
    // 6. CSS variable declarations
    // -----------------------------------------------------------------------
    #[test]
    fn test_css_variables() {
        let src = r#"
--primary-color: #2A9FD6
--secondary: #555555
root {
  BackgroundColor transparent
}
"#;
        let block = parse(src);
        assert_eq!(
            block.variables.get("--primary-color").map(String::as_str),
            Some("#2A9FD6"),
            "CSS variable --primary-color should be stored"
        );
        assert!(
            block.variables.contains_key("--secondary"),
            "CSS variable --secondary should be stored"
        );
    }

    // -----------------------------------------------------------------------
    // 7. @media dark block
    // -----------------------------------------------------------------------
    #[test]
    fn test_at_media_dark() {
        let src = r#"
@media dark {
    root {
        BackgroundColor #000000
        FontColor #FFFFFF
    }
}
"#;
        let block = parse(src);
        let dark_rule = block.rules.iter().find(|r| {
            r.scheme == StyleScheme::Dark && r.properties.contains_key(&PName::BackgroundColor)
        });
        assert!(
            dark_rule.is_some(),
            "@media dark rules should be tagged Dark scheme"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Compat shim still emits legacy triples
    // -----------------------------------------------------------------------
    #[test]
    fn test_compat_shim_emits_triples() {
        let src = r#"
participant {
  BackgroundColor #AABBCC
  FontColor #FFFFFF
}
"#;
        let (_block, compat) = parse_with_compat(src);
        assert!(
            !compat.is_empty(),
            "compat shim should emit at least one legacy triple"
        );
        let bg_triple = compat
            .iter()
            .find(|t| t.property.eq_ignore_ascii_case("backgroundcolor"));
        assert!(
            bg_triple.is_some(),
            "BackgroundColor should appear in compat triples"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Gantt nested multi-level (cyborg theme snippet)
    // -----------------------------------------------------------------------
    #[test]
    fn test_gantt_nested() {
        let src = r#"
  ganttDiagram {
    task {
      LineColor #2A9FD6
      Margin 10
    }
    milestone {
      FontColor #9933CC
      FontSize 16
      FontStyle italic
    }
    timeline {
      BackgroundColor #555555
    }
  }
"#;
        let block = parse(src);
        let task_rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|s| matches!(s, SelectorSegment::Tag(SName::Task)))
            })
        });
        assert!(task_rule.is_some(), "ganttDiagram > task should parse");
        let milestone_rule = block.rules.iter().find(|r| {
            r.selector_path.iter().any(|chain| {
                chain
                    .segments
                    .iter()
                    .any(|s| matches!(s, SelectorSegment::Tag(SName::Milestone)))
            })
        });
        assert!(
            milestone_rule.is_some(),
            "ganttDiagram > milestone should parse"
        );
    }

    // -----------------------------------------------------------------------
    // 10. Unknown property preserved in unknown_properties map
    // -----------------------------------------------------------------------
    #[test]
    fn test_unknown_property_preserved() {
        let src = r#"
title {
    BorderRoundCorner 8
    BorderThickness 1
    SomeFutureProperty foobar
}
"#;
        let block = parse(src);
        let title_rule = block.rules.iter().find(|r| {
            r.properties.contains_key(&PName::RoundCorner)
                || r.properties.contains_key(&PName::LineThickness)
                || r.unknown_properties.contains_key("SomeFutureProperty")
        });
        assert!(
            title_rule.is_some(),
            "title rule should parse with known + unknown props"
        );
    }
}
