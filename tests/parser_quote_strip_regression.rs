//! Regression tests for issue #1289: stray closing `"` leaks into package/node
//! titles when a `<<stereotype>>` follows the quoted display name.
//!
//! Acceptance criteria:
//!  - `package "Outer" <<external>>` produces a group label of `Outer`, not `Outer"`.
//!  - `package "Message Bus" <<queue>>` produces `Message Bus`, not `Message Bus"`.
//!  - `node "Cloud Region (us-east-1)" <<cloud>>` produces the clean name.
//!  - The frame's `data-uml-group` attribute never contains a stray `"` character.
//!  - Existing quoted-name-with-alias parsing (`"Name" as Alias`) still works.
//!  - Existing quoted-name-with-color parsing (`"Name" #LightBlue`) still works.

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

// ─────────────────────────────────────────────────────────────────────────────
// Component family: package "Name" <<stereotype>> { }
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn package_quoted_name_with_stereotype_no_stray_quote() {
    let src = r#"@startuml
package "Message Bus" <<queue>> {
  component "RabbitMQ" as MQ
}
@enduml
"#;
    let svg = render(src);

    // The frame must be findable by the clean label without a trailing quote.
    assert!(
        svg.contains("data-uml-group=\"Message Bus\""),
        "group frame should use clean label 'Message Bus', not 'Message Bus\"'"
    );
    // The stray HTML-encoded quote must not appear in any group attribute value.
    assert!(
        !svg.contains("data-uml-group=\"Message Bus&quot;"),
        "stray HTML-encoded quote must not appear in data-uml-group"
    );
}

#[test]
fn package_quoted_name_database_stereotype_no_stray_quote() {
    let src = r#"@startuml
package "Database" <<database>> {
  component "PostgreSQL" as PG
}
@enduml
"#;
    let svg = render(src);

    assert!(
        svg.contains("data-uml-group=\"Database\""),
        "group frame should use clean label 'Database'"
    );
    assert!(
        !svg.contains("data-uml-group=\"Database&quot;"),
        "stray HTML-encoded quote must not appear in data-uml-group"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deployment family: node "Name" <<stereotype>> { }
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn node_quoted_name_with_cloud_stereotype_no_stray_quote() {
    let src = r#"@startuml
node "Cloud Region (us-east-1)" <<cloud>> {
  node "Load Balancer" as LB
}
@enduml
"#;
    let svg = render(src);

    assert!(
        svg.contains("data-uml-group=\"Cloud Region (us-east-1)\""),
        "node group frame should use clean label without stray quote"
    );
    assert!(
        !svg.contains("data-uml-group=\"Cloud Region (us-east-1)&quot;"),
        "stray HTML-encoded quote must not appear in node data-uml-group"
    );
}

#[test]
fn node_quoted_name_queue_stereotype_no_stray_quote() {
    let src = r#"@startuml
node "Message Queue" <<queue>> {
  node "SQS" as SQS
}
@enduml
"#;
    let svg = render(src);

    assert!(
        svg.contains("data-uml-group=\"Message Queue\""),
        "node group frame should use clean label 'Message Queue'"
    );
    assert!(
        !svg.contains("data-uml-group=\"Message Queue&quot;"),
        "stray HTML-encoded quote must not appear in data-uml-group"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regression guards: existing valid forms must still work
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn package_quoted_name_without_stereotype_still_works() {
    let src = r#"@startuml
package "Order Service" {
  component "OrderController" as OC
}
@enduml
"#;
    let svg = render(src);

    assert!(
        svg.contains("data-uml-group=\"Order Service\""),
        "plain quoted package name should still work"
    );
}

#[test]
fn package_quoted_name_with_color_still_works() {
    let src = r#"@startuml
package "Production" #LightYellow {
  component "AppServer" as AS
}
@enduml
"#;
    let svg = render(src);

    assert!(
        svg.contains("data-uml-group=\"Production\""),
        "quoted package name with color modifier should still produce clean label"
    );
}
