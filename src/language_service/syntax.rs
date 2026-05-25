#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTokenKind {
    Keyword,
    Operator,
    Directive,
    Preprocessor,
    Comment,
    String,
    Color,
    Stereotype,
    Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxTokenSpec {
    pub lexeme: &'static str,
    pub kind: SyntaxTokenKind,
    pub families: &'static [&'static str],
}

pub fn syntax_token_specs() -> &'static [SyntaxTokenSpec] {
    use SyntaxTokenKind::{Directive, Keyword, Operator, Preprocessor};
    &[
        SyntaxTokenSpec {
            lexeme: "@startuml",
            kind: Directive,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "@enduml",
            kind: Directive,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "@startpicouml",
            kind: Directive,
            families: &["picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "@endpicouml",
            kind: Directive,
            families: &["picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!include",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!include_once",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!include_many",
            kind: Preprocessor,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "!includeurl",
            kind: Preprocessor,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "!includesub",
            kind: Preprocessor,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "!define",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!import",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!if",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!else",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!endif",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!theme",
            kind: Preprocessor,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "!startsub",
            kind: Preprocessor,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "!endsub",
            kind: Preprocessor,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "sprite",
            kind: Keyword,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "listsprite",
            kind: Keyword,
            families: &["plantuml"],
        },
        SyntaxTokenSpec {
            lexeme: "participant",
            kind: Keyword,
            families: &["sequence", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "actor",
            kind: Keyword,
            families: &["sequence", "usecase"],
        },
        SyntaxTokenSpec {
            lexeme: "class",
            kind: Keyword,
            families: &["class"],
        },
        SyntaxTokenSpec {
            lexeme: "interface",
            kind: Keyword,
            families: &["class", "component"],
        },
        SyntaxTokenSpec {
            lexeme: "enum",
            kind: Keyword,
            families: &["class"],
        },
        SyntaxTokenSpec {
            lexeme: "state",
            kind: Keyword,
            families: &["state"],
        },
        SyntaxTokenSpec {
            lexeme: "start",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "stop",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "if",
            kind: Keyword,
            families: &["activity", "preprocessor"],
        },
        SyntaxTokenSpec {
            lexeme: "fork",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "while",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "repeat",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "partition",
            kind: Keyword,
            families: &["activity"],
        },
        SyntaxTokenSpec {
            lexeme: "component",
            kind: Keyword,
            families: &["component"],
        },
        SyntaxTokenSpec {
            lexeme: "node",
            kind: Keyword,
            families: &["deployment"],
        },
        SyntaxTokenSpec {
            lexeme: "artifact",
            kind: Keyword,
            families: &["deployment"],
        },
        SyntaxTokenSpec {
            lexeme: "cloud",
            kind: Keyword,
            families: &["deployment"],
        },
        SyntaxTokenSpec {
            lexeme: "skinparam",
            kind: Keyword,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "note",
            kind: Keyword,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "legend",
            kind: Keyword,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "title",
            kind: Keyword,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "->",
            kind: Operator,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "-->",
            kind: Operator,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "<-",
            kind: Operator,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "<--",
            kind: Operator,
            families: &["plantuml", "picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "=>",
            kind: Operator,
            families: &["picouml"],
        },
        SyntaxTokenSpec {
            lexeme: "<|--",
            kind: Operator,
            families: &["class"],
        },
        SyntaxTokenSpec {
            lexeme: "*--",
            kind: Operator,
            families: &["class"],
        },
        SyntaxTokenSpec {
            lexeme: "o--",
            kind: Operator,
            families: &["class"],
        },
        SyntaxTokenSpec {
            lexeme: "..>",
            kind: Operator,
            families: &["class", "component", "deployment"],
        },
        SyntaxTokenSpec {
            lexeme: "[*]",
            kind: Keyword,
            families: &["state"],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_catalog_names_plantuml_and_picouml_surface_tokens() {
        let lexemes = syntax_token_specs()
            .iter()
            .map(|spec| spec.lexeme)
            .collect::<Vec<_>>();

        for expected in [
            "@startuml",
            "@startpicouml",
            "!include",
            "!include_once",
            "!includeurl",
            "!startsub",
            "!import",
            "sprite",
            "class",
            "component",
            "state",
            "start",
            "fork",
            "<|--",
            "=>",
            "[*]",
        ] {
            assert!(lexemes.contains(&expected), "missing {expected}");
        }
    }
}
