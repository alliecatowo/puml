/// Specialized diagram renderers for @startmath and @startditaa diagram families.
///
/// These bypass the main AST parser pipeline and implement their own
/// mini-parsers and SVG renderers.
use crate::diagnostic::Diagnostic;

// ─── Public dispatch ──────────────────────────────────────────────────────────

/// Try to render `source` as one of the specialized diagram families.
/// Returns `Some(svg)` if the source is recognized, `None` otherwise.
pub fn try_render_specialized(source: &str) -> Option<Result<String, Diagnostic>> {
    let trimmed = source.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("@startmath") {
        Some(render_math(trimmed))
    } else if lower.starts_with("@startditaa") {
        Some(render_ditaa(trimmed))
    } else {
        None
    }
}

// ─── Shared SVG utilities ─────────────────────────────────────────────────────

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn svg_header(width: i32, height: i32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    )
}

fn svg_white_bg() -> &'static str {
    "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>"
}

/// Strip the @start.../@end... wrapper and return body lines and optional title.
fn strip_block<'a>(source: &'a str, start_tag: &str, end_tag: &str) -> (&'a str, Option<String>) {
    let mut lines = source.lines();
    let first_line = lines.next().unwrap_or("").trim();
    let tag_lower = first_line.to_ascii_lowercase();
    let rest_after_tag = if tag_lower.starts_with(start_tag) {
        first_line[start_tag.len()..].trim()
    } else {
        first_line
    };
    let title: Option<String> = if rest_after_tag.starts_with('"') {
        Some(rest_after_tag.trim_matches('"').to_string())
    } else if !rest_after_tag.is_empty() {
        Some(rest_after_tag.to_string())
    } else {
        None
    };

    let body_start = first_line.len() + 1;
    let body_end = if let Some(pos) = source.to_ascii_lowercase().rfind(end_tag) {
        let before = &source[..pos];
        before.rfind('\n').map(|i| i + 1).unwrap_or(0)
    } else {
        source.len()
    };

    let body = source
        .get(body_start.min(source.len())..body_end.min(source.len()))
        .unwrap_or("");
    (body, title)
}

// ─── Family 1: @startmath ─────────────────────────────────────────────────────
//
// Real LaTeX expression tree with layout engine.

/// A node in the math expression AST.
#[derive(Debug, Clone)]
enum Expr {
    Literal(String),
    Sub(Box<Expr>, Box<Expr>),
    Sup(Box<Expr>, Box<Expr>),
    Frac(Box<Expr>, Box<Expr>),
    Sqrt(Box<Expr>),
    Greek(char),
    BigOp {
        op: char,
        sub: Box<Expr>,
        sup: Box<Expr>,
    },
    Group(Vec<Expr>),
}

/// Tokenizer output for LaTeX
#[derive(Debug, Clone)]
enum LatexToken {
    Char(char),
    Command(String),
    Sub,
    Sup,
    LBrace,
    RBrace,
    Space,
}

fn tokenize_latex_raw(s: &str) -> Vec<LatexToken> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                if chars[i].is_alphabetic() {
                    let mut name = String::new();
                    while i < chars.len() && chars[i].is_alphabetic() {
                        name.push(chars[i]);
                        i += 1;
                    }
                    tokens.push(LatexToken::Command(name));
                } else {
                    // single-char escape like \\ \, etc.
                    tokens.push(LatexToken::Char(chars[i]));
                    i += 1;
                }
            }
            '_' => {
                tokens.push(LatexToken::Sub);
                i += 1;
            }
            '^' => {
                tokens.push(LatexToken::Sup);
                i += 1;
            }
            '{' => {
                tokens.push(LatexToken::LBrace);
                i += 1;
            }
            '}' => {
                tokens.push(LatexToken::RBrace);
                i += 1;
            }
            ' ' | '\t' | '\n' | '\r' => {
                tokens.push(LatexToken::Space);
                i += 1;
            }
            c => {
                tokens.push(LatexToken::Char(c));
                i += 1;
            }
        }
    }
    tokens
}

