#[test]
fn sequence_teoz_arrow_style_parity_renders_head_variants_and_response_labels() {
    let src = r##"@startuml
!pragma teoz true
skinparam SequenceMessageAlign center
skinparam ResponseMessageBelowArrow true
Alice -[#red,dotted]> Bob : dotted red
Bob ->> Alice : async reply
Alice ->o Bob : create-like lollipop
Bob ->x Alice : lost message
@enduml
"##;

    let svg = puml::render_source_to_svg(src).expect("sequence render should succeed");

    assert!(svg.contains("stroke=\"#ff0000\""));
    assert!(svg.contains("stroke-dasharray=\"2 4\""));
    assert!(svg.contains("<polyline points="));
    assert!(svg.contains("<circle cx="));
    assert!(svg.contains("<g stroke="));
    assert!(svg.contains("text-anchor=\"middle\""));
}

#[test]
fn class_object_and_usecase_partial_member_rows_merge_into_declared_nodes() {
    let class_svg = puml::render_source_to_svg(
        r#"@startuml
class Account
Account : +id: UUID
Account : {method} +close()
Account --> Ledger : posts
@enduml
"#,
    )
    .expect("class render should succeed");
    assert_eq!(class_svg.matches(">class Account<").count(), 1);
    assert!(class_svg.contains("+id: UUID"));
    assert!(class_svg.contains("+close()"));

    let object_svg = puml::render_source_to_svg(
        r#"@startuml
object order
order : status = paid
@enduml
"#,
    )
    .expect("object render should succeed");
    assert_eq!(object_svg.matches(">order<").count(), 1);
    assert!(object_svg.contains("status = paid"));

    let usecase_svg = puml::render_source_to_svg(
        r#"@startuml
usecase Checkout
Checkout : primary actor: Shopper
@enduml
"#,
    )
    .expect("usecase render should succeed");
    assert_eq!(usecase_svg.matches(">Checkout<").count(), 1);
    assert!(usecase_svg.contains("primary actor: Shopper"));
}

#[test]
fn core_uml_advanced_rows_render_across_component_deployment_state_activity_timing() {
    let component_svg = puml::render_source_to_svg(
        r#"@startuml
skinparam componentArrowColor navy
component "API Gateway" as api
interface "Orders" as orders
api -down-> orders : exposes
@enduml
"#,
    )
    .expect("component render should succeed");
    assert!(component_svg.contains("API Gateway"));
    assert!(component_svg.contains("Orders"));
    assert!(component_svg.contains("exposes"));

    let deployment_svg = puml::render_source_to_svg(
        r#"@startuml
node "Kubernetes" as k8s
database "Orders DB" as db
k8s -right-> db : stores
@enduml
"#,
    )
    .expect("deployment render should succeed");
    assert!(deployment_svg.contains("Kubernetes"));
    assert!(deployment_svg.contains("Orders DB"));
    assert!(deployment_svg.contains("stores"));

    let state_svg = puml::render_source_to_svg(
        r#"@startuml
state Ready
Ready : entry / boot
Ready --> [H*] : resume
[H*] --> Ready
@enduml
"#,
    )
    .expect("state render should succeed");
    assert!(state_svg.contains("entry / boot"));
    assert!(state_svg.contains(">H*<"));

    let activity_svg = puml::render_source_to_svg(
        r#"@startuml
start
partition Worker {
:load;
split
:fast path;
split again
:slow path;
end split
}
stop
@enduml
"#,
    )
    .expect("activity render should succeed");
    assert!(activity_svg.contains("partition: Worker"));
    assert!(activity_svg.contains("split again"));

    let timing_svg = puml::render_source_to_svg(
        r#"@startuml
clock "Scheduler" as CLK with period 10, pulse 4
binary FLAG
@0 FLAG is off
@5 FLAG is on
@10 checkpoint
@enduml
"#,
    )
    .expect("timing render should succeed");
    assert!(timing_svg.contains("Scheduler"));
    assert!(timing_svg.contains("period 10"));
    assert!(timing_svg.contains("checkpoint"));
}

#[test]
fn core_uml_relation_namespace_lollipop_and_activity_beta_parity_depth() {
    let class_svg = puml::render_source_to_svg(
        r#"@startuml
class "Order-Service"
class "Line-Item"
"Order-Service" "1" --> "0..*" "Line-Item": contains
@enduml
"#,
    )
    .expect("class relation render should succeed");
    assert!(class_svg.contains("contains"));
    assert!(class_svg.contains("&gt;1&lt;") || class_svg.contains(">1<"));
    assert!(class_svg.contains("0..*"));

    let component_svg = puml::render_source_to_svg(
        r#"@startuml
skinparam interfaceBackgroundColor lightblue
namespace Edge {
  component API
  interface "Orders" as Orders
}
API --() Orders: provides
@enduml
"#,
    )
    .expect("component namespace render should succeed");
    assert!(component_svg.contains("namespace Edge"));
    assert!(component_svg.contains("API"));
    assert!(component_svg.contains("Orders"));
    assert!(component_svg.contains("provides"));

    let activity_svg = puml::render_source_to_svg(
        r#"@startuml
start
if (ready?) then (yes)
elseif (warm?) then (maybe)
continue
break
endif
repeat
:again;
repeat while (more?)
end repeat
stop
@enduml
"#,
    )
    .expect("activity beta render should succeed");
    assert!(activity_svg.contains("elseif warm? / maybe"));
    assert!(activity_svg.contains("continue"));
    assert!(activity_svg.contains("break"));
    assert!(activity_svg.contains("end repeat"));
}

#[test]
fn core_uml_family_relation_bracket_styles_survive_to_svg() {
    let class_svg = puml::render_source_to_svg(
        r##"@startuml
skinparam classArrowColor #111111
class API
class Worker
class Hidden
API -[#red,dashed,thickness=4]-> Worker : styled
Worker -[hidden]-> Hidden : layout only
@enduml
"##,
    )
    .expect("styled class relation should render");
    assert!(class_svg.contains("stroke=\"#ff0000\""));
    assert!(class_svg.contains("stroke-width=\"4\""));
    assert!(class_svg.contains("stroke-dasharray=\"5 3\""));
    assert!(class_svg.contains("visibility=\"hidden\""));

    let component_svg = puml::render_source_to_svg(
        r##"@startuml
component API
component DB
API -[#008800,bold]-> DB : persists
@enduml
"##,
    )
    .expect("styled component relation should render");
    assert!(component_svg.contains("stroke=\"#008800\""));
    assert!(component_svg.contains("stroke-width=\"3\""));
    assert!(component_svg.contains("persists"));
}

#[test]
fn core_uml_relation_stereotypes_and_cardinality_survive_to_svg() {
    let component_svg = puml::render_source_to_svg(
        r##"@startuml
package "Edge" {
  component "API Gateway" as api
  port "HTTP" as http
  interface "Orders" as orders
}
api "1" -[#008800,thickness=3]-> "0..*" orders <<REST>> : exposes
http --> api :binds
@enduml
"##,
    )
    .expect("component relation metadata should render");
    assert!(component_svg.contains("package Edge"));
    assert!(component_svg.contains("api"));
    assert!(component_svg.contains("http"));
    assert!(component_svg.contains("&lt;&lt;REST&gt;&gt;"));
    assert!(component_svg.contains("exposes"));
    assert!(component_svg.contains("0..*"));
    assert!(component_svg.contains("stroke=\"#008800\""));
    assert!(component_svg.contains("stroke-width=\"3\""));

    let deployment_svg = puml::render_source_to_svg(
        r##"@startuml
node "Cluster" as cluster
database "Orders DB" as db
cluster --> db : <<deploys>> stores
@enduml
"##,
    )
    .expect("deployment relation metadata should render");
    assert!(deployment_svg.contains("Cluster"));
    assert!(deployment_svg.contains("Orders DB"));
    assert!(deployment_svg.contains("&lt;&lt;deploys&gt;&gt;"));
    assert!(deployment_svg.contains("stores"));
}
