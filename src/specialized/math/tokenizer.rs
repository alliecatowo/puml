/// Tokenizer output for LaTeX
#[derive(Debug, Clone)]
pub(super) enum LatexToken {
    Char(char),
    Command(String),
    Sub,
    Sup,
    LBrace,
    RBrace,
    Space,
}

pub(super) fn tokenize_latex_raw(s: &str) -> Vec<LatexToken> {
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
