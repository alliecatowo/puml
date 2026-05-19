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

#[test]
fn object_relation_labels_stay_centered_on_vertical_relations() {
    let svg = puml::render_source_to_svg(
        "@startuml\nobject Order\nobject Customer\nOrder --> Customer : hasSession\n@enduml\n",
    )
    .expect("object svg should render");

    let label_x = attr_value_in_tag(&svg, ">hasSession</text>", "x");
    let target_center_x = attr_value_in_tag(&svg, ">Customer</text>", "x");

    assert!(
        label_x == target_center_x,
        "expected object relation label to stay centered on the vertical relation: label_x={label_x}, target_center_x={target_center_x}"
    );
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
