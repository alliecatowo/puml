fn attr_value_in_tag(haystack: &str, marker: &str, attr: &str) -> i32 {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    let tag_start = haystack[..=marker_idx]
        .rfind('<')
        .expect("tag start should exist");
    let tag_end = haystack[marker_idx..]
        .find('>')
        .map(|idx| marker_idx + idx)
        .expect("tag end should exist");
    let tag = &haystack[tag_start..=tag_end];
    let needle = format!("{attr}=\"");
    let attr_start = tag.find(&needle).expect("attribute should exist") + needle.len();
    let rest = &tag[attr_start..];
    let end = rest.find('"').expect("attribute should terminate");
    rest[..end]
        .parse::<i32>()
        .expect("attribute should parse as i32")
}

fn attr_value_in_next_tag_after(haystack: &str, marker: &str, tag_prefix: &str, attr: &str) -> i32 {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    let tag_start = haystack[marker_idx..]
        .find(tag_prefix)
        .map(|idx| marker_idx + idx)
        .expect("next tag should exist");
    let tag_end = haystack[tag_start..]
        .find('>')
        .map(|idx| tag_start + idx)
        .expect("tag end should exist");
    let tag = &haystack[tag_start..=tag_end];
    let needle = format!("{attr}=\"");
    let attr_start = tag.find(&needle).expect("attribute should exist") + needle.len();
    let rest = &tag[attr_start..];
    let end = rest.find('"').expect("attribute should terminate");
    rest[..end]
        .parse::<i32>()
        .expect("attribute should parse as i32")
}

fn polyline_points_after(haystack: &str, marker: &str) -> Vec<(i32, i32)> {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    // Try <polyline points="..."> first (Polyline/Ortho routing modes).
    let points_key = "points=\"";
    if let Some(rel_idx) = haystack[marker_idx..].find(points_key) {
        let points_start = marker_idx + rel_idx + points_key.len();
        let rest = &haystack[points_start..];
        let points_end = rest.find('"').expect("points attribute should terminate");
        return rest[..points_end]
            .split_whitespace()
            .map(|pair| {
                let (x, y) = pair
                    .split_once(',')
                    .expect("polyline point should contain comma");
                (
                    x.parse::<i32>().expect("x coordinate should parse"),
                    y.parse::<i32>().expect("y coordinate should parse"),
                )
            })
            .collect();
    }
    // Fall back to <path d="..."> (Splines routing mode / cubic Bézier).
    // Extract all explicit (x,y) coordinate pairs by stripping SVG command
    // letters and collecting consecutive numeric token pairs.
    let d_key = "d=\"";
    let d_idx = haystack[marker_idx..]
        .find(d_key)
        .map(|idx| marker_idx + idx + d_key.len())
        .expect("path d attribute should exist");
    let rest = &haystack[d_idx..];
    let d_end = rest.find('"').expect("d attribute should terminate");
    let d = &rest[..d_end];
    let mut result: Vec<(i32, i32)> = Vec::new();
    let mut pending: Vec<f64> = Vec::new();
    for tok in d.split(|c: char| c == ',' || c.is_whitespace()) {
        if tok.is_empty() {
            continue;
        }
        if let Ok(n) = tok.parse::<f64>() {
            pending.push(n);
            if pending.len() == 2 {
                result.push((pending[0].round() as i32, pending[1].round() as i32));
                pending.clear();
            }
        } else {
            pending.clear();
        }
    }
    result
}

#[test]
fn object_relation_labels_stay_centered_on_vertical_relations() {
    let svg = puml::render_source_to_svg(
        "@startuml\nobject Order\nobject Customer\nOrder --> Customer : hasSession\n@enduml\n",
    )
    .expect("object svg should render");

    let label_x = attr_value_in_tag(&svg, ">hasSession</text>", "x");
    let target_center_x = attr_value_in_tag(&svg, ">Customer</text>", "x");

    // Allow up to 20px lateral offset — parallel-edge fanning logic in
    // src/render/family.rs (PR #775) intentionally shifts edge midpoints to
    // disambiguate fanned edges. A label drifting ≤20px from the target
    // center is still visually "centered on the vertical relation."
    let drift = (label_x - target_center_x).abs();
    assert!(
        drift <= 20,
        "expected object relation label within 20px of target center: label_x={label_x}, target_center_x={target_center_x}, drift={drift}"
    );
}

