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
    assert!(svg.contains("data-sequence-arrow-end=\"circle\""));
    assert!(svg.contains("data-sequence-arrow-end=\"cross\""));
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
    assert_eq!(class_svg.matches(">Account<").count(), 1);
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
    // Wave 3-D (#492 #533): swimlane name renders as a header label; the
    // `split`/`split again` syntax keywords are layout directives, not visible
    // text. Assert on the action labels and lane name instead.
    assert!(activity_svg.contains("Worker"));
    assert!(activity_svg.contains("load"));
    assert!(activity_svg.contains("fast path"));
    assert!(activity_svg.contains("slow path"));

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
fn json_yaml_projection_boxes_render_in_component_and_deployment_contexts() {
    let component = r#"@startuml
component API
json $cfg {
  "service": {"name": "orders", "replicas": 3},
  "ports": [8080, 9090]
}
API --> $cfg : config
@enduml
"#;
    let component_svg =
        puml::render_source_to_svg(component).expect("component projection should render");
    assert!(component_svg.contains("class=\"uml-projection\""));
    assert!(component_svg.contains("data-uml-projection=\"$cfg\""));
    assert!(component_svg.contains("data-uml-projection-format=\"json\""));
    assert!(component_svg.contains("data-uml-projection-row-label=\"service\""));
    assert!(component_svg.contains("data-uml-projection-row-label=\"name: orders\""));
    assert!(component_svg.contains("data-uml-projection-row-label=\"ports\""));
    assert!(component_svg.contains("data-uml-projection-row-label=\"[1]: 9090\""));
    assert!(component_svg.contains("class=\"uml-projection-connector\""));

    let deployment = r#"@startuml
node Runtime
yaml $settings {
  image: puml
  resources:
    cpu: 2
}
Runtime --> $settings : reads
@enduml
"#;
    let deployment_svg =
        puml::render_source_to_svg(deployment).expect("deployment projection should render");
    assert!(deployment_svg.contains("data-uml-projection=\"$settings\""));
    assert!(deployment_svg.contains("data-uml-projection-format=\"yaml\""));
    assert!(deployment_svg.contains("image: puml"));
    assert!(deployment_svg.contains("data-uml-projection-row-label=\"resources\""));
    assert!(deployment_svg.contains("data-uml-projection-row-label=\"cpu: 2\""));
    assert!(deployment_svg.contains("class=\"uml-projection-connector\""));
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
    // Wave 3-D (#427 #533): guard text floats on outgoing arrows; "endif"
    // and "elseif" are layout keywords not emitted as visible text. The
    // remaining visible content is the first if condition, branch guards, and
    // explicit break/continue action keywords.
    assert!(activity_svg.contains("ready?"));
    assert!(activity_svg.contains("yes"));
    assert!(activity_svg.contains("continue"));
    assert!(activity_svg.contains("break"));
    assert!(activity_svg.contains("again"));
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
fn core_uml_directional_relation_styles_and_activity_note_forms_render() {
    let class_svg = puml::render_source_to_svg(
        r##"@startuml
class API
class Worker
class Audit
API -[#red;line.dashed;line.thickness=5]right-> Worker : emits
Worker -[#blue;line.bold]up-> Audit : reports
@enduml
"##,
    )
    .expect("directional styled class relations should render");
    assert!(class_svg.contains("data-uml-direction=\"right\""));
    assert!(class_svg.contains("data-uml-direction=\"up\""));
    assert!(class_svg.contains("stroke=\"#ff0000\""));
    assert!(class_svg.contains("stroke-width=\"5\""));
    assert!(class_svg.contains("stroke-dasharray=\"5 3\""));
    assert!(class_svg.contains("stroke=\"#0000ff\""));
    assert!(class_svg.contains("reports"));

    let component_svg = puml::render_source_to_svg(
        r##"@startuml
package "Edge" {
  port "HTTP" as http
  interface "Orders API" as orders
  component "Gateway" as gateway
  http -[#008800;line.thick]down-> gateway : mounted
  gateway -[#orange;line.dotted]left-> orders : provides
}
@enduml
"##,
    )
    .expect("component ports/interfaces direction styles should render");
    assert!(component_svg.contains("port"));
    assert!(component_svg.contains("Orders API"));
    assert!(component_svg.contains("data-uml-direction=\"down\""));
    assert!(component_svg.contains("data-uml-direction=\"left\""));
    assert!(component_svg.contains("stroke=\"#008800\""));
    assert!(component_svg.contains("stroke=\"#ffa500\""));

    let activity_svg = puml::render_source_to_svg(
        r#"@startuml
start
if (ready?) is (yes) then (fast)
:ship;
else (slow)
floating note right: manual review
endif
repeat
:retry;
repeat while (again?) is (yes) not (no)
stop
@enduml
"#,
    )
    .expect("activity note and branch labels should render");
    assert!(
        activity_svg.contains("ready?"),
        "condition text should appear in diamond"
    );
    assert!(
        activity_svg.contains("yes / fast"),
        "then-guard should float on outgoing arrow"
    );
    assert!(activity_svg.contains("manual review"));
    assert!(
        activity_svg.contains("again?"),
        "repeat condition should appear in diamond"
    );
    assert!(
        activity_svg.contains("yes / no"),
        "repeat guard labels should float on arrow"
    );
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

#[test]
fn core_uml_next_wave_component_state_activity_and_skinparam_parity() {
    let component_svg = puml::render_source_to_svg(
        r##"@startuml
skinparam deploymentArrowColor teal
skinparam interfaceColor lightblue
package "Edge" {
  component "API Gateway" as api <<service>>
  interface "Orders API" as orders <<REST>>
  portin "HTTP" as http <<inbound>>
}
api -[#red,dashed]right-> orders <<provides>> : publishes
http |-- api : mounted
@enduml
"##,
    )
    .expect("component next-wave render should succeed");
    assert!(component_svg.contains("&lt;&lt;service&gt;&gt;"));
    assert!(component_svg.contains("&lt;&lt;REST&gt;&gt;"));
    assert!(component_svg.contains("&lt;&lt;inbound&gt;&gt;"));
    assert!(component_svg.contains("&lt;&lt;provides&gt;&gt;"));
    assert!(component_svg.contains("stroke=\"#ff0000\""));
    assert!(component_svg.contains("stroke-dasharray=\"5 3\""));
    assert!(component_svg.contains("#add8e6"));
    assert!(component_svg.contains("mounted"));

    let state_svg = puml::render_source_to_svg(
        r#"@startuml
state Ready {
  entry / setup
  do / work
  exit / cleanup
}
Ready --> [*] : done
@enduml
"#,
    )
    .expect("state bare internals render should succeed");
    assert!(state_svg.contains("entry / setup"));
    assert!(state_svg.contains("do / work"));
    assert!(state_svg.contains("exit / cleanup"));

    let activity_svg = puml::render_source_to_svg(
        r##"@startuml
skinparam activityDiamondColor #ffeeaa
switch (kind?)
case (fast)
:fast path;
split
:one;
split again
:two;
end split
kill
@enduml
"##,
    )
    .expect("activity beta controls should detect and render");
    assert!(activity_svg.contains("switch kind?"));
    // Wave 3-D (#533): "(else)" / "(endif)" no longer render as literal text;
    // the branch label "fast" still appears on the outgoing arrow.
    assert!(activity_svg.contains("fast"));
    assert!(activity_svg.contains("#ffeeaa"));
    assert!(activity_svg.contains("kill"));
}

#[test]
fn core_uml_package_alias_and_skinparam_alias_parity() {
    let usecase_svg = puml::render_source_to_svg(include_str!(
        "fixtures/families/valid_usecase_package_aliases.puml"
    ))
    .expect("usecase package render should succeed");
    assert!(usecase_svg.contains("package Checkout Domain"));
    assert!(usecase_svg.contains("Checkout Domain::Shopper"));
    assert!(usecase_svg.contains("Place Order"));
    assert!(usecase_svg.contains("&lt;&lt;extend&gt;&gt;"));

    let component_svg = puml::render_source_to_svg(include_str!(
        "fixtures/families/valid_component_bracketed_ports.puml"
    ))
    .expect("bracketed component declaration render should succeed");
    assert!(component_svg.contains("Inventory API"));
    assert!(component_svg.contains("HTTPS"));
    assert!(component_svg.contains("&lt;&lt;service&gt;&gt;"));
    assert!(component_svg.contains("&lt;&lt;inbound&gt;&gt;"));
    assert!(component_svg.contains("#add8e6"));
    assert!(component_svg.contains("binds"));
}

#[test]
fn core_uml_inline_fill_styles_render_for_class_component_and_deployment_nodes() {
    let class_svg = puml::render_source_to_svg(
        r##"@startuml
class Account #palegreen
class Ledger #ffeeaa
Account --> Ledger : records
@enduml
"##,
    )
    .expect("class inline style render should succeed");
    assert!(class_svg.contains("#98fb98"));
    assert!(class_svg.contains("#ffeeaa"));
    assert!(class_svg.contains("records"));

    let object_svg = puml::render_source_to_svg(
        r##"@startuml
object "Order Snapshot" as snap #ffeeaa
object Archive #palegreen
snap --> Archive : stores
@enduml
"##,
    )
    .expect("object inline style render should succeed");
    assert!(object_svg.contains("#ffeeaa"));
    assert!(object_svg.contains("#98fb98"));

    let usecase_svg = puml::render_source_to_svg(
        r##"@startuml
actor Customer
usecase (Checkout) as UC #lightblue
Customer ..> UC : uses
@enduml
"##,
    )
    .expect("usecase inline style render should succeed");
    assert!(usecase_svg.contains("#add8e6"));
    assert!(usecase_svg.contains("uses"));

    let component_svg = puml::render_source_to_svg(
        r##"@startuml
component API #aliceblue
interface Orders #palegreen
port HTTP #ffeeaa
API --> Orders : exposes
HTTP --> API : binds
@enduml
"##,
    )
    .expect("component inline style render should succeed");
    assert!(component_svg.contains("#f0f8ff"));
    assert!(component_svg.contains("#98fb98"));
    assert!(component_svg.contains("#ffeeaa"));

    let deployment_svg = puml::render_source_to_svg(
        r##"@startuml
node Cluster #aliceblue
package Edge #ffeeaa
database Orders #palegreen
Cluster --> Orders : hosts
@enduml
"##,
    )
    .expect("deployment inline style render should succeed");
    assert!(deployment_svg.contains("#ffeeaa"));
    assert!(deployment_svg.contains("hosts"));
}

#[test]
fn skinparam_family_compatibility_chunk_reaches_svg_for_non_sequence_families() {
    let class_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_class_object_usecase_compat.puml"
    ))
    .expect("class/object/usecase skinparam fixture should render");
    assert!(class_svg.contains("#fef3c7"));
    assert!(class_svg.contains("#7c2d12"));
    assert!(class_svg.contains("#831843"));
    assert!(class_svg.contains("#0f766e"));
    assert!(class_svg.contains("FiraCode"));
    assert!(class_svg.contains("font-size=\"15\""));

    let component_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_component_deployment_compat.puml"
    ))
    .expect("component/deployment skinparam fixture should render");
    assert!(component_svg.contains("#ecfeff"));
    assert!(component_svg.contains("#0e7490"));
    assert!(component_svg.contains("#064e3b"));

    let state_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_state_compat.puml"
    ))
    .expect("state skinparam fixture should render");
    assert!(state_svg.contains("#fef9c3"));
    assert!(state_svg.contains("#854d0e"));
    assert!(state_svg.contains("#1f2937"));
    assert!(state_svg.contains("#7c2d12"));

    let activity_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_activity_compat.puml"
    ))
    .expect("activity skinparam fixture should render");
    assert!(activity_svg.contains("#ecfdf5"));
    assert!(activity_svg.contains("#047857"));
    assert!(activity_svg.contains("#064e3b"));
    assert!(activity_svg.contains("#bbf7d0"));

    let timing_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_timing_chart_salt_compat.puml"
    ))
    .expect("timing skinparam fixture should render");
    assert!(timing_svg.contains("data-timing-style=\"#f8fafc #0f766e #ccfbf1"));
    assert!(timing_svg.contains("#134e4a"));

    let chart_svg = puml::render_source_to_svg_for_family(
        include_str!("fixtures/styling/valid_skinparam_chart_compat.puml"),
        puml::DiagramFamily::Chart,
    )
    .expect("chart skinparam fixture should render");
    assert!(chart_svg.contains("#fff7ed"));
    assert!(chart_svg.contains("#9a3412"));
    assert!(chart_svg.contains("#fed7aa"));
    assert!(chart_svg.contains("#2563eb"));
    assert!(chart_svg.contains("#7c2d12"));

    let salt_svg = puml::render_source_to_svg(include_str!(
        "fixtures/styling/valid_skinparam_salt_compat.puml"
    ))
    .expect("salt skinparam fixture should render");
    assert!(salt_svg.contains("#f8fafc"));
    assert!(salt_svg.contains("#ecfeff"));
    assert!(salt_svg.contains("#0e7490"));
    assert!(salt_svg.contains("#164e63"));
}