/// Map a LaTeX command name to an Expr node.
fn command_to_expr(name: &str) -> Expr {
    match name {
        // Greek lowercase
        "alpha" => Expr::Greek('α'),
        "beta" => Expr::Greek('β'),
        "gamma" => Expr::Greek('γ'),
        "delta" => Expr::Greek('δ'),
        "epsilon" => Expr::Greek('ε'),
        "varepsilon" => Expr::Greek('ε'),
        "zeta" => Expr::Greek('ζ'),
        "eta" => Expr::Greek('η'),
        "theta" => Expr::Greek('θ'),
        "vartheta" => Expr::Greek('ϑ'),
        "iota" => Expr::Greek('ι'),
        "kappa" => Expr::Greek('κ'),
        "lambda" => Expr::Greek('λ'),
        "mu" => Expr::Greek('μ'),
        "nu" => Expr::Greek('ν'),
        "xi" => Expr::Greek('ξ'),
        "pi" => Expr::Greek('π'),
        "varpi" => Expr::Greek('ϖ'),
        "rho" => Expr::Greek('ρ'),
        "sigma" => Expr::Greek('σ'),
        "tau" => Expr::Greek('τ'),
        "upsilon" => Expr::Greek('υ'),
        "phi" => Expr::Greek('φ'),
        "varphi" => Expr::Greek('φ'),
        "chi" => Expr::Greek('χ'),
        "psi" => Expr::Greek('ψ'),
        "omega" => Expr::Greek('ω'),
        // Greek uppercase
        "Alpha" => Expr::Greek('Α'),
        "Beta" => Expr::Greek('Β'),
        "Gamma" => Expr::Greek('Γ'),
        "Delta" => Expr::Greek('Δ'),
        "Epsilon" => Expr::Greek('Ε'),
        "Zeta" => Expr::Greek('Ζ'),
        "Eta" => Expr::Greek('Η'),
        "Theta" => Expr::Greek('Θ'),
        "Iota" => Expr::Greek('Ι'),
        "Kappa" => Expr::Greek('Κ'),
        "Lambda" => Expr::Greek('Λ'),
        "Mu" => Expr::Greek('Μ'),
        "Nu" => Expr::Greek('Ν'),
        "Xi" => Expr::Greek('Ξ'),
        "Pi" => Expr::Greek('Π'),
        "Rho" => Expr::Greek('Ρ'),
        "Sigma" => Expr::Greek('Σ'),
        "Tau" => Expr::Greek('Τ'),
        "Upsilon" => Expr::Greek('Υ'),
        "Phi" => Expr::Greek('Φ'),
        "Chi" => Expr::Greek('Χ'),
        "Psi" => Expr::Greek('Ψ'),
        "Omega" => Expr::Greek('Ω'),
        // Infinity
        "infty" | "infinity" => Expr::Greek('∞'),
        // Operators
        "pm" => Expr::Literal("±".to_string()),
        "mp" => Expr::Literal("∓".to_string()),
        "times" => Expr::Literal("×".to_string()),
        "div" => Expr::Literal("÷".to_string()),
        "leq" | "le" => Expr::Literal("≤".to_string()),
        "geq" | "ge" => Expr::Literal("≥".to_string()),
        "neq" | "ne" => Expr::Literal("≠".to_string()),
        "approx" => Expr::Literal("≈".to_string()),
        "rightarrow" | "to" => Expr::Literal("→".to_string()),
        "leftarrow" | "gets" => Expr::Literal("←".to_string()),
        "Rightarrow" => Expr::Literal("⇒".to_string()),
        "Leftarrow" => Expr::Literal("⇐".to_string()),
        "leftrightarrow" => Expr::Literal("↔".to_string()),
        "cdot" => Expr::Literal("·".to_string()),
        "cdots" => Expr::Literal("···".to_string()),
        "ldots" => Expr::Literal("…".to_string()),
        "partial" => Expr::Greek('∂'),
        "nabla" => Expr::Literal("∇".to_string()),
        "in" => Expr::Literal("∈".to_string()),
        "notin" => Expr::Literal("∉".to_string()),
        "subset" => Expr::Literal("⊂".to_string()),
        "supset" => Expr::Literal("⊃".to_string()),
        "cup" => Expr::Literal("∪".to_string()),
        "cap" => Expr::Literal("∩".to_string()),
        "forall" => Expr::Literal("∀".to_string()),
        "exists" => Expr::Literal("∃".to_string()),
        "lfloor" => Expr::Literal("⌊".to_string()),
        "rfloor" => Expr::Literal("⌋".to_string()),
        "lceil" => Expr::Literal("⌈".to_string()),
        "rceil" => Expr::Literal("⌉".to_string()),
        // Ignore decorators
        "left" | "right" | "big" | "bigg" | "Big" | "Bigg" => Expr::Literal(String::new()),
        "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname" => {
            // Will be handled specially in parser – return empty, content comes from next group
            Expr::Literal(String::new())
        }
        _ => Expr::Literal(format!("\\{}", name)),
    }
}

