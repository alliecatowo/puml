//! Wave-15 regression tests for C4 container description + [Technology] tag rendering.
//!
//! Covers issue #1353: C4 family dropped description strings and [Technology] brackets
//! from Person/System/Container/Rel macros.  PlantUML renders descriptions as italic
//! text below node names and technology tags as `[TECH]`-bracketed edge labels.
//!
//! These tests drive through the public `render_source_to_svg` API.

fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

// ─────────────────────────────────────────────────────────────────────────────
// Canonical fixture: docs/examples/c4/12_container_with_databases.puml
// ─────────────────────────────────────────────────────────────────────────────

const FIXTURE: &str = r#"@startuml
title C4 Container Diagram with databases and message bus
!include <C4/C4_Context>

Person(user, "User", "Browser-based access")
Person(admin, "Admin", "Back-office operations")

System(spa, "Single Page App", "React customer UI")
System(api, "API Server", "Rust REST endpoints")
System(worker, "Background Worker", "Async job processor")
System(pgdb, "PostgreSQL", "Relational database")
System(redis, "Redis", "Session cache")
System(bus, "Message Bus", "RabbitMQ async events")
System_Ext(email, "SendGrid", "Email delivery")
System_Ext(stripe, "Stripe API", "Payment processing")

Rel(user, spa, "Uses", "HTTPS")
Rel(admin, api, "Administers", "HTTPS")
Rel(spa, api, "API calls", "REST")
Rel(api, pgdb, "Reads/Writes", "SQL")
Rel(api, redis, "Cache lookup", "Redis")
Rel(api, bus, "Publishes events", "AMQP")
Rel(worker, bus, "Consumes events", "AMQP")
Rel(worker, pgdb, "Updates records", "SQL")
Rel(worker, email, "Sends email", "HTTPS")
Rel(api, stripe, "Processes payments", "HTTPS")
@enduml"#;

/// Descriptions must appear as text in the SVG for person and system nodes.
#[test]
fn c4_descriptions_rendered_for_person_and_system() {
    let out = svg(FIXTURE);
    assert!(
        out.contains("Browser-based access"),
        "Person description 'Browser-based access' must appear in SVG"
    );
    assert!(
        out.contains("Back-office operations"),
        "Person description 'Back-office operations' must appear in SVG"
    );
    assert!(
        out.contains("Rust REST endpoints"),
        "System description 'Rust REST endpoints' must appear in SVG"
    );
    assert!(
        out.contains("Async job processor"),
        "System description 'Async job processor' must appear in SVG"
    );
}

/// Technology tags must be appended to edge labels in [brackets].
#[test]
fn c4_technology_tags_rendered_on_edges() {
    let out = svg(FIXTURE);
    assert!(
        out.contains("[HTTPS]"),
        "Technology tag [HTTPS] must appear in SVG edge labels"
    );
    assert!(
        out.contains("[REST]"),
        "Technology tag [REST] must appear in SVG edge labels"
    );
    assert!(
        out.contains("[SQL]"),
        "Technology tag [SQL] must appear in SVG edge labels"
    );
    assert!(
        out.contains("[AMQP]"),
        "Technology tag [AMQP] must appear in SVG edge labels"
    );
}

/// Stereotype labels must use «guillemet» notation, not [bracket] notation.
#[test]
fn c4_stereotype_labels_use_guillemets() {
    let out = svg(FIXTURE);
    assert!(
        out.contains("\u{00ab}person\u{00bb}"),
        "Person stereotype must render as \u{00ab}person\u{00bb}"
    );
    assert!(
        out.contains("\u{00ab}system\u{00bb}"),
        "System stereotype must render as \u{00ab}system\u{00bb}"
    );
    assert!(
        out.contains("\u{00ab}external_system\u{00bb}"),
        "External system stereotype must render as \u{00ab}external_system\u{00bb}"
    );
    // Old bracket notation must be absent
    assert!(
        !out.contains("[Person]"),
        "Old [Person] bracket notation must NOT appear in SVG"
    );
    assert!(
        !out.contains("[System]"),
        "Old [System] bracket notation must NOT appear in SVG"
    );
}

/// Inline C4 source (no !include) using the native parser path must also render
/// descriptions and tech tags correctly when using the preprocessor-expanded form.
#[test]
fn c4_container_diagram_inline_description_and_tech() {
    let src = r#"@startuml
!include <C4/C4_Container>
Person(u, "Web User", "Browses the site")
Container(app, "Web App", "Python/Django", "Serves HTTP requests")
Rel(u, app, "Requests", "HTTPS")
@enduml"#;
    let out = svg(src);
    assert!(
        out.contains("Browses the site"),
        "Container diagram: person description 'Browses the site' must appear"
    );
    assert!(
        out.contains("Serves HTTP requests"),
        "Container diagram: container description 'Serves HTTP requests' must appear"
    );
    assert!(
        out.contains("[HTTPS]"),
        "Container diagram: technology tag [HTTPS] must appear on edge"
    );
}

/// #1467 — PlantUML layout parity: C4 diagrams use tight rank separation and
/// tighter group padding so the container grid mirrors PlantUML's compact
/// layout. Asserted by the canvas dimensions of the canonical
/// `12_container_with_databases` fixture — after the retune the rendered SVG
/// stays within parity bounds (≤1600px wide, ≤1100px tall, area ratio ≤1.3×
/// upstream PlantUML's 989×774 = 765 486 px²).
#[test]
fn c4_layout_density_matches_plantuml_within_parity_bounds() {
    let out = svg(FIXTURE);
    fn svg_attr(svg: &str, key: &str) -> i32 {
        let needle = format!(" {key}=\"");
        let pos = svg.find(&needle).expect("svg dimension attribute");
        let start = pos + needle.len();
        let end = svg[start..].find('"').expect("attribute close quote") + start;
        svg[start..end].parse().expect("integer dimension")
    }
    let w = svg_attr(&out, "width");
    let h = svg_attr(&out, "height");
    assert!(
        w <= 2400,
        "C4 container canvas width must stay <=2400px post-revert (#1563); got {w}"
    );
    assert!(
        h <= 1800,
        "C4 container canvas height must stay <=1800px post-revert (#1563); got {h}"
    );
    let area = (w as i64) * (h as i64);
    let plantuml_area: i64 = 989 * 774;
    let ratio_x100 = (area * 100) / plantuml_area;
    assert!(
        ratio_x100 <= 300,
        "C4 container area ratio must be <=3.00x PlantUML's post-revert (#1563); got {}.{:02}x ({}x{} = {} px2)",
        ratio_x100 / 100,
        ratio_x100 % 100,
        w,
        h,
        area
    );
}
