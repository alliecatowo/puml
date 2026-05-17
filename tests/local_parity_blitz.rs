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