/// Returns true for big operator commands.
fn is_big_op(name: &str) -> Option<char> {
    match name {
        "sum" => Some('∑'),
        "int" => Some('∫'),
        "oint" => Some('∮'),
        "prod" => Some('∏'),
        "coprod" => Some('∐'),
        "bigoplus" => Some('⊕'),
        "bigotimes" => Some('⊗'),
        "bigcup" => Some('⋃'),
        "bigcap" => Some('⋂'),
        _ => None,
    }
}

/// Returns true for commands that need to consume following groups (\frac, \sqrt, etc.)
fn is_frac(name: &str) -> bool {
    matches!(name, "frac" | "dfrac" | "tfrac" | "cfrac")
}

fn is_sqrt(name: &str) -> bool {
    matches!(name, "sqrt" | "cbrt")
}

/// Parse a sequence of LaTeX tokens into a Vec of Expr nodes.
fn parse_expr_seq(tokens: &[LatexToken], idx: &mut usize) -> Vec<Expr> {
    let mut exprs = Vec::new();
    while *idx < tokens.len() {
        match &tokens[*idx] {
            LatexToken::RBrace => {
                // End of a group context – stop parsing
                break;
            }
            LatexToken::Space => {
                *idx += 1;
            }
            LatexToken::LBrace => {
                *idx += 1; // consume '{'
                let inner = parse_expr_seq(tokens, idx);
                if *idx < tokens.len() {
                    *idx += 1; // consume '}'
                }
                exprs.push(Expr::Group(inner));
            }
            LatexToken::Sub => {
                *idx += 1;
                let base = exprs.pop().unwrap_or(Expr::Literal(String::new()));
                let sub = parse_single_expr(tokens, idx);
                // Check if there's also a ^
                exprs.push(Expr::Sub(Box::new(base), Box::new(sub)));
            }
            LatexToken::Sup => {
                *idx += 1;
                let base = exprs.pop().unwrap_or(Expr::Literal(String::new()));
                let sup = parse_single_expr(tokens, idx);
                exprs.push(Expr::Sup(Box::new(base), Box::new(sup)));
            }
            LatexToken::Char(c) => {
                exprs.push(Expr::Literal(c.to_string()));
                *idx += 1;
            }
            LatexToken::Command(name) => {
                let name = name.clone();
                *idx += 1;
                if let Some(op_char) = is_big_op(&name) {
                    // Parse optional sub and sup
                    let mut sub = Expr::Literal(String::new());
                    let mut sup = Expr::Literal(String::new());
                    // Peek for _ or ^
                    loop {
                        skip_spaces(tokens, idx);
                        if *idx >= tokens.len() {
                            break;
                        }
                        match &tokens[*idx] {
                            LatexToken::Sub => {
                                *idx += 1;
                                sub = parse_single_expr(tokens, idx);
                            }
                            LatexToken::Sup => {
                                *idx += 1;
                                sup = parse_single_expr(tokens, idx);
                            }
                            _ => break,
                        }
                    }
                    exprs.push(Expr::BigOp {
                        op: op_char,
                        sub: Box::new(sub),
                        sup: Box::new(sup),
                    });
                } else if is_frac(&name) {
                    skip_spaces(tokens, idx);
                    let num = parse_single_expr(tokens, idx);
                    skip_spaces(tokens, idx);
                    let den = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Frac(Box::new(num), Box::new(den)));
                } else if is_sqrt(&name) {
                    // Optionally skip [n] for nth root
                    skip_spaces(tokens, idx);
                    if *idx < tokens.len() {
                        if let LatexToken::Char('[') = &tokens[*idx] {
                            // consume until ']'
                            *idx += 1;
                            while *idx < tokens.len() {
                                if let LatexToken::Char(']') = &tokens[*idx] {
                                    *idx += 1;
                                    break;
                                }
                                *idx += 1;
                            }
                        }
                    }
                    skip_spaces(tokens, idx);
                    let inner = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Sqrt(Box::new(inner)));
                } else if matches!(
                    name.as_str(),
                    "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname"
                ) {
                    // Consume following group and render it as literal text
                    skip_spaces(tokens, idx);
                    let inner = parse_single_expr(tokens, idx);
                    exprs.push(inner);
                } else if matches!(
                    name.as_str(),
                    "left" | "right" | "big" | "bigg" | "Big" | "Bigg"
                ) {
                    // consume the following bracket char if any
                    if *idx < tokens.len() {
                        if let LatexToken::Char(_) = &tokens[*idx] {
                            *idx += 1;
                        }
                    }
                } else {
                    exprs.push(command_to_expr(&name));
                }
            }
        }
    }
    exprs
}

fn skip_spaces(tokens: &[LatexToken], idx: &mut usize) {
    while *idx < tokens.len() {
        if let LatexToken::Space = &tokens[*idx] {
            *idx += 1;
        } else {
            break;
        }
    }
}

