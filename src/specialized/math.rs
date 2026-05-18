// ─── Family 1: @startmath ─────────────────────────────────────────────────────
//
// Real LaTeX expression tree with layout engine.

use super::shared::{escape_xml, strip_block, svg_header, svg_white_bg};
use crate::diagnostic::Diagnostic;

/// A node in the math expression AST.
#[derive(Debug, Clone)]
enum Expr {
    Literal(String),
    Text(String),
    Sub(Box<Expr>, Box<Expr>),
    Sup(Box<Expr>, Box<Expr>),
    SubSup(Box<Expr>, Box<Expr>, Box<Expr>),
    Frac(Box<Expr>, Box<Expr>),
    Binom(Box<Expr>, Box<Expr>),
    Sqrt(Box<Expr>),
    Accent {
        kind: String,
        inner: Box<Expr>,
    },
    Greek(char),
    Matrix {
        env: String,
        rows: Vec<Vec<Expr>>,
    },
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
        "emptyset" | "varnothing" => Expr::Literal("∅".to_string()),
        "land" | "wedge" => Expr::Literal("∧".to_string()),
        "lor" | "vee" => Expr::Literal("∨".to_string()),
        "neg" | "lnot" => Expr::Literal("¬".to_string()),
        "therefore" => Expr::Literal("∴".to_string()),
        "because" => Expr::Literal("∵".to_string()),
        "equiv" => Expr::Literal("≡".to_string()),
        "propto" => Expr::Literal("∝".to_string()),
        "sim" => Expr::Literal("∼".to_string()),
        "simeq" => Expr::Literal("≃".to_string()),
        "cong" => Expr::Literal("≅".to_string()),
        "ll" => Expr::Literal("≪".to_string()),
        "gg" => Expr::Literal("≫".to_string()),
        "subseteq" => Expr::Literal("⊆".to_string()),
        "supseteq" => Expr::Literal("⊇".to_string()),
        "oplus" => Expr::Literal("⊕".to_string()),
        "otimes" => Expr::Literal("⊗".to_string()),
        "perp" => Expr::Literal("⊥".to_string()),
        "parallel" => Expr::Literal("∥".to_string()),
        "angle" => Expr::Literal("∠".to_string()),
        "degree" => Expr::Literal("°".to_string()),
        "lfloor" => Expr::Literal("⌊".to_string()),
        "rfloor" => Expr::Literal("⌋".to_string()),
        "lceil" => Expr::Literal("⌈".to_string()),
        "rceil" => Expr::Literal("⌉".to_string()),
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "log" | "ln" | "lim" | "min" | "max"
        | "det" | "dim" | "ker" | "Pr" => Expr::Literal(name.to_string()),
        "," | ";" | ":" | "quad" | "qquad" => Expr::Literal(" ".to_string()),
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

fn is_binom(name: &str) -> bool {
    matches!(name, "binom" | "dbinom" | "tbinom")
}

fn is_sqrt(name: &str) -> bool {
    matches!(name, "sqrt" | "cbrt")
}

fn read_braced_literal(tokens: &[LatexToken], idx: &mut usize) -> Option<String> {
    skip_spaces(tokens, idx);
    if !matches!(tokens.get(*idx), Some(LatexToken::LBrace)) {
        return None;
    }
    *idx += 1;
    let mut out = String::new();
    while *idx < tokens.len() {
        match &tokens[*idx] {
            LatexToken::RBrace => {
                *idx += 1;
                return Some(out);
            }
            LatexToken::Char(c) => {
                out.push(*c);
                *idx += 1;
            }
            LatexToken::Command(name) => {
                out.push_str(name);
                *idx += 1;
            }
            LatexToken::Space => {
                out.push(' ');
                *idx += 1;
            }
            LatexToken::Sub => {
                out.push('_');
                *idx += 1;
            }
            LatexToken::Sup => {
                out.push('^');
                *idx += 1;
            }
            LatexToken::LBrace => {
                out.push('{');
                *idx += 1;
            }
        }
    }
    None
}

fn peek_end_env(tokens: &[LatexToken], idx: usize, env: &str) -> Option<usize> {
    if !matches!(tokens.get(idx), Some(LatexToken::Command(name)) if name == "end") {
        return None;
    }
    let mut cursor = idx + 1;
    let name = read_braced_literal(tokens, &mut cursor)?;
    if name.trim() == env {
        Some(cursor)
    } else {
        None
    }
}

fn parse_cell_expr(tokens: &[LatexToken]) -> Expr {
    let mut idx = 0;
    Expr::Group(parse_expr_seq(tokens, &mut idx))
}

fn parse_matrix_env(tokens: &[LatexToken], idx: &mut usize, env: &str) -> Expr {
    let mut rows: Vec<Vec<Expr>> = Vec::new();
    let mut row: Vec<Expr> = Vec::new();
    let mut cell: Vec<LatexToken> = Vec::new();
    let mut depth = 0usize;

    while *idx < tokens.len() {
        if depth == 0 {
            if let Some(end_idx) = peek_end_env(tokens, *idx, env) {
                row.push(parse_cell_expr(&cell));
                if !row.is_empty() {
                    rows.push(row);
                }
                *idx = end_idx;
                return Expr::Matrix {
                    env: env.to_string(),
                    rows,
                };
            }
            match &tokens[*idx] {
                LatexToken::Char('&') => {
                    row.push(parse_cell_expr(&cell));
                    cell.clear();
                    *idx += 1;
                    continue;
                }
                LatexToken::Char('\\') => {
                    row.push(parse_cell_expr(&cell));
                    cell.clear();
                    rows.push(row);
                    row = Vec::new();
                    *idx += 1;
                    continue;
                }
                _ => {}
            }
        }

        match &tokens[*idx] {
            LatexToken::LBrace => depth += 1,
            LatexToken::RBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
        cell.push(tokens[*idx].clone());
        *idx += 1;
    }

    row.push(parse_cell_expr(&cell));
    rows.push(row);
    Expr::Matrix {
        env: env.to_string(),
        rows,
    }
}

fn is_matrix_env(name: &str) -> bool {
    matches!(
        name,
        "matrix"
            | "pmatrix"
            | "bmatrix"
            | "Bmatrix"
            | "vmatrix"
            | "Vmatrix"
            | "smallmatrix"
            | "array"
            | "cases"
            | "aligned"
            | "align"
    )
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
                skip_spaces(tokens, idx);
                if *idx < tokens.len() && matches!(tokens[*idx], LatexToken::Sup) {
                    *idx += 1;
                    let sup = parse_single_expr(tokens, idx);
                    exprs.push(Expr::SubSup(Box::new(base), Box::new(sub), Box::new(sup)));
                } else {
                    exprs.push(Expr::Sub(Box::new(base), Box::new(sub)));
                }
            }
            LatexToken::Sup => {
                *idx += 1;
                let base = exprs.pop().unwrap_or(Expr::Literal(String::new()));
                let sup = parse_single_expr(tokens, idx);
                skip_spaces(tokens, idx);
                if *idx < tokens.len() && matches!(tokens[*idx], LatexToken::Sub) {
                    *idx += 1;
                    let sub = parse_single_expr(tokens, idx);
                    exprs.push(Expr::SubSup(Box::new(base), Box::new(sub), Box::new(sup)));
                } else {
                    exprs.push(Expr::Sup(Box::new(base), Box::new(sup)));
                }
            }
            LatexToken::Char(c) => {
                exprs.push(Expr::Literal(c.to_string()));
                *idx += 1;
            }
            LatexToken::Command(name) => {
                let name = name.clone();
                *idx += 1;
                if name == "begin" {
                    if let Some(env) = read_braced_literal(tokens, idx) {
                        if is_matrix_env(env.trim()) {
                            exprs.push(parse_matrix_env(tokens, idx, env.trim()));
                        } else {
                            exprs.push(Expr::Literal(format!("\\begin{{{}}}", env)));
                        }
                    }
                } else if let Some(op_char) = is_big_op(&name) {
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
                } else if is_binom(&name) {
                    skip_spaces(tokens, idx);
                    let top = parse_single_expr(tokens, idx);
                    skip_spaces(tokens, idx);
                    let bottom = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Binom(Box::new(top), Box::new(bottom)));
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
                    "hat" | "bar" | "overline" | "underline" | "vec" | "dot" | "ddot"
                ) {
                    skip_spaces(tokens, idx);
                    let inner = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Accent {
                        kind: name,
                        inner: Box::new(inner),
                    });
                } else if matches!(
                    name.as_str(),
                    "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname"
                ) {
                    // Consume following group and render it as literal text
                    skip_spaces(tokens, idx);
                    if let Some(text) = read_braced_literal(tokens, idx) {
                        exprs.push(Expr::Text(text));
                    } else {
                        let inner = parse_single_expr(tokens, idx);
                        exprs.push(inner);
                    }
                } else if matches!(
                    name.as_str(),
                    "left" | "right" | "big" | "bigg" | "Big" | "Bigg"
                ) {
                    // consume and render the following delimiter; \left. / \right. are invisible
                    if *idx < tokens.len() {
                        if let LatexToken::Char(c) = &tokens[*idx] {
                            if *c != '.' {
                                exprs.push(Expr::Literal(c.to_string()));
                            }
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
            if name == "begin" {
                if let Some(env) = read_braced_literal(tokens, idx) {
                    if is_matrix_env(env.trim()) {
                        parse_matrix_env(tokens, idx, env.trim())
                    } else {
                        Expr::Literal(format!("\\begin{{{}}}", env))
                    }
                } else {
                    Expr::Literal("\\begin".to_string())
                }
            } else if is_frac(&name) {
                skip_spaces(tokens, idx);
                let num = parse_single_expr(tokens, idx);
                skip_spaces(tokens, idx);
                let den = parse_single_expr(tokens, idx);
                Expr::Frac(Box::new(num), Box::new(den))
            } else if is_sqrt(&name) {
                skip_spaces(tokens, idx);
                let inner = parse_single_expr(tokens, idx);
                Expr::Sqrt(Box::new(inner))
            } else if matches!(
                name.as_str(),
                "hat" | "bar" | "overline" | "underline" | "vec" | "dot" | "ddot"
            ) {
                skip_spaces(tokens, idx);
                let inner = parse_single_expr(tokens, idx);
                Expr::Accent {
                    kind: name,
                    inner: Box::new(inner),
                }
            } else if matches!(
                name.as_str(),
                "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname"
            ) {
                skip_spaces(tokens, idx);
                if let Some(text) = read_braced_literal(tokens, idx) {
                    Expr::Text(text)
                } else {
                    parse_single_expr(tokens, idx)
                }
            } else {
                command_to_expr(&name)
            }
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
        Expr::Text(s) => {
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
        Expr::Matrix { env, rows } => {
            let cell_pad_x = font_size * 0.45;
            let row_gap = font_size * 0.25;
            let layouts: Vec<Vec<Layout>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| layout_expr(cell, font_size * 0.9))
                        .collect()
                })
                .collect();
            let col_count = layouts.iter().map(|row| row.len()).max().unwrap_or(0);
            let mut col_widths = vec![font_size * 0.6; col_count];
            let mut row_heights = vec![font_size; layouts.len()];
            let mut row_ascents = vec![font_size * 0.75; layouts.len()];
            for (r, row) in layouts.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    col_widths[c] = col_widths[c].max(cell.width);
                    row_heights[r] = row_heights[r].max(cell.height);
                    row_ascents[r] = row_ascents[r].max(cell.ascent);
                }
            }
            let body_w = col_widths.iter().sum::<f64>() + cell_pad_x * 2.0 * col_count as f64;
            let body_h =
                row_heights.iter().sum::<f64>() + row_gap * layouts.len().saturating_sub(1) as f64;
            let fence_w =
                if env == "matrix" || env == "smallmatrix" || env == "aligned" || env == "align" {
                    0.0
                } else {
                    font_size * 0.45
                };
            let total_w = body_w + fence_w * 2.0;
            let total_h = body_h.max(font_size);
            let ascent = total_h * 0.58;
            let mut svg = format!("<g data-math-env=\"{}\">", escape_xml(env));