#[test]
fn core_uml_advanced_metadata_wave_for_members_states_and_activity_beta() {
    let class_svg = puml::render_source_to_svg(
        r#"@startuml
class Account <<entity>> {
  +id: UUID
  -secret: String
  #protected_id: String
  ~package_id: String
  {static} +find(id)
  {abstract} +close()
}
@enduml
"#,
    )
    .expect("class metadata render should succeed");
    assert!(class_svg.contains("data-uml-visibility=\"public\""));
    assert!(class_svg.contains("data-uml-visibility=\"private\""));
    assert!(class_svg.contains("data-uml-visibility=\"protected\""));
    assert!(class_svg.contains("data-uml-visibility=\"package\""));
    assert!(class_svg.contains("data-uml-modifier=\"static\""));
    assert!(class_svg.contains("data-uml-modifier=\"abstract\""));
    // Fix #551: user stereotypes now render as guillemet labels in the header
    assert!(class_svg.contains("\u{ab}entity\u{bb}"));

    let state_svg = puml::render_source_to_svg(
        r##"@startuml
state "Composite" as Comp {
  [H]
  state Idle
  ||
  state Active
}
choice Choose
fork Split
join Merge
[*] -[#red,dashed,thickness=4]right-> Choose : begin
Choose --> Split : yes
Split --> Merge : done
Merge --> [*]
@enduml
"##,
    )
    .expect("state pseudo-state metadata render should succeed");
    assert!(state_svg.contains("data-state-kind=\"choice\""));
    assert!(state_svg.contains("data-state-kind=\"fork\""));
    assert!(state_svg.contains("data-state-kind=\"join\""));
    assert!(state_svg.contains("data-state-kind=\"history-shallow\""));
    assert!(state_svg.contains("data-state-direction=\"right\""));
    assert!(state_svg.contains("stroke=\"#ff0000\""));
    assert!(state_svg.contains("stroke-width=\"4\""));
    assert!(state_svg.contains("stroke-dasharray=\"5 3\""));

    let activity_svg = puml::render_source_to_svg(
        r#"@startuml
start
switch (kind?)
case (fast)
:ship;
case (slow)
detach
endswitch
fork
:left;
fork again
:right;
end fork
kill
@enduml
"#,
    )
    .expect("activity metadata render should succeed");
    assert!(activity_svg.contains("data-activity-kind=\"IfStart\""));
    assert!(activity_svg.contains("data-activity-kind=\"Fork\""));
    assert!(activity_svg.contains("data-activity-kind=\"ForkAgain\""));
    assert!(activity_svg.contains("data-activity-kind=\"EndFork\""));
    assert!(activity_svg.contains(">detach<"));
    assert!(activity_svg.contains(">kill<"));
}