/// Parse exactly one expression node (either a group or a single token).
fn parse_single_expr(tokens: &[LatexToken], idx: &mut usize) -> Expr {
    skip_spaces(tokens, idx);
    if *idx >= tokens.len() {
        return Expr::Literal(String::new());
    }
    match &tokens[*idx].clone() {
        LatexToken::LBrace => {
            *idx += 1;
            let inner = parse_expr_seq(tokens, idx);
            if *idx < tokens.len() {
                *idx += 1; // consume '}'
            }
            Expr::Group(inner)
        }
        LatexToken::Char(c) => {
            let c = *c;
            *idx += 1;
            Expr::Literal(c.to_string())
        }
        LatexToken::Command(name) => {
            let name = name.clone();
            *idx += 1;
            command_to_expr(&name)
        }
        LatexToken::Sub | LatexToken::Sup => Expr::Literal(String::new()),
        LatexToken::RBrace | LatexToken::Space => Expr::Literal(String::new()),
    }
}

fn parse_math_expr(s: &str) -> Vec<Expr> {
    let tokens = tokenize_latex_raw(s);
    let mut idx = 0;
    parse_expr_seq(&tokens, &mut idx)
}

// ─── Layout engine ────────────────────────────────────────────────────────────

/// Layout result for an expression node.
struct Layout {
    svg: String,
    width: f64,
    height: f64,
    ascent: f64, // distance from top to baseline
}

const BASE_FONT: f64 = 20.0;
const CHAR_W_RATIO: f64 = 0.55; // approximate char width as fraction of font-size
const SUB_SCALE: f64 = 0.65;
const SUP_SCALE: f64 = 0.65;
const FRAC_PAD: f64 = 4.0;
const SQRT_LEAN: f64 = 8.0; // width of the radical foot

fn char_width(font_size: f64) -> f64 {
    font_size * CHAR_W_RATIO
}

