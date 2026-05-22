use puml::{parse, render_source_to_svg, render_source_to_svgs};

#[test]
fn actor_style_awesome_and_business_variants_render_distinctly() {
    let source = r#"@startuml
skinparam actorStyle awesome
actor/ Seller
usecase/ Invoice as Invoice
Seller --> Invoice
@enduml
"#;

    let svg = render_source_to_svg(source).expect("usecase parity svg should render");

    assert!(svg.contains("data-actor-style=\"awesome\""));
    assert!(svg.contains("data-business-actor=\"true\""));
    assert!(svg.contains("data-business-usecase=\"true\""));
    assert!(!svg.contains("&lt;&lt;business&gt;&gt;"));
}

#[test]
fn actor_style_hollow_renders_alternate_actor_glyph() {
    let source = r#"@startuml
skinparam actorStyle hollow
actor Customer
usecase Login
Customer --> Login
@enduml
"#;

    let svg = render_source_to_svg(source).expect("hollow actor svg should render");

    assert!(svg.contains("data-actor-style=\"hollow\""));
}

#[test]
fn family_newpage_splits_usecase_pages() {
    let source = r#"@startuml
title Catalog
actor Buyer
newpage Checkout
usecase Purchase
@enduml
"#;

    let pages = render_source_to_svgs(source).expect("family newpage should paginate");

    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("Buyer"));
    assert!(!pages[0].contains("Purchase"));
    assert!(pages[1].contains("Purchase"));
    assert!(pages[1].contains("Checkout"));
}

#[test]
fn parser_accepts_family_newpage_statements() {
    let document = parse("@startuml\nclass A\nnewpage Second\nclass B\n@enduml\n")
        .expect("family newpage should parse");

    assert!(document
        .statements
        .iter()
        .any(|statement| matches!(statement.kind, puml::ast::StatementKind::NewPage(_))));
}
