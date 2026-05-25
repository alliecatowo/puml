use std::fs;
use std::path::PathBuf;

use puml::language_service::{syntax_token_specs, SyntaxTokenKind};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn shared_syntax_catalog_covers_plantuml_and_picouml_contract_tokens() {
    let specs = syntax_token_specs();
    let has = |lexeme: &str, kind: SyntaxTokenKind| {
        specs
            .iter()
            .any(|spec| spec.lexeme == lexeme && spec.kind == kind)
    };

    assert!(has("@startuml", SyntaxTokenKind::Directive));
    assert!(has("@startpicouml", SyntaxTokenKind::Directive));
    assert!(has("!include", SyntaxTokenKind::Preprocessor));
    assert!(has("!include_once", SyntaxTokenKind::Preprocessor));
    assert!(has("!includeurl", SyntaxTokenKind::Preprocessor));
    assert!(has("!startsub", SyntaxTokenKind::Preprocessor));
    assert!(has("sprite", SyntaxTokenKind::Keyword));
    assert!(has("class", SyntaxTokenKind::Keyword));
    assert!(has("state", SyntaxTokenKind::Keyword));
    assert!(has("fork", SyntaxTokenKind::Keyword));
    assert!(has("=>", SyntaxTokenKind::Operator));
}

#[test]
fn textmate_and_site_tokenizers_cover_catalog_slice() {
    let textmate = fs::read_to_string(repo_path("extensions/vscode/syntaxes/puml.tmLanguage.json"))
        .expect("read TextMate grammar");
    let site_tokens = fs::read_to_string(repo_path("site/static/js/puml-tokens.js"))
        .expect("read site tokenizer");

    for token in [
        "startpicouml",
        "include_once",
        "includeurl",
        "startsub",
        "sprite",
        "class",
        "state",
        "fork",
        "=>",
    ] {
        assert!(
            textmate.contains(token),
            "TextMate grammar missing contract token {token}"
        );
    }

    for token in [
        "include_once",
        "includeurl",
        "startsub",
        "sprite",
        "class",
        "state",
        "fork",
        "=>",
    ] {
        assert!(
            site_tokens.contains(token),
            "site tokenizer missing contract token {token}"
        );
    }

    assert!(
        site_tokens.contains("'picouml'"),
        "site tokenizer should advertise PicoUML highlighting"
    );

    for token in ["import", "o--"] {
        assert!(
            textmate.contains(token),
            "TextMate grammar missing expanded contract token {token}"
        );
        assert!(
            site_tokens.contains(token),
            "site tokenizer missing expanded contract token {token}"
        );
    }
    for (token, textmate_token, site_token) in
        [("<|--", "<\\\\|--", "<\\|--"), ("*--", "\\\\*--", "\\*--")]
    {
        assert!(
            textmate.contains(textmate_token),
            "TextMate grammar missing expanded contract token {token}"
        );
        assert!(
            site_tokens.contains(site_token),
            "site tokenizer missing expanded contract token {token}"
        );
    }

    for (token, textmate_token, site_token) in [
        ("<&folder>", "&[A-Za-z]", "openIconic"),
        ("<$folder>", "<\\\\$[A-Za-z_]", "spriteRef"),
    ] {
        assert!(
            textmate.contains(textmate_token),
            "TextMate grammar missing icon/sprite contract token {token}"
        );
        assert!(
            site_tokens.contains(site_token),
            "site tokenizer missing icon/sprite contract token {token}"
        );
    }
}
