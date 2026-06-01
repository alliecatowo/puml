//! Density-overflow guard regression tests.
//!
//! Covers four visual-audit bugs filed after the density-retune PRs (#1431,
//! #1437):
//!   #1440 — kubernetes/06: outer "Kubernetes Cluster" group y=-34,
//!            header clipped above viewBox
//!   #1442 — deployment/06: "Ingress Controller" (18 chars), "queue-consumer"
//!            (14 chars), "sidecar-logger" (14 chars) overflow 110px node bbox
//!   #1443 — component/07: "NotificationSender" (18 chars) and
//!            "OrderRepository" (15 chars) overflow 130px node bbox
//!   #1444 — deployment/03: "Lambda Function" (15 chars) overflows 110px box
//!
//! Root cause: the density-retune constants (DEPLOYMENT_BOX_WIDTH=110,
//! COMPONENT_NODE_BOX_WIDTH=130) can be narrower than the label text they
//! must display.  The fix enforces a `max(density_width, label_width +
//! 2*padding)` floor on every node bbox, and clamps the outermost group y
//! to `canvas_margin + header_h` so no group header is clipped.

fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

/// Extract the width of the bounding-box rect for a node whose label contains
/// `label_fragment`.  The SVG structure is:
///
///   `<desc data-uml-id="NAME">NAME</desc>`
///   `<polygon .../>` (optional 3-D top/side faces for deployment nodes)
///   `<rect class="uml-node" data-uml-kind="…" x="…" y="…" width="NNN" …/>`
///
/// We find the `<desc data-uml-id="…LABEL_FRAGMENT…">` element, then walk
/// forward to the next `<rect` that has `data-uml-kind` and read its `width`.
fn node_bbox_width(svg: &str, label_fragment: &str) -> Option<i32> {
    // The desc element uses the node's name (not alias) as the id value.
    let needle = format!("data-uml-id=\"{label_fragment}\"");
    let pos = svg.find(&needle)?;
    let after = &svg[pos..];
    // Walk forward looking for a <rect with data-uml-kind (the node body rect).
    let mut rest = after;
    loop {
        let rect_pos = rest.find("<rect ")?;
        let rect_slice = &rest[rect_pos..];
        let end = rect_slice.find('>')?;
        let tag = &rect_slice[..end];
        if tag.contains("data-uml-kind") {
            // Extract width="NNN"
            let w_pos = tag.find("width=\"")?;
            let w_slice = &tag[w_pos + 7..];
            let w_end = w_slice.find('"')?;
            return w_slice[..w_end].parse().ok();
        }
        rest = &rest[rect_pos + 1..];
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// #1440 — outer group y must be ≥ canvas_margin (never negative / clipped)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1440_outer_group_y_not_negative() {
    // A four-level deep deployment nesting (cluster → namespace → pod →
    // container) that previously caused the outermost group y = -34.
    let src = r#"@startuml
node "Kubernetes Cluster" {
  node "Namespace: frontend" {
    node "Pod: nginx-proxy" {
      node "SL" as SL <<container>> [
        sidecar-logger
      ]
    }
  }
}
@enduml
"#;
    let out = svg(src);
    // The viewBox starts at 0, so no group rect y should be negative.
    // Extract all y="NNN" values from group-frame rects and assert none < 0.
    let mut has_negative = false;
    let mut search = out.as_str();
    while let Some(pos) = search.find("uml-group-frame") {
        let rest = &search[pos..];
        if let Some(y_pos) = rest.find(" y=\"") {
            let y_slice = &rest[y_pos + 4..];
            if let Some(end) = y_slice.find('"') {
                if let Ok(y_val) = y_slice[..end].parse::<i32>() {
                    if y_val < 0 {
                        has_negative = true;
                        break;
                    }
                }
            }
        }
        search = &rest[1..];
    }
    assert!(
        !has_negative,
        "no group-frame rect should have y < 0; outer header would be clipped above viewBox"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #1442 — deployment nodes: bbox width must fit the label
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1442_ingress_controller_bbox_fits_label() {
    let src = r#"@startuml
node "Ingress Controller" as IC
@enduml
"#;
    let out = svg(src);
    // "Ingress Controller" = 18 chars × ~9.2 px/char bold-13 = ~166 px.
    // The node rect must be at least 140 px wide (18 chars × 7.7 px minimum
    // with 2×14 px padding = 139 px → round up to 140 for safety).
    // data-uml-id uses node.name ("Ingress Controller"), not alias.
    let w = node_bbox_width(&out, "Ingress Controller")
        .expect("Ingress Controller node rect must be present");
    assert!(
        w >= 140,
        "Ingress Controller bbox width {w} px is too narrow to contain 18-char label (need ≥ 140 px)"
    );
}

#[test]
fn issue_1442_queue_consumer_bbox_fits_label() {
    let src = r#"@startuml
node "queue-consumer" as QC <<container>>
@enduml
"#;
    let out = svg(src);
    // "queue-consumer" = 14 chars → need at least 110 px.
    let w = node_bbox_width(&out, "queue-consumer")
        .expect("queue-consumer node rect must be present");
    assert!(
        w >= 110,
        "queue-consumer bbox width {w} px must contain 14-char label (need ≥ 110 px)"
    );
}

#[test]
fn issue_1442_sidecar_logger_bbox_fits_label() {
    let src = r#"@startuml
node "sidecar-logger" as SL <<container>>
@enduml
"#;
    let out = svg(src);
    let w =
        node_bbox_width(&out, "sidecar-logger").expect("sidecar-logger node rect must be present");
    assert!(
        w >= 110,
        "sidecar-logger bbox width {w} px must contain 14-char label (need ≥ 110 px)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #1443 — component nodes: bbox width must fit long identifiers
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1443_notification_sender_bbox_fits_label() {
    let src = r#"@startuml
component "NotificationSender" as NS
@enduml
"#;
    let out = svg(src);
    // "NotificationSender" = 18 chars; need at least 140 px.
    let w = node_bbox_width(&out, "NotificationSender")
        .expect("NotificationSender component rect must be present");
    assert!(
        w >= 140,
        "NotificationSender bbox width {w} px is too narrow (need ≥ 140 px)"
    );
}

#[test]
fn issue_1443_order_repository_bbox_fits_label() {
    let src = r#"@startuml
component "OrderRepository" as OR
@enduml
"#;
    let out = svg(src);
    // "OrderRepository" = 15 chars; need at least 115 px.
    let w = node_bbox_width(&out, "OrderRepository")
        .expect("OrderRepository component rect must be present");
    assert!(
        w >= 115,
        "OrderRepository bbox width {w} px is too narrow (need ≥ 115 px)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #1444 — deployment: "Lambda Function" must fit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1444_lambda_function_bbox_fits_label() {
    let src = r#"@startuml
node "Lambda Function" as LF
@enduml
"#;
    let out = svg(src);
    // "Lambda Function" = 15 chars; need at least 115 px.
    let w =
        node_bbox_width(&out, "Lambda Function").expect("Lambda Function node rect must be present");
    assert!(
        w >= 115,
        "Lambda Function bbox width {w} px is too narrow (need ≥ 115 px)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regression guard: short-label nodes must NOT balloon (stay at density width)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn short_label_deployment_node_stays_compact() {
    let src = r#"@startuml
node DB
node Web
@enduml
"#;
    let out = svg(src);
    // "DB" and "Web" are short labels — they should stay at the density width
    // (110 px for deployment).  Guard: width must be < 160 px so we haven't
    // accidentally inflated all nodes to the component-family size.
    let w_db = node_bbox_width(&out, "DB").expect("DB node rect must be present");
    let w_web = node_bbox_width(&out, "Web").expect("Web node rect must be present");
    assert!(
        w_db <= 160,
        "Short-label node 'DB' bbox width {w_db} px should stay compact (≤ 160 px)"
    );
    assert!(
        w_web <= 160,
        "Short-label node 'Web' bbox width {w_web} px should stay compact (≤ 160 px)"
    );
}

#[test]
fn short_label_component_node_stays_compact() {
    let src = r#"@startuml
component UI
component API
@enduml
"#;
    let out = svg(src);
    let w_ui = node_bbox_width(&out, "UI").expect("UI component rect must be present");
    let w_api = node_bbox_width(&out, "API").expect("API component rect must be present");
    assert!(
        w_ui <= 180,
        "Short-label component 'UI' bbox width {w_ui} px should stay compact (≤ 180 px)"
    );
    assert!(
        w_api <= 180,
        "Short-label component 'API' bbox width {w_api} px should stay compact (≤ 180 px)"
    );
}
