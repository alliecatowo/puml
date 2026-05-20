use super::ast::Expr;

/// Map a LaTeX command name to an Expr node.
pub(super) fn command_to_expr(name: &str) -> Expr {
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
pub(super) fn is_big_op(name: &str) -> Option<char> {
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
pub(super) fn is_frac(name: &str) -> bool {
    matches!(name, "frac" | "dfrac" | "tfrac" | "cfrac")
}

pub(super) fn is_binom(name: &str) -> bool {
    matches!(name, "binom" | "dbinom" | "tbinom")
}

pub(super) fn is_sqrt(name: &str) -> bool {
    matches!(name, "sqrt" | "cbrt")
}