#[test]
fn object_fork_nonparallel_edges_share_midpoint_channel() {
    let svg = puml::render_source_to_svg(
        "@startuml\nobject Server {\n  host = api.example.com\n  port = 443\n}\nobject Cache {\n  host = redis.internal\n  port = 6379\n}\nobject Database {\n  host = db.internal\n  port = 5432\n}\nServer --> Cache : uses\nServer --> Database : connects\n@enduml\n",
    )
    .expect("object fork svg should render");

    let right = polyline_points_after(&svg, "data-uml-from=\"Server\" data-uml-to=\"Database\"");
    let left = polyline_points_after(&svg, "data-uml-from=\"Server\" data-uml-to=\"Cache\"");
    assert!(
        right.len() >= 3 && left.len() >= 3,
        "fork edges should keep orthogonal bend points"
    );

    // For non-parallel siblings in an object fork, both first channel bends
    // should route through the same midpoint y.
    assert_eq!(
        right[1].1, left[1].1,
        "non-parallel object fork edges should share midpoint channel y"
    );
}

#[test]
fn deployment_exotic_arrow_endpoints_use_distinct_markers() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
node A
node B
node C
node D
node E
node F
A --0 B
B --@ C
C --# D
D --+ E
E -->> F
@enduml
"#,
    )
    .expect("deployment exotic arrows should render");

    for (arrow, marker) in [
        ("--0", "arrow-circle-open"),
        ("--@", "arrow-circle-filled"),
        ("--#", "arrow-box-filled"),
        ("--+", "arrow-plus"),
        ("--&gt;&gt;", "arrow-double-open"),
    ] {
        assert!(
            svg.contains(&format!("data-uml-arrow=\"{arrow}\"")),
            "relation should preserve exotic arrow token {arrow}"
        );
        assert!(
            svg.contains(&format!("marker-end=\"url(#{marker})\"")),
            "relation {arrow} should use marker {marker}"
        );
    }
}

/// Regression test for #478: alias identifiers ('as UC1', 'as MP') must not appear as
/// visible text in the rendered usecase SVG. Only the human-readable name should render.
#[test]
fn usecase_alias_identifiers_do_not_appear_as_rendered_text() {
    // 02_with_actors pattern: aliases UC1/UC2/UC3 must be invisible
    let svg = puml::render_source_to_svg(
        r#"@startuml
actor Customer
actor Admin
usecase BrowseProducts as UC1
usecase PlaceOrder as UC2
usecase ManageInventory as UC3
Customer --> UC1
Customer --> UC2
Admin --> UC3
@enduml
"#,
    )
    .expect("usecase with aliases should render");

    // Display names must be present
    assert!(
        svg.contains(">BrowseProducts<"),
        "BrowseProducts should appear as display text"
    );
    assert!(
        svg.contains(">PlaceOrder<"),
        "PlaceOrder should appear as display text"
    );
    assert!(
        svg.contains(">ManageInventory<"),
        "ManageInventory should appear as display text"
    );

    // Alias identifiers must NOT appear as rendered text (they are internal routing ids)
    assert!(
        !svg.contains(">UC1<"),
        "alias UC1 must not render as visible text (#478)"
    );
    assert!(
        !svg.contains(">UC2<"),
        "alias UC2 must not render as visible text (#478)"
    );
    assert!(
        !svg.contains(">UC3<"),
        "alias UC3 must not render as visible text (#478)"
    );

    // 04_with_packages pattern: aliases MP/MO inside rectangles must be invisible
    let svg2 = puml::render_source_to_svg(
        r#"@startuml
actor Manager
rectangle "Back Office" {
  usecase ManageProducts as MP
  usecase ManageOrders as MO
}
Manager --> MP
Manager --> MO
MP --> MO : depends
@enduml
"#,
    )
    .expect("usecase with package aliases should render");

    assert!(
        svg2.contains(">ManageProducts<"),
        "ManageProducts should appear as display text"
    );
    assert!(
        svg2.contains(">ManageOrders<"),
        "ManageOrders should appear as display text"
    );
    assert!(
        !svg2.contains(">MP<"),
        "alias MP must not render as visible text (#478)"
    );
    assert!(
        !svg2.contains(">MO<"),
        "alias MO must not render as visible text (#478)"
    );
}

