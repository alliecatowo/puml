//! Regression tests for issue #1373: usecase actor edge drop + boundary placement.
//!
//! **Bug 1** (`usecase/02_with_actors`): Three actor→usecase edges declared in
//! the source (`Customer→UC1`, `Customer→UC2`, `Admin→UC3`) were visually lost
//! because the multi-rank L-bend router dropped the vertical drop segment through
//! intermediate nodes without obstacle avoidance.  All five edges must now appear
//! in the rendered SVG.
//!
//! **Bug 2** (`usecase/05_actor_generalization_system_boundary`): Four actors
//! (User, Registered User, Premium User, Administrator) were rendered INSIDE the
//! dashed `E-Commerce Platform` system-boundary frame.  After the fix, all actor
//! nodes must have an x/y position that lies outside the frame's bounding box.

use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model,
    ParsePipelineOptions,
};

fn render(src: &str) -> String {
    let opts = ParsePipelineOptions::default();
    let document = parse_with_pipeline_options(src, &opts).expect("source should parse");
    let model = normalize_family(document).expect("source should normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    artifacts
        .into_iter()
        .next()
        .map(|a| a.svg)
        .unwrap_or_default()
}

// ── Bug 1: actor→usecase edges must all appear in the SVG ────────────────────

/// `Customer --> UC2` (multi-rank, skipping UC1) must appear as a rendered
/// relation element.  Before the fix, the vertical drop from the L-bend through
/// UC1's body caused the edge to be visually obscured/merged.
#[test]
fn usecase_02_all_actor_edges_present() {
    let src = r#"
@startuml
actor Customer
actor Admin
usecase BrowseProducts as UC1
usecase PlaceOrder as UC2
usecase ManageInventory as UC3
Customer --> UC1
Customer --> UC2
Admin --> UC3
UC1 --> UC2 : leads to
UC2 --> UC3 : triggers
@enduml
"#;
    let svg = render(src);

    // All three actor→usecase relations must appear.
    assert!(
        svg.contains("data-uml-from=\"Customer\" data-uml-to=\"UC1\""),
        "Customer→UC1 edge missing from SVG"
    );
    assert!(
        svg.contains("data-uml-from=\"Customer\" data-uml-to=\"UC2\""),
        "Customer→UC2 edge missing from SVG (multi-rank drop regression)"
    );
    assert!(
        svg.contains("data-uml-from=\"Admin\" data-uml-to=\"UC3\""),
        "Admin→UC3 edge missing from SVG (multi-rank drop regression)"
    );
    // The UC1→UC2 and UC2→UC3 chain edges must also be present.
    assert!(
        svg.contains("data-uml-from=\"UC1\" data-uml-to=\"UC2\""),
        "UC1→UC2 edge missing"
    );
    assert!(
        svg.contains("data-uml-from=\"UC2\" data-uml-to=\"UC3\""),
        "UC2→UC3 edge missing"
    );
}

/// `Admin --> UC3` is a multi-rank edge (Admin at rank 0, UC3 at rank 3 after
/// transitive dependencies).  It must be rendered as a `<polyline>` (ortho
/// path) rather than a straight `<line>`, proving it gets a proper routed path.
#[test]
fn usecase_02_admin_uc3_is_polyline() {
    let src = r#"
@startuml
actor Customer
actor Admin
usecase BrowseProducts as UC1
usecase PlaceOrder as UC2
usecase ManageInventory as UC3
Customer --> UC1
Customer --> UC2
Admin --> UC3
UC1 --> UC2 : leads to
UC2 --> UC3 : triggers
@enduml
"#;
    let svg = render(src);
    // Admin→UC3 is multi-rank: must have a routed <polyline>, not a bare <line>.
    assert!(
        svg.contains(
            "<polyline class=\"uml-relation\" data-uml-from=\"Admin\" data-uml-to=\"UC3\""
        ),
        "Admin→UC3 should be a routed <polyline> for the multi-rank path"
    );
}

// ── Bug 2: actors must lie outside the system-boundary frame ─────────────────

/// In a usecase diagram with a `rectangle` system-boundary group, all actors
/// declared OUTSIDE the boundary must be positioned above the frame's top
/// edge after layout.  Verified by parsing the rendered SVG to extract actor
/// head circle cy values and the group-frame rect top-y, then asserting that
/// every actor's head cy is above the frame top.
///
/// Uses a simplified fixture with single-word actor labels to simplify SVG
/// text matching.
#[test]
fn usecase_05_actors_outside_system_boundary() {
    let src = r#"
@startuml
actor Shopper
actor Manager
actor Premium
actor Support

Shopper <|-- Manager
Manager <|-- Premium

rectangle "E-Commerce Platform" {
  usecase "Browse Catalog" as UC1
  usecase "Search Products" as UC2
  usecase "Checkout" as UC5
  usecase "Apply Promo Code" as UC6
}

Shopper --> UC1
Shopper --> UC2
Manager --> UC5
Support --> UC1
UC6 .> UC5 : extends
@enduml
"#;
    let svg = render(src);

    // Parse the group frame rect: <rect class="uml-group-frame" ... y="NNN" .../>
    let frame_y = parse_group_frame_y(&svg).expect("group frame rect must be present in SVG");

    // All four actors must have their head circle cy ABOVE the frame top.
    for actor_label in &["Shopper", "Manager", "Premium", "Support"] {
        let actor_cy = parse_actor_head_cy(&svg, actor_label).unwrap_or_else(|| {
            panic!(
                "could not find actor head circle for label {actor_label} in SVG\n\
                     (frame_y={frame_y})\nSVG (first 4000 chars):\n{}",
                &svg[..svg.len().min(4000)]
            )
        });
        assert!(
            actor_cy < frame_y,
            "Actor '{actor_label}' head (cy={actor_cy}) is inside or below the system boundary \
             frame (frame top y={frame_y}); actors must be positioned above the boundary"
        );
    }
}

/// A simpler isolated case: a single actor outside a rectangle boundary must
/// be positioned above the group frame top.  Uses a single-word actor label
/// for straightforward SVG text matching.
#[test]
fn usecase_actor_above_rectangle_boundary() {
    let src = r#"
@startuml
actor Buyer
rectangle "Online Store" {
  usecase "Browse" as UC1
  usecase "Checkout" as UC2
  usecase "Apply Coupon" as UC3
}
Buyer --> UC1
UC1 --> UC2
@enduml
"#;
    let svg = render(src);

    let frame_y = parse_group_frame_y(&svg).expect("group frame rect must be present");
    let actor_cy = parse_actor_head_cy(&svg, "Buyer")
        .expect("actor head circle must be present for label 'Buyer'");

    assert!(
        actor_cy < frame_y,
        "Actor 'Buyer' (cy={actor_cy}) should be above the system boundary (frame top y={frame_y})"
    );
}

// ── SVG parsing helpers ───────────────────────────────────────────────────────

/// Extract the `y` attribute of the first `<rect class="uml-group-frame" .../>`.
fn parse_group_frame_y(svg: &str) -> Option<i32> {
    let prefix = "class=\"uml-group-frame\"";
    let start = svg.find(prefix)?;
    // The rect element spans from some '<rect' before `prefix` to the closing '/>'.
    // Look backward from start to find '<rect'.
    let rect_start = svg[..start].rfind("<rect")?;
    let rect_end = svg[start..].find("/>")? + start;
    let rect_elem = &svg[rect_start..rect_end];
    parse_attr_i32(rect_elem, "y")
}

/// Extract the `cy` of the head circle element for a given actor display label.
///
/// Actor head circles are rendered as `<circle cx="NNN" cy="NNN" r="6" .../>`.
/// Each actor stick figure is followed by a `<text>` label element at the same
/// center x.  We scan all r="6" circles, then look for the nearest text element
/// at the same cx that contains the label as an exact word boundary.
fn parse_actor_head_cy(svg: &str, label: &str) -> Option<i32> {
    let mut search_from = 0;
    while let Some(circ_rel) = svg[search_from..].find("<circle") {
        let circ_start = search_from + circ_rel;
        let circ_end = svg[circ_start..].find("/>")? + circ_start;
        let circ_elem = &svg[circ_start..circ_end];
        if !circ_elem.contains("r=\"6\"") {
            search_from = circ_end;
            continue;
        }
        let cx = parse_attr_i32(circ_elem, "cx")?;
        let cy = parse_attr_i32(circ_elem, "cy")?;
        // Look for the text label element whose x equals cx (middle-anchored).
        let text_needle = format!("<text x=\"{cx}\" y=");
        if let Some(text_rel) = svg[circ_end..].find(&text_needle) {
            let text_pos = circ_end + text_rel;
            // Find the closing tag start — the content is between '>' (end of
            // opening tag) and '</text>'.
            let close_rel = svg[text_pos..].find("</text>")?;
            let open_close_rel = svg[text_pos..text_pos + close_rel].rfind('>')?;
            let content_start = text_pos + open_close_rel + 1;
            let content_end = text_pos + close_rel;
            let content = &svg[content_start..content_end];
            if content.trim() == label {
                return Some(cy);
            }
        }
        search_from = circ_end;
    }
    None
}

fn parse_attr_i32(elem: &str, attr: &str) -> Option<i32> {
    let needle = format!(" {attr}=\"");
    let start = elem.find(&needle)? + needle.len();
    let end = elem[start..].find('"')? + start;
    elem[start..end].parse().ok()
}