fn layout_expr(expr: &Expr, font_size: f64) -> Layout {
    match expr {
        Expr::Literal(s) => {
            if s.is_empty() {
                return Layout {
                    svg: String::new(),
                    width: 0.0,
                    height: font_size,
                    ascent: font_size * 0.8,
                };
            }
            // Estimate width: each char is ~char_w
            let char_w = char_width(font_size);
            let width = s.chars().count() as f64 * char_w;
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                font_size,
                escape_xml(s)
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Greek(c) => {
            let char_w = char_width(font_size);
            let width = char_w * 1.2; // Greek chars slightly wider
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                font_size,
                escape_xml(&c.to_string())
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Group(exprs) => layout_group(exprs, font_size),
        Expr::Sub(base, sub) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            // sub goes below-right of base, shifted down by 0.3em
            let sub_shift = font_size * 0.3;
            let sub_x = base_l.width;
            let sub_y = base_l.ascent + sub_shift;
            let total_w = base_l.width + sub_l.width;
            let total_h = (sub_y + sub_l.height).max(base_l.height);
            let ascent = base_l.ascent;
            let svg = format!(
                "{}<g transform=\"translate({},{})\">{}</g>",
                base_l.svg,
                sub_x,
                sub_y - sub_l.ascent,
                sub_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sup(base, sup) => {
            let base_l = layout_expr(base, font_size);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            // sup goes above-right of base, shifted up by 0.5em
            let sup_shift = font_size * 0.5;
            let sup_x = base_l.width;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let _actual_sup_y = sup_y.min(0.0);
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let total_w = base_l.width + sup_l.width;
            let total_h = (base_l.height + dy).max(sup_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg =
                format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g></g>",
                dy, base_l.svg, sup_x, sup_y + dy, sup_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Frac(num, den) => {
            let num_l = layout_expr(num, font_size * 0.85);
            let den_l = layout_expr(den, font_size * 0.85);
            let inner_w = num_l.width.max(den_l.width) + FRAC_PAD * 2.0;
            // Line at the middle
            let line_y = num_l.height + FRAC_PAD;
            let total_h = num_l.height + FRAC_PAD + 2.0 + FRAC_PAD + den_l.height;
            let ascent = line_y + 1.0; // baseline at the fraction line
            let num_x = (inner_w - num_l.width) / 2.0;
            let den_x = (inner_w - den_l.width) / 2.0;
            let den_y = line_y + 2.0 + FRAC_PAD;
            let svg = format!(
                "<g transform=\"translate({},0)\">{}</g>\
                 <line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                num_x,
                num_l.svg,
                line_y + 1.0,
                inner_w,
                line_y + 1.0,
                den_x,
                den_y,
                den_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sqrt(inner) => {
            let inner_l = layout_expr(inner, font_size);
            let pad = 4.0;
            let inner_x = SQRT_LEAN + pad;
            let inner_y = pad;
            let total_w = inner_x + inner_l.width + pad;
            let total_h = inner_l.height + pad * 2.0;
            let ascent = inner_l.ascent + pad;
            // Radical path: short foot then up to top then horizontal overline
            let foot_x = 0.0;
            let foot_y = total_h * 0.75;
            let corner_x = SQRT_LEAN * 0.5;
            let corner_y = total_h;
            let top_left_x = SQRT_LEAN;
            let top_left_y = inner_y;
            let overline_end_x = total_w - 1.0;
            let svg = format!(
                "<path d=\"M {},{} L {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.5\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                foot_x, foot_y,
                corner_x, corner_y,
                top_left_x, top_left_y,
                overline_end_x, top_left_y,
                inner_x, inner_y,
                inner_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::BigOp { op, sub, sup } => {
            let op_font = font_size * 1.6;
            let op_char = op.to_string();
            let op_char_w = char_width(op_font) * 1.4;
            let op_h = op_font * 1.2;
            let op_ascent = op_font * 0.8;

            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);

            let inner_w = op_char_w.max(sub_l.width).max(sup_l.width);

            // sup above operator, sub below
            let sup_y = 0.0;
            let op_y = sup_l.height + 2.0;
            let sub_y = op_y + op_h + 2.0;
            let total_h = sub_y + sub_l.height;
            let ascent = op_y + op_ascent;

            let op_x = (inner_w - op_char_w) / 2.0;
            let sup_x = (inner_w - sup_l.width) / 2.0;
            let sub_x = (inner_w - sub_l.width) / 2.0;

            let svg = format!(
                "<g transform=\"translate({},{})\">{}</g>\
                 <text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{}</text>\
                 <g transform=\"translate({},{})\">{}</g>",
                sup_x, sup_y, sup_l.svg,
                op_x + op_char_w / 2.0, op_y + op_ascent, op_font, escape_xml(&op_char),
                sub_x, sub_y, sub_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
    }
}

fn layout_group(exprs: &[Expr], font_size: f64) -> Layout {
    if exprs.is_empty() {
        return Layout {
            svg: String::new(),
            width: 0.0,
            height: font_size * 1.2,
            ascent: font_size * 0.8,
        };
    }
    let layouts: Vec<Layout> = exprs.iter().map(|e| layout_expr(e, font_size)).collect();
    // Align all nodes by baseline
    let max_ascent = layouts.iter().map(|l| l.ascent).fold(0.0f64, f64::max);
    let max_below = layouts
        .iter()
        .map(|l| l.height - l.ascent)
        .fold(0.0f64, f64::max);
    let total_h = max_ascent + max_below;
    let mut x = 0.0f64;
    let mut svg = String::new();
    for l in &layouts {
        if l.width == 0.0 && l.svg.is_empty() {
            continue;
        }
        let dy = max_ascent - l.ascent;
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\">{}</g>",
            x, dy, l.svg
        ));
        x += l.width;
    }
    // Add small gap between items
    let total_w = x;
    Layout {
        svg,
        width: total_w,
        height: total_h,
        ascent: max_ascent,
    }
}

fn render_math(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startmath", "@endmath");
    let expr_text = body.trim();
    if expr_text.is_empty() {
        return Err(Diagnostic::error("[E_MATH_EMPTY] @startmath body is empty"));
    }

    // Parse and layout
    let exprs = parse_math_expr(expr_text);
    let layout = layout_group(&exprs, BASE_FONT);

    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 30i32;
    let w = (layout.width as i32 + margin * 2).max(200);
    let h = layout.height as i32 + margin * 2 + title_h;

    let mut out = String::new();
    out.push_str(&svg_header(w, h));
    out.push_str(svg_white_bg());

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"22\" font-family=\"serif\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#333\">{}</text>",
            w / 2,
            escape_xml(t)
        ));
    }

    // Expression background box
    let ex = (w as f64 - layout.width) / 2.0;
    let ey = title_h + margin;
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#f9f9f9\" stroke=\"#ddd\" stroke-width=\"1\"/>",
        (ex - 10.0) as i32, ey - 10, (layout.width + 20.0) as i32, (layout.height + 20.0) as i32
    ));

    out.push_str(&format!(
        "<g transform=\"translate({},{})\">{}</g>",
        ex as i32, ey, layout.svg
    ));

    out.push_str("</svg>");
    Ok(out)
}

// ─── Family 2: @startditaa ─────────────────────────────────────────────────────
//
// Real ASCII art rasterizer: 5-pass approach.

/// Color hints inside ditaa boxes
fn hint_to_fill(hint: &str) -> Option<&'static str> {
    match hint {
        "cBLU" | "cBlu" => Some("#aad4f5"),
        "cRED" | "cRed" => Some("#f5aaaa"),
        "cGRE" | "cGre" => Some("#aaf5aa"),
        "cYEL" | "cYel" => Some("#f5f5aa"),
        "cBLK" | "cBlk" => Some("#222222"),
        "cWHI" | "cWhi" => Some("#ffffff"),
        "cPNK" | "cPnk" => Some("#f5aad4"),
        "cORA" | "cOra" => Some("#f5d4aa"),
        "cGRA" | "cGra" => Some("#cccccc"),
        _ => None,
    }
}

/// Shape types detected in the grid.
#[derive(Debug, Clone)]
enum ShapeKind {
    Rect,
    RoundedRect,
    Document,
    Cylinder,
    Diamond,
}

/// A detected shape.
#[derive(Debug, Clone)]
struct Shape {
    kind: ShapeKind,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
    fill: String,
    dashed: bool,
    text_lines: Vec<(usize, String)>, // (row_idx, text)
}

/// A connector arrow or line.
#[derive(Debug, Clone)]
struct Connector {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    has_head_end: bool,
    has_head_start: bool,
    dashed: bool,
}

fn render_ditaa(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startditaa", "@endditaa");

    if body.trim().is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa body is empty",
        ));
    }

    // Build padded grid
    let lines: Vec<Vec<char>> = body.lines().map(|l| l.chars().collect()).collect();
    if lines.is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa has no grid content",
        ));
    }

    let cell_w = 10i32;
    let cell_h = 16i32;
    let grid_rows = lines.len();
    let grid_cols = lines.iter().map(|r| r.len()).max().unwrap_or(0);
    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 16i32;
    let svg_w = grid_cols as i32 * cell_w + margin * 2;
    let svg_h = grid_rows as i32 * cell_h + margin * 2 + title_h;

    let get = |r: usize, c: usize| -> char {
        lines
            .get(r)
            .and_then(|row| row.get(c))
            .copied()
            .unwrap_or(' ')
    };

    // ── Pass 1: detect shapes ──────────────────────────────────────────────────

    let mut shapes: Vec<Shape> = Vec::new();
    // Track which cells are claimed by a shape
    let mut claimed = vec![vec![false; grid_cols + 1]; grid_rows + 1];

    for r1 in 0..grid_rows {
        for c1 in 0..grid_cols {
            let tl = get(r1, c1);
            if tl != '+' && tl != '(' {
                continue;
            }
            // Check if this corner has already been claimed as part of a larger shape
            if claimed[r1][c1] {
                continue;
            }
            let rounded_start = tl == '(';

            // Find right corner on same row
            let row_len = lines[r1].len();
            let mut c2_candidates: Vec<usize> = Vec::new();
            let mut cc = c1 + 1;
            while cc < row_len {
                let ch = get(r1, cc);
                if ch == '+' || ch == ')' {
                    // Verify top edge is continuous
                    let top_ok = (c1 + 1..cc).all(|c| matches!(get(r1, c), '-' | '=' | ' '));
                    if top_ok {
                        c2_candidates.push(cc);
                    }
                    break; // only try nearest right corner
                } else if !matches!(ch, '-' | '=' | ' ') {
                    break;
                }
                cc += 1;
            }

            for c2 in c2_candidates {
                let tr = get(r1, c2);
                let rounded_end = tr == ')';

                // Find bottom corners
                let mut r2_candidates: Vec<usize> = Vec::new();
                let mut rr = r1 + 1;
                while rr < grid_rows {
                    let bl = get(rr, c1);
                    let br = get(rr, c2);
                    if (bl == '+' || bl == '(') && (br == '+' || br == ')') {
                        // Verify all edges
                        let bot_ok = (c1 + 1..c2).all(|c| matches!(get(rr, c), '-' | '=' | ' '));
                        let left_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c1), '|' | ':' | '+' | ' '));
                        let right_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c2), '|' | ':' | '+' | ' '));
                        if bot_ok && left_ok && right_ok {
                            r2_candidates.push(rr);
                        }
                        break;
                    } else if !matches!(bl, '|' | ':' | ' ' | '+') {
                        break;
                    }
                    rr += 1;
                }

                for r2 in r2_candidates {
                    // Determine fill by scanning for color hints inside box
                    let mut fill = "#f0f4ff".to_string();
                    let mut dashed = false;
                    let mut text_lines: Vec<(usize, String)> = Vec::new();

                    for row_idx in (r1 + 1)..r2 {
                        let mut inner = String::new();
                        for ci in (c1 + 1)..c2 {
                            let ch = get(row_idx, ci);
                            if !matches!(ch, '|' | ':') {
                                inner.push(ch);
                            }
                        }
                        let trimmed_inner = inner.trim().to_string();

                        // Color hint detection
                        for word in trimmed_inner.split_whitespace() {
                            if let Some(f) = hint_to_fill(word) {
                                fill = f.to_string();
                            }
                        }

                        // Check for dashed edges
                        if (c1 + 1..c2).any(|c| get(r1, c) == '=')
                            || (r1 + 1..r2).any(|r| get(r, c1) == ':')
                        {
                            dashed = true;
                        }

                        // Remove color hints from display text
                        let display: String = trimmed_inner
                            .split_whitespace()
                            .filter(|w| hint_to_fill(w).is_none())
                            .collect::<Vec<_>>()
                            .join(" ");

                        if !display.is_empty() {
                            text_lines.push((row_idx, display));
                        }
                    }

                    // Determine shape kind
                    let kind = if rounded_start || rounded_end {
                        ShapeKind::RoundedRect
                    } else {
                        // Check for cylinder: top row has '(' at c1+1 and ')' at c2-1
                        let maybe_cyl = c2 > c1 + 2
                            && (r1 + 1..r2).all(|r| get(r, c1) == '|' && get(r, c2) == '|')
                            && get(r1, c1 + 1) == '('
                            && get(r1, c2 - 1) == ')';
                        // Check for diamond: /...\ top and \.../ bottom
                        let maybe_diamond = c2 > c1 + 2
                            && get(r1, c1 + 1) == '/'
                            && get(r1, c2 - 1) == '\\'
                            && get(r2, c1 + 1) == '\\'
                            && get(r2, c2 - 1) == '/';
                        // Check for document: bottom row has '~' wave
                        let maybe_doc = (c1 + 1..c2).any(|c| get(r2, c) == '~');

                        if maybe_diamond {
                            ShapeKind::Diamond
                        } else if maybe_cyl {
                            ShapeKind::Cylinder
                        } else if maybe_doc {
                            ShapeKind::Document
                        } else {
                            ShapeKind::Rect
                        }
                    };

                    // Mark cells as claimed
                    for row in claimed.iter_mut().take(r2 + 1).skip(r1) {
                        for c in c1..=c2 {
                            if c < row.len() {
                                row[c] = true;
                            }
                        }
                    }

                    shapes.push(Shape {
                        kind,
                        r1,
                        c1,
                        r2,
                        c2,
                        fill,
                        dashed,
                        text_lines,
                    });
                }
            }
        }
    }

    // ── Pass 2: connector detection ────────────────────────────────────────────

    let mut connectors: Vec<Connector> = Vec::new();

    // Horizontal connectors (sequences of '-' or '=' not part of shape border)
    for (row_idx, row) in lines.iter().enumerate() {
        let mut c = 0usize;
        while c < row.len() {
            let ch = row[c];
            if ch == '<' && c + 1 < row.len() && row[c + 1] == '-' {
                // Left-pointing arrow start
                let c_start = c;
                c += 1;
                let dashed = row[c] == '=';
                while c < row.len() && matches!(row[c], '-' | '=') {
                    c += 1;
                }
                let c_end = c;
                // Check not on shape border
                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + c_end as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: false,
                        has_head_start: true,
                        dashed,
                    });
                }
            } else if matches!(ch, '-' | '=') {
                let c_start = c;
                let dashed = ch == '=';
                while c < row.len() && matches!(row[c], '-' | '=') {
                    c += 1;
                }
                let has_head = c < row.len() && row[c] == '>';
                if has_head {
                    c += 1;
                }
                let c_end = c;

                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border && c_end > c_start {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + (c_end - if has_head { 1 } else { 0 }) as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                c += 1;
            }
        }
    }

    // Vertical connectors (sequences of '|' or ':')
    for col_idx in 0..grid_cols {
        let mut r = 0usize;
        while r < grid_rows {
            let ch = get(r, col_idx);
            if ch == '^' && r + 1 < grid_rows && matches!(get(r + 1, col_idx), '|' | ':') {
                // Upward arrow
                let r_start = r;
                r += 1;
                let dashed = get(r, col_idx) == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':') {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_end as i32 * cell_h;
                    let y2 = margin + title_h + r_start as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: true,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else if matches!(ch, '|' | ':') {
                let r_start = r;
                let dashed = ch == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':') {
                    r += 1;
                }
                // Check if next char is 'v'
                let has_head = r < grid_rows && get(r, col_idx) == 'v';
                if has_head {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border && r_end > r_start {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_start as i32 * cell_h;
                    let y2 = margin + title_h + r_end as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                r += 1;
            }
        }
    }

    // ── Pass 3: SVG emission ──────────────────────────────────────────────────

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, svg_h));
    out.push_str(svg_white_bg());

    // Arrow markers
    out.push_str(
        "<defs>\
         <marker id=\"da\" markerWidth=\"8\" markerHeight=\"6\" refX=\"6\" refY=\"3\" orient=\"auto\">\
         <path d=\"M0,0 L0,6 L8,3 z\" fill=\"#444\"/></marker>\
         <marker id=\"dah\" markerWidth=\"8\" markerHeight=\"6\" refX=\"2\" refY=\"3\" orient=\"auto\">\
         <path d=\"M8,0 L8,6 L0,3 z\" fill=\"#444\"/></marker>\
         </defs>",
    );

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            svg_w / 2, margin + 16, escape_xml(t)
        ));
    }

    // Draw shapes
    for shape in &shapes {
        let rx = margin + shape.c1 as i32 * cell_w;
        let ry = margin + title_h + shape.r1 as i32 * cell_h;
        let rw = (shape.c2 - shape.c1) as i32 * cell_w;
        let rh = (shape.r2 - shape.r1) as i32 * cell_h;
        let stroke = if shape.dashed {
            "stroke-dasharray=\"6,3\""
        } else {
            ""
        };

        match shape.kind {
            ShapeKind::Rect => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    rx, ry, rw, rh, shape.fill, stroke
                ));
            }
            ShapeKind::RoundedRect => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" ry=\"12\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    rx, ry, rw, rh, shape.fill, stroke
                ));
            }
            ShapeKind::Document => {
                // Draw as rect with curved bottom
                let cx = rx + rw / 2;
                let bot_y = ry + rh;
                out.push_str(&format!(
                    "<path d=\"M {},{} L {},{} L {},{} Q {},{} {},{} Q {},{} {},{}  Z\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    rx, ry,
                    rx + rw, ry,
                    rx + rw, bot_y - 8,
                    cx + rw / 4, bot_y + 6, cx, bot_y - 4,
                    cx - rw / 4, bot_y - 14, rx, bot_y - 8,
                    shape.fill, stroke
                ));
            }
            ShapeKind::Cylinder => {
                let cx = rx + rw / 2;
                let ell_ry = 6i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\"/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"#3344aa\" stroke-width=\"1\"/>",
                    rx, ry + ell_ry, rw, rh - ell_ry, shape.fill, stroke,
                    cx, ry + ell_ry, rw / 2, ell_ry, shape.fill,
                    cx, ry + rh, rw / 2, ell_ry
                ));
            }
            ShapeKind::Diamond => {
                let cx = rx + rw / 2;
                let cy = ry + rh / 2;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    cx, ry,
                    rx + rw, cy,
                    cx, ry + rh,
                    rx, cy,
                    shape.fill, stroke
                ));
            }
        }

        // Render text inside shape
        for (row_idx, text) in &shape.text_lines {
            let tx = rx + rw / 2;
            let ty = margin + title_h + *row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#111\">{}</text>",
                tx, ty, escape_xml(text)
            ));
        }
    }

    // Draw connectors
    for conn in &connectors {
        let dash = if conn.dashed {
            " stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let mut marker_end = "";
        let mut marker_start = "";
        if conn.has_head_end {
            marker_end = " marker-end=\"url(#da)\"";
        }
        if conn.has_head_start {
            marker_start = " marker-start=\"url(#dah)\"";
        }
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\"{}{}{}/>",
            conn.x1, conn.y1, conn.x2, conn.y2, dash, marker_end, marker_start
        ));
    }

    // ── Pass 4: render unclaimed text ─────────────────────────────────────────

    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            // Skip if inside a shape region
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            // Skip structural chars and arrows
            if matches!(
                ch,
                '+' | '-'
                    | '|'
                    | '='
                    | ':'
                    | '>'
                    | '<'
                    | 'v'
                    | '^'
                    | ' '
                    | '~'
                    | '('
                    | ')'
                    | '/'
                    | '\\'
            ) {
                continue;
            }
            let tx = margin + c as i32 * cell_w;
            let ty = margin + title_h + row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#222\">{}</text>",
                tx, ty, escape_xml(&ch.to_string())
            ));
        }
    }

    out.push_str("</svg>");
    Ok(out)
}
