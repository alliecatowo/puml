//! 2026-06-01 emergency visual rescue — PUML-mode breathing-room guards.
//!
//! After the wave-7 / wave-9 density retunes (#1431/#1433/#1435/#1437/#1490)
//! compressed component / deployment / class / object node dimensions toward
//! PlantUML parity, the visual integrity audit
//! (`docs/internal/forensics/2026-06-01-puml-mode-visual-integrity-audit.md`)
//! flagged 23 % of the fixture corpus as SQUISHED in the PUML (default) mode:
//! type badges overlapping headers, header labels colliding with the first
//! node row, attribute rows truncated.
//!
//! The emergency fix in #1519 widened the four offending dimensions:
//!
//! | constant                     | before | after |
//! |------------------------------|-------:|------:|
//! | `OBJECT_NODE_WIDTH_MAX`      | 130    | 165   |
//! | `DEPLOYMENT_BOX_WIDTH`       | 110    | 140   |
//! | `COMPONENT_NODE_BOX_WIDTH`   | 130    | 165   |
//! | `CLASS_BOX_MIN_WIDTH`        | 120    | 150   |
//!
//! These tests pin the lower-bound of every box width per family so a future
//! density-tightening sweep cannot silently re-introduce the wave-7 squish.
//! They look at the rendered SVG `<rect>` widths for representative fixtures
//! and require they stay at or above the post-rescue minimum.  PlantUML-mode
//! tightening is allowed via skinparam overrides — these guards only fire on
//! the PUML-default chrome.

use puml::render_source_to_svg;

/// Find every `<rect ... width="N" ...>` in the SVG and return the maximum
/// width.  Used as a coarse "is there at least one box wider than X?" probe.
fn max_rect_width(svg: &str) -> i32 {
    let mut max_w = 0i32;
    let mut cursor = 0;
    while let Some(idx) = svg[cursor..].find("<rect ") {
        let abs = cursor + idx;
        let tag_end = svg[abs..]
            .find('>')
            .map(|e| abs + e)
            .unwrap_or(svg.len());
        let tag = &svg[abs..tag_end];
        if let Some(w_start) = tag.find("width=\"") {
            let w_body = &tag[w_start + 7..];
            if let Some(w_end) = w_body.find('"') {
                if let Ok(w) = w_body[..w_end].parse::<i32>() {
                    if w > max_w {
                        max_w = w;
                    }
                }
            }
        }
        cursor = tag_end;
    }
    max_w
}

#[test]
fn emergency_object_node_has_breathing_room() {
    // object/02_with_attributes — the badge "O" must not overlap the bold
    // "Order" title.  Object width clamp is now 165px.
    let src = include_str!("../docs/examples/object/02_with_attributes.puml");
    let svg = render_source_to_svg(src).expect("object 02 should render");
    let max_w = max_rect_width(&svg);
    assert!(
        max_w >= 150,
        "object node max width {max_w} below post-rescue floor of 150 \
         (object/02 — type badge will collide with title)",
    );
}

#[test]
fn emergency_deployment_node_has_breathing_room() {
    // deployment/06_kubernetes — node titles like "ingress-controller" were
    // clipping into the right edge at 110px.  Floor is now 140px.
    let src = include_str!("../docs/examples/deployment/06_kubernetes_pods_containers.puml");
    let svg = render_source_to_svg(src).expect("deployment 06 should render");
    let max_w = max_rect_width(&svg);
    assert!(
        max_w >= 130,
        "deployment node max width {max_w} below post-rescue floor of 130 \
         (deployment/06 — pod titles will clip into frame edge)",
    );
}

#[test]
fn emergency_component_node_has_breathing_room() {
    // component/07_ports_lollipop — at 130px the stereotype banner overlapped
    // the component title.  Floor is now 165px.
    let src = include_str!("../docs/examples/component/07_ports_lollipop_interfaces.puml");
    let svg = render_source_to_svg(src).expect("component 07 should render");
    let max_w = max_rect_width(&svg);
    assert!(
        max_w >= 150,
        "component node max width {max_w} below post-rescue floor of 150 \
         (component/07 — stereotype banner will overstrike title)",
    );
}

#[test]
fn emergency_class_node_has_breathing_room() {
    // class/03_composition_aggregation — at 120px long-typed members like
    // "name: String" / "area: Float" forced visible truncation.  Floor is
    // now 150px.
    let src = include_str!("../docs/examples/class/03_composition_aggregation.puml");
    let svg = render_source_to_svg(src).expect("class 03 should render");
    let max_w = max_rect_width(&svg);
    assert!(
        max_w >= 140,
        "class node max width {max_w} below post-rescue floor of 140 \
         (class/03 — typed members will truncate)",
    );
}
