/// A node in the math expression AST.
#[derive(Debug, Clone)]
pub(super) enum Expr {
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