/// Regression test for #477: C4 Rel() edge labels must not be clipped or truncated.
/// Every label character must survive from source to rendered SVG text element.
#[test]
fn c4_rel_labels_are_fully_rendered_without_truncation() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
!include <C4/C4_Container>
!Person(user, "User")
!Container(spa, "SPA", "React", "Single page app")
!Container(api, "API", "FastAPI", "REST backend")
!Container(worker, "Worker", "Celery", "Background tasks")
!System_Ext(email, "Email")
!Rel(user, spa, "Uses")
!Rel(spa, api, "Calls")
!Rel(api, worker, "Enqueues")
!Rel(worker, email, "Sends via")
@enduml
"#,
    )
    .expect("C4 container diagram should render");

    // All Rel() labels must appear in full — none truncated mid-word (#477)
    assert!(
        svg.contains(">Uses<"),
        "Rel label 'Uses' must render in full (#477)"
    );
    assert!(
        svg.contains(">Calls<"),
        "Rel label 'Calls' must render in full (#477)"
    );
    assert!(
        svg.contains(">Enqueues<"),
        "Rel label 'Enqueues' must render in full (#477)"
    );
    assert!(
        svg.contains(">Sends via<"),
        "Rel label 'Sends via' must render in full (#477)"
    );

    // Microservices pattern: multi-word and slash labels
    let svg2 = puml::render_source_to_svg(
        r#"@startuml
!include <C4/C4_Container>
!Person(client, "Client")
!Container(gw, "API Gateway", "Kong", "Routes")
!Container(svc, "User Service", "Go", "Users")
!System_Ext(db, "User DB")
!Rel(client, gw, "API calls")
!Rel(gw, svc, "Routes")
!Rel(svc, db, "Reads/writes")
@enduml
"#,
    )
    .expect("C4 microservices diagram should render");

    assert!(
        svg2.contains(">API calls<"),
        "Rel label 'API calls' must render in full (#477)"
    );
    assert!(
        svg2.contains(">Routes<"),
        "Rel label 'Routes' must render in full (#477)"
    );
    assert!(
        svg2.contains(">Reads/writes<"),
        "Rel label 'Reads/writes' must render in full (#477)"
    );
}

#[test]
fn deployment_svg_keeps_rightmost_node_inside_viewbox_with_gutter() {
    let svg = puml::render_source_to_svg(
        "@startuml\nnode WebServer\nnode AppServer\nnode DBServer\nWebServer --> AppServer : HTTP\nAppServer --> DBServer : readsRequests\n@enduml\n",
    )
    .expect("deployment svg should render");

    let viewbox_width = attr_value_in_tag(&svg, "<svg ", "width");
    let node_x = attr_value_in_next_tag_after(
        &svg,
        "data-uml-id=\"DBServer\"",
        "<rect class=\"uml-node uml-deployment-shape\"",
        "x",
    );
    let node_w = attr_value_in_next_tag_after(
        &svg,
        "data-uml-id=\"DBServer\"",
        "<rect class=\"uml-node uml-deployment-shape\"",
        "width",
    );
    let label_x = attr_value_in_tag(&svg, ">readsRequests</text>", "x");

    assert!(
        viewbox_width - (node_x + node_w) >= 40,
        "expected rightmost deployment node to keep at least 40px gutter: width={viewbox_width}, node_right={}",
        node_x + node_w
    );
    assert!(
        viewbox_width - label_x >= 80,
        "expected deployment relation label to stay clear of right canvas edge: width={viewbox_width}, label_x={label_x}"
    );
}
