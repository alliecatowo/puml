use super::ast::{Expr, LatexToken};

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

pub(super) fn parse_math_expr(s: &str) -> Vec<Expr> {
    let tokens = tokenize_latex_raw(s);
    let mut idx = 0;
    parse_expr_seq(&tokens, &mut idx)
}
