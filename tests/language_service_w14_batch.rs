//! Language-service wave-14 batch coverage — issues #190, #400, #402.
//!
//! #190 — Verify completion and hover cover all supported diagram families
//!         (class, state, activity, use-case, object, preprocessor).
//! #400 — Verify the WASM compile DTO covers diagnostics, model, scene,
//!         semantic tokens, and language-service surface (all already
//!         exported via compile_json; these tests assert the contract shape).
//! #402 — Verify the shared syntax taxonomy covers token kinds required by
//!         TextMate, Tree-sitter, and LSP semantic token consumers.

use puml::language_service::{
    completion_items, hover, resolve_completion_item, syntax_token_specs, CompletionItemKind,
    SyntaxTokenKind,
};

// ---------------------------------------------------------------------------
// #190 — Completion coverage for all diagram families
// ---------------------------------------------------------------------------

#[test]
fn completion_covers_class_diagram_keywords() {
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    // Core class diagram keywords (acceptance criterion 1)
    assert!(labels.contains(&"class"), "missing: class");
    assert!(labels.contains(&"interface"), "missing: interface");
    assert!(labels.contains(&"enum"), "missing: enum");
    assert!(
        labels.contains(&"abstract class"),
        "missing: abstract class"
    );

    // Class relation arrows
    assert!(labels.contains(&"<|--"), "missing: <|--");
    assert!(labels.contains(&"*--"), "missing: *--");
    assert!(labels.contains(&"o--"), "missing: o--");
    assert!(labels.contains(&"..|>"), "missing: ..|>");
}

#[test]
fn completion_covers_state_diagram_keywords() {
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    // Acceptance criterion 2
    assert!(labels.contains(&"state"), "missing: state");
    assert!(labels.contains(&"[*]"), "missing: [*]");
    assert!(labels.contains(&"--"), "missing: --");
}

#[test]
fn completion_covers_activity_diagram_keywords_and_action_snippet() {
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    // Acceptance criterion 3
    assert!(labels.contains(&"start"), "missing: start");
    assert!(labels.contains(&"stop"), "missing: stop");
    assert!(labels.contains(&":action;"), "missing: :action;");
    assert!(labels.contains(&"if"), "missing: if");
    assert!(labels.contains(&"while"), "missing: while");
    assert!(labels.contains(&"fork"), "missing: fork");

    // :action; must be a Snippet
    let action = resolve_completion_item(":action;").expect(":action; should resolve");
    assert_eq!(
        action.kind,
        CompletionItemKind::Snippet,
        ":action; should be kind Snippet"
    );
    assert_eq!(action.detail, "Activity Diagram");
}

#[test]
fn completion_covers_preprocessor_directives() {
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    // Acceptance criterion 4
    assert!(labels.contains(&"!define"), "missing: !define");
    assert!(labels.contains(&"!include"), "missing: !include");
    assert!(labels.contains(&"!if"), "missing: !if");
    assert!(labels.contains(&"!theme"), "missing: !theme");
    assert!(labels.contains(&"!import"), "missing: !import");
}

#[test]
fn completion_covers_use_case_and_object_diagram_keywords() {
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    assert!(labels.contains(&"usecase"), "missing: usecase");
    assert!(labels.contains(&"object"), "missing: object");
}

#[test]
fn hover_covers_skinparam_value_type_annotations_for_all_families() {
    // Acceptance criterion 5 — skinparam hover with value types
    let skinparam_checks = [
        ("ArrowColor", "color"),
        ("BackgroundColor", "color"),
        ("classBackgroundColor", "color"),
        ("StateBackgroundColor", "color"),
        ("ActivityBackgroundColor", "color"),
        ("ComponentBackgroundColor", "color"),
        ("TimingBackgroundColor", "color"),
    ];

    for (param, expected_type) in skinparam_checks {
        let src = format!("@startuml\nskinparam {param} #abc\nA -> B\n@enduml\n");
        let hover_result =
            hover(&src, (1, 10)).unwrap_or_else(|| panic!("hover should resolve for {param}"));
        let md = &hover_result.markdown;
        assert!(
            md.contains(&format!("`{param}`")),
            "hover for {param} should contain label"
        );
        assert!(
            md.contains(&format!("Value type: {expected_type}")),
            "hover for {param} should contain value type annotation"
        );
    }
}

// ---------------------------------------------------------------------------
// #400 — WASM compile DTO shape contract (lib-level; no wasm-bindgen needed)
// ---------------------------------------------------------------------------

