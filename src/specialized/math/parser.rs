use super::ast::Expr;
use super::commands::{command_to_expr, is_big_op, is_binom, is_frac, is_sqrt};
use super::tokenizer::{tokenize_latex_raw, LatexToken};

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