            let mut y = 0.0;
            for (r, row) in layouts.iter().enumerate() {
                let mut x = fence_w;
                for (c, cell) in row.iter().enumerate() {
                    let cell_x = x + cell_pad_x + (col_widths[c] - cell.width) / 2.0;
                    let cell_y = y + row_ascents[r] - cell.ascent;
                    svg.push_str(&format!(
                        "<g transform=\"translate({},{})\">{}</g>",
                        cell_x, cell_y, cell.svg
                    ));
                    x += col_widths[c] + cell_pad_x * 2.0;
                }
                y += row_heights[r] + row_gap;
            }

            match env.as_str() {
                "pmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">(</text><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">)</text>",
                        fence_w / 2.0, ascent, total_h * 1.15, total_w - fence_w / 2.0, ascent, total_h * 1.15
                    ));
                }
                "bmatrix" => {
                    svg.push_str(&format!(
                        "<path d=\"M {},0 L 0,0 L 0,{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/><path d=\"M {},0 L {},0 L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/>",
                        fence_w, total_h, fence_w, total_h, total_w - fence_w, total_w, total_w, total_h, total_w - fence_w, total_h
                    ));
                }
                "Bmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{{</text><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">}}</text>",
                        fence_w / 2.0, ascent, total_h * 1.15, total_w - fence_w / 2.0, ascent, total_h * 1.15
                    ));
                }
                "vmatrix" | "Vmatrix" => {
                    let sw = if env == "Vmatrix" { 2.2 } else { 1.4 };
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/><line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/>",
                        fence_w / 2.0, fence_w / 2.0, total_h, sw, total_w - fence_w / 2.0, total_w - fence_w / 2.0, total_h, sw
                    ));
                }
                _ => {}
            }
            svg.push_str("</g>");
            Layout {
                svg,
                width: total_w,
                height: total_h,
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
        Expr::SubSup(base, sub, sup) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            let script_w = sub_l.width.max(sup_l.width);
            let sup_shift = font_size * 0.5;
            let sub_shift = font_size * 0.3;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let sub_y = base_l.ascent + sub_shift + dy;
            let total_w = base_l.width + script_w;
            let total_h = (sub_y + sub_l.height - sub_l.ascent).max(base_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg = format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g><g transform=\"translate({},{})\">{}</g></g>",
                dy,
                base_l.svg,
                base_l.width,
                sup_y + dy,
                sup_l.svg,
                base_l.width,
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
        Expr::Binom(top, bottom) => {
            let top_l = layout_expr(top, font_size * 0.85);
            let bottom_l = layout_expr(bottom, font_size * 0.85);
            let pad = font_size * 0.35;
            let body_w = top_l.width.max(bottom_l.width) + pad * 2.0;
            let gap = font_size * 0.08;
            let body_h = top_l.height + gap + bottom_l.height;
            let fence_w = font_size * 0.35;
            let total_w = body_w + fence_w * 2.0;
            let total_h = body_h.max(font_size * 1.3);
            let ascent = total_h * 0.58;
            let top_x = fence_w + (body_w - top_l.width) / 2.0;
            let bottom_x = fence_w + (body_w - bottom_l.width) / 2.0;
            let bottom_y = top_l.height + gap;
            let svg = format!(
                "<g data-math-construct=\"binom\"><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">(</text><g transform=\"translate({},{})\">{}</g><g transform=\"translate({},{})\">{}</g><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">)</text></g>",
                fence_w / 2.0,
                ascent,
                total_h * 1.1,
                top_x,
                0.0,
                top_l.svg,
                bottom_x,
                bottom_y,
                bottom_l.svg,
                total_w - fence_w / 2.0,
                ascent,
                total_h * 1.1
            );
            Layout {
                svg,
                width: total_w,
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
        Expr::Accent { kind, inner } => {
            let inner_l = layout_expr(inner, font_size);
            let top_pad = if kind == "underline" {
                2.0
            } else {
                font_size * 0.35
            };
            let bottom_pad = if kind == "underline" {
                font_size * 0.25
            } else {
                0.0
            };
            let width = inner_l.width.max(font_size * 0.7);
            let height = inner_l.height + top_pad + bottom_pad;
            let ascent = inner_l.ascent + top_pad;
            let inner_x = (width - inner_l.width) / 2.0;
            let mut svg = String::new();
            match kind.as_str() {
                "hat" => svg.push_str(&format!(
                    "<path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    width * 0.2,
                    top_pad,
                    width * 0.5,
                    1.0,
                    width * 0.8,
                    top_pad
                )),
                "vec" => svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\" marker-end=\"url(#math-arrow)\"/>",
                    width * 0.15,
                    top_pad * 0.55,
                    width * 0.85,
                    top_pad * 0.55
                )),
                "dot" | "ddot" => {
                    svg.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                        width * 0.45,
                        top_pad * 0.45
                    ));
                    if kind == "ddot" {
                        svg.push_str(&format!(
                            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                            width * 0.6,
                            top_pad * 0.45
                        ));
                    }
                }
                "underline" => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    inner_l.height + top_pad + 2.0,
                    width,
                    inner_l.height + top_pad + 2.0
                )),
                _ => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    top_pad * 0.45,
                    width,
                    top_pad * 0.45
                )),
            }
            svg.push_str(&format!(
                "<g transform=\"translate({},{})\">{}</g>",
                inner_x, top_pad, inner_l.svg
            ));
            Layout {
                svg,
                width,
                height,
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

/// Render a math diagram directly from its body and optional title, without
/// reconstructing the `@startmath...@endmath` wrapper. Called from the model
/// render path (`render::specialized::math`) so the LSP and CLI share the same
/// rendering logic without the two-hop reconstruct round-trip.
pub(crate) fn render_math_from_parts(
    body: &str,
    title: Option<&str>,
) -> Result<String, Diagnostic> {
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
    out.push_str(
        "<defs><marker id=\"math-arrow\" markerWidth=\"7\" markerHeight=\"5\" refX=\"6\" refY=\"2.5\" orient=\"auto\"><path d=\"M0,0 L0,5 L7,2.5 z\" fill=\"#333\"/></marker></defs>",
    );

    if let Some(t) = title {
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

pub(super) fn render_math(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startmath", "@endmath");
    render_math_from_parts(body, title.as_deref())
}

pub(super) fn render_latex(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startlatex", "@endlatex");
    let mut normalized = String::from("@startmath");
    if let Some(t) = title {
        normalized.push(' ');
        normalized.push('"');
        normalized.push_str(&t.replace('"', "\\\""));
        normalized.push('"');
    }
    normalized.push('\n');
    normalized.push_str(body);
    normalized.push('\n');
    normalized.push_str("@endmath\n");
    render_math(&normalized)
}