#[test]
fn compile_dto_surfaces_family_pages_diagnostics_tokens_and_language_service() {
    use puml::language_service::language_service_surface_json;
    use puml::language_service::{diagnostics_with_options, semantic_tokens};
    use puml::ParsePipelineOptions;

    let source = "@startuml\nclass User\n@enduml\n";
    let options = ParsePipelineOptions::default();

    // Diagnostics DTO
    let report = diagnostics_with_options(source, &options);
    assert!(
        report.diagnostics.is_empty(),
        "class diagram should parse cleanly"
    );

    // Semantic tokens DTO
    let tokens = semantic_tokens(source);
    assert!(
        tokens.iter().any(|t| {
            use puml::language_service::SemanticTokenKind;
            t.kind == SemanticTokenKind::Keyword
        }),
        "semantic tokens should include at least one keyword"
    );

    // Language-service surface DTO (schema contract)
    let surface = language_service_surface_json();
    assert_eq!(surface["schema"], "puml.languageService");
    assert_eq!(surface["schemaVersion"], 1);

    // families array
    assert!(
        surface["families"]
            .as_array()
            .expect("families")
            .iter()
            .any(|f| f["name"] == "class"),
        "language service surface should list class family"
    );

    // completion items present
    assert!(
        surface["completion"]["items"]
            .as_array()
            .expect("completion items")
            .iter()
            .any(|item| item["label"] == "class"),
        "language service completion should include class keyword"
    );

    // semanticTokens legend present
    assert!(
        surface["semanticTokens"]["legend"]
            .as_array()
            .expect("legend")
            .iter()
            .any(|t| t == "keyword"),
        "semantic token legend should contain keyword type"
    );
}

// ---------------------------------------------------------------------------
// #402 — Syntax taxonomy: shared token catalog completeness
// ---------------------------------------------------------------------------

#[test]
fn syntax_catalog_covers_activity_detach_kill_tokens() {
    let specs = syntax_token_specs();
    let has = |lexeme: &str, kind: SyntaxTokenKind| {
        specs
            .iter()
            .any(|spec| spec.lexeme == lexeme && spec.kind == kind)
    };

    assert!(
        has("detach", SyntaxTokenKind::Keyword),
        "syntax catalog missing: detach (activity)"
    );
    assert!(
        has("kill", SyntaxTokenKind::Keyword),
        "syntax catalog missing: kill (activity)"
    );
}

#[test]
fn syntax_catalog_covers_usecase_and_object_family_tokens() {
    let specs = syntax_token_specs();
    let has = |lexeme: &str, kind: SyntaxTokenKind| {
        specs
            .iter()
            .any(|spec| spec.lexeme == lexeme && spec.kind == kind)
    };

    assert!(
        has("usecase", SyntaxTokenKind::Keyword),
        "syntax catalog missing: usecase"
    );
    assert!(
        has("object", SyntaxTokenKind::Keyword),
        "syntax catalog missing: object"
    );
    assert!(
        has("abstract", SyntaxTokenKind::Keyword),
        "syntax catalog missing: abstract (class)"
    );
}

#[test]
fn syntax_catalog_token_family_associations_are_accurate() {
    let specs = syntax_token_specs();

    // Verify family associations for key tokens
    let class_spec = specs
        .iter()
        .find(|s| s.lexeme == "class")
        .expect("class token should be in catalog");
    assert!(
        class_spec.families.contains(&"class"),
        "class token should be in class family"
    );

    let state_spec = specs
        .iter()
        .find(|s| s.lexeme == "state")
        .expect("state token should be in catalog");
    assert!(
        state_spec.families.contains(&"state"),
        "state token should be in state family"
    );

    let fork_spec = specs
        .iter()
        .find(|s| s.lexeme == "fork")
        .expect("fork token should be in catalog");
    assert!(
        fork_spec.families.contains(&"activity"),
        "fork token should be in activity family"
    );

    let usecase_spec = specs
        .iter()
        .find(|s| s.lexeme == "usecase")
        .expect("usecase token should be in catalog");
    assert!(
        usecase_spec.families.contains(&"usecase"),
        "usecase token should be in usecase family"
    );
}

#[test]
fn syntax_catalog_completion_and_token_specs_are_aligned_for_key_keywords() {
    // Cross-check: every keyword in the syntax catalog that has a matching
    // completion item has consistent kind labelling (both Keyword).
    let specs = syntax_token_specs();
    let labels: Vec<&str> = completion_items()
        .items
        .iter()
        .map(|item| item.label)
        .collect();

    for spec in specs {
        if spec.kind == SyntaxTokenKind::Keyword {
            // Not every syntax token needs a completion item (short operators,
            // etc.), but the reverse should hold: any syntax token that IS a
            // completion keyword should have consistent kind.
            if let Some(item) = resolve_completion_item(spec.lexeme) {
                assert_ne!(
                    item.kind,
                    CompletionItemKind::Operator,
                    "token `{}` is Keyword in syntax catalog but Operator in completion",
                    spec.lexeme
                );
            }
            // The syntax token should appear in at least one family list
            assert!(
                !spec.families.is_empty(),
                "syntax token `{}` has no family associations",
                spec.lexeme
            );
        }
    }

    // Spot-check that catalog tokens appear in the completion surface
    for expected in ["class", "state", "start", "fork", "component", "node"] {
        assert!(
            labels.contains(&expected),
            "completion surface missing catalog keyword: {expected}"
        );
    }
}
