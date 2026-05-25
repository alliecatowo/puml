use puml::model::{
    ChenNodeKind, FamilyNodeKind, FamilyOrientation, FamilyStyle, NormalizedDocument, StateNodeKind,
};
use puml::{ast::StatementKind, extract_metadata};

fn family_model(src: &str) -> puml::model::FamilyDocument {
    let document = puml::parse(src).expect("diagram should parse");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("family diagram should normalize")
    else {
        panic!("expected normalized family document");
    };
    model
}

fn state_model(src: &str) -> puml::model::StateDocument {
    let document = puml::parse(src).expect("state diagram should parse");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("state diagram should normalize")
    else {
        panic!("expected normalized state document");
    };
    model
}

fn msg_labels(src: &str) -> Vec<String> {
    let document = puml::parse(src).expect("preprocessed diagram should parse");
    document
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::Message(message) => message.label.clone(),
            _ => None,
        })
        .collect()
}

#[test]
fn chen_parser_normalizes_entities_attributes_and_cardinality() {
    let src = r#"@startchen
left to right direction
entity "Customer" as CUSTOMER {
  Number : INTEGER <<key>>
  Name {
    First : STRING
    Last : STRING
  }
  Bonus : REAL <<derived>>
  Email : STRING <<multi>>
}
entity MOVIE {
  Code <<key>>
}
relationship "was-rented-to" as RENTED_TO <<identifying>> {
  Date
}
RENTED_TO =1= CUSTOMER
RENTED_TO -(0,N)- MOVIE
@endchen
"#;
    let document = puml::parse(src).expect("chen diagram should parse");
    assert_eq!(document.kind, puml::ast::DiagramKind::Chen);
    assert!(document
        .statements
        .iter()
        .any(|stmt| matches!(stmt.kind, StatementKind::ChenRelation(_))));

    let NormalizedDocument::Chen(model) =
        puml::normalize_family(document).expect("chen diagram should normalize")
    else {
        panic!("expected Chen model");
    };
    assert_eq!(model.orientation, FamilyOrientation::LeftToRight);
    let customer = model
        .nodes
        .iter()
        .find(|node| node.id == "CUSTOMER")
        .expect("customer entity");
    assert_eq!(customer.kind, ChenNodeKind::Entity);
    assert_eq!(customer.label, "Customer");
    assert!(customer.attributes.iter().any(|attr| attr.key));
    assert!(customer.attributes.iter().any(|attr| attr.derived));
    assert!(customer.attributes.iter().any(|attr| attr.multivalued));
    assert!(customer
        .attributes
        .iter()
        .any(|attr| attr.label == "Name" && attr.children.len() == 2));

    let relationship = model
        .nodes
        .iter()
        .find(|node| node.id == "RENTED_TO")
        .expect("relationship node");
    assert_eq!(relationship.kind, ChenNodeKind::Relationship);
    assert!(relationship.identifying);

    assert!(model.relations.iter().any(|rel| {
        rel.from == "RENTED_TO"
            && rel.to == "CUSTOMER"
            && rel.cardinality == "1"
            && rel.total_participation
    }));
    assert!(model
        .relations
        .iter()
        .any(|rel| { rel.from == "RENTED_TO" && rel.to == "MOVIE" && rel.cardinality == "(0,N)" }));
}

#[test]
fn class_parser_normalizes_association_classes_roles_lollipops_and_styles() {
    let model = family_model(
        r##"@startuml
left to right direction
class "Order<T>" as Order {
  +id: String
}
class LineItem
class Enrollment
(Student, Course) .. Enrollment
Customer "1" :owner -[#336699,dashed,thickness=4]-> "0..*" [items] Order : places
() PaymentPort -- Order
Order --() ShipmentPort
@enduml
"##,
    );

    assert_eq!(model.orientation, FamilyOrientation::LeftToRight);
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "Order<T>" && node.alias.as_deref() == Some("Order")));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "Enrollment" && node.kind == FamilyNodeKind::Class));
    assert!(model
        .relations
        .iter()
        .any(|rel| { rel.from == "Student" && rel.to == "Course" && rel.arrow == ".." }));

    let styled = model
        .relations
        .iter()
        .find(|rel| rel.from == "Customer" && rel.to == "Order")
        .expect("styled class relation");
    assert_eq!(styled.label.as_deref(), Some("places"));
    assert_eq!(styled.left_cardinality.as_deref(), Some("1"));
    assert_eq!(styled.left_role.as_deref(), Some("owner"));
    assert_eq!(styled.right_cardinality.as_deref(), Some("0..*"));
    assert_eq!(styled.right_role.as_deref(), Some("items"));
    assert_eq!(styled.line_color.as_deref(), Some("#336699"));
    assert!(styled.dashed);
    assert_eq!(styled.thickness, Some(4));

    assert!(model
        .relations
        .iter()
        .any(|rel| rel.from == "PaymentPort" && rel.to == "Order" && rel.left_lollipop));
    assert!(model
        .relations
        .iter()
        .any(|rel| rel.from == "Order" && rel.to == "ShipmentPort" && rel.right_lollipop));
}

#[test]
fn object_parser_normalizes_member_rows_map_links_and_inline_styles() {
    let model = family_model(
        r##"@startuml
object "Cart" as cart #LightBlue
cart : total = 42
cart : status = "ready"
map Lookup #palegreen {
  sku *-> cart::total
  state *--> cart::status
}
diamond Decision #Gold
cart --> Lookup::sku : indexes
Lookup::state --> Decision
@enduml
"##,
    );

    let cart = model
        .nodes
        .iter()
        .find(|node| node.name == "Cart" && node.alias.as_deref() == Some("cart"))
        .expect("aliased cart object");
    assert_eq!(cart.kind, FamilyNodeKind::Object);
    assert_eq!(cart.fill_color.as_deref(), Some("#add8e6"));
    assert!(cart
        .members
        .iter()
        .any(|member| member.text == "total = 42"));
    assert!(cart
        .members
        .iter()
        .any(|member| member.text == "status = \"ready\""));

    let lookup = model
        .nodes
        .iter()
        .find(|node| node.name == "Lookup")
        .expect("map node");
    assert_eq!(lookup.kind, FamilyNodeKind::Map);
    assert_eq!(lookup.fill_color.as_deref(), Some("#98fb98"));
    assert!(model
        .relations
        .iter()
        .any(|rel| { rel.from == "Lookup::sku" && rel.to == "cart::total" && rel.arrow == "*->" }));
    assert!(model.relations.iter().any(|rel| {
        rel.from == "Lookup::state" && rel.to == "cart::status" && rel.arrow == "*-->"
    }));

    let decision = model
        .nodes
        .iter()
        .find(|node| node.name == "Decision")
        .expect("diamond node");
    assert_eq!(decision.kind, FamilyNodeKind::Diamond);
    assert_eq!(decision.fill_color.as_deref(), Some("#ffd700"));
    assert!(model
        .relations
        .iter()
        .any(|rel| rel.from == "cart" && rel.to == "Lookup::sku"));
}

#[test]
fn deployment_parser_normalizes_scoped_nodes_tags_styles_and_visibility() {
    let model = family_model(
        r##"@startuml
skinparam nodeBackgroundColor HoneyDew
right to left direction
node "Web Tier" as web #AliceBlue $prod
cloud "CDN" as cdn
folder "Assets" as assets $internal
package "VPC" {
  node "API" as api #LightYellow
  database "Primary DB" as db
  artifact "service.jar" as jar
  api -[#red,dashed,thickness=3]-> db : SQL
  api --> jar
}
web -[#green,bold]-> VPC::api : HTTP
cdn --> web
hide $internal
@enduml
"##,
    );

    assert_eq!(model.orientation, FamilyOrientation::RightToLeft);
    let Some(FamilyStyle::Component(style)) = &model.family_style else {
        panic!("deployment diagrams should carry component/deployment style");
    };
    assert_eq!(style.background_color, "#f0fff0");

    assert!(model.nodes.iter().any(|node| node.name == "Web Tier"
        && node.alias.as_deref() == Some("web")
        && node.kind == FamilyNodeKind::Node
        && node.fill_color.as_deref() == Some("#f0f8ff")));
    assert!(model.nodes.iter().any(|node| node.name == "VPC::api"
        && node.label.as_deref() == Some("API")
        && node.kind == FamilyNodeKind::Node
        && node.fill_color.as_deref() == Some("#ffffe0")));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "VPC::db" && node.kind == FamilyNodeKind::Database));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "VPC::jar" && node.kind == FamilyNodeKind::Artifact));
    assert!(!model.nodes.iter().any(|node| node.name == "Assets"));

    let sql = model
        .relations
        .iter()
        .find(|rel| rel.from == "VPC::api" && rel.to == "VPC::db")
        .expect("scoped SQL relation");
    assert_eq!(sql.label.as_deref(), Some("SQL"));
    assert_eq!(sql.line_color.as_deref(), Some("#ff0000"));
    assert!(sql.dashed);
    assert_eq!(sql.thickness, Some(3));

    let http = model
        .relations
        .iter()
        .find(|rel| rel.from == "web" && rel.to == "VPC::api")
        .expect("external scoped relation");
    assert_eq!(http.line_color.as_deref(), Some("#008000"));
    assert_eq!(http.thickness, Some(3));
}

#[test]
fn state_parser_normalizes_regions_bare_actions_history_and_final_split() {
    let model = state_model(
        r##"@startuml
skinparam stateBackgroundColor MintCream
state Running #back:lightblue;line:navy;line.bold;text:white {
  [*] --> Idle
  entry / boot
  do / poll
  exit / cleanup
  --
  state Worker <<fork>>
  Worker --> [H]
}
state Choice <<choice>>
state Done <<end>>
[*] --> Running
Running --> Choice : evaluate
Choice -[#orange,dashed]down-> Done : finish
Done --> [*]
Running --> Running[H*] : resume deep
@enduml
"##,
    );

    assert_eq!(model.state_style.background_color, "#f5fffa");
    assert!(model.nodes.iter().any(|node| node.name == "[*]__end"));
    let running = model
        .nodes
        .iter()
        .find(|node| node.name == "Running")
        .expect("Running composite");
    assert_eq!(running.style.fill_color.as_deref(), Some("lightblue"));
    assert_eq!(running.style.border_color.as_deref(), Some("navy"));
    assert_eq!(running.style.border_thickness, Some(3));
    assert_eq!(running.style.text_color.as_deref(), Some("white"));
    assert_eq!(
        running.regions.len(),
        2,
        "-- should split composite regions"
    );
    assert!(running
        .internal_actions
        .iter()
        .any(|action| action.kind == "entry" && action.action == "boot"));
    assert!(running
        .internal_actions
        .iter()
        .any(|action| action.kind == "do" && action.action == "poll"));
    assert!(running
        .internal_actions
        .iter()
        .any(|action| action.kind == "exit" && action.action == "cleanup"));
    assert!(running
        .regions
        .iter()
        .flatten()
        .any(|node| { node.name == "Running[H*]" && node.kind == StateNodeKind::HistoryDeep }));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "Choice" && node.kind == StateNodeKind::Choice));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.name == "Done" && node.kind == StateNodeKind::End));
    let finish = model
        .transitions
        .iter()
        .find(|transition| transition.from == "Choice" && transition.to == "Done")
        .expect("styled state transition");
    assert_eq!(finish.line_color.as_deref(), Some("#ffa500"));
    assert!(finish.dashed);
    assert_eq!(finish.direction.as_deref(), Some("down"));
}

#[test]
fn timing_parser_normalizes_anchors_clock_math_ranges_orders_and_controls() {
    let model = family_model(
        r##"@startuml
skinparam timingAxisColor DarkSlateGray
mode compact
manual time-axis
hide time-axis
scale 10 as 50 pixels
concise "Request Phase" as REQ
robust BUS
BUS has "Idle State" as idle, "Run State" as run, fault
binary FLAG
clock CLK with period 5 pulse 2 offset 1
analog "Temperature" between 0 and 100 as TEMP
@0 as :start
@CLK*2 REQ is {idle} #LightGreen : ignored annotation
@+3 BUS is "Run State"
@:start+10 FLAG is on
highlight :start+5 to CLK*3 #Gold;line:DimGrey : warmup
REQ@:start+5 -> BUS@CLK*3 : handoff
@enduml
"##,
    );

    let Some(FamilyStyle::Timing(style)) = &model.family_style else {
        panic!("timing diagram should carry timing style");
    };
    assert_eq!(style.axis_color, "#2f4f4f");

    let req = model
        .nodes
        .iter()
        .find(|node| node.name == "REQ" && node.kind == FamilyNodeKind::TimingConcise)
        .expect("REQ signal");
    assert_eq!(req.label.as_deref(), Some("Request Phase"));
    let clk = model
        .nodes
        .iter()
        .find(|node| node.name == "CLK" && node.kind == FamilyNodeKind::TimingClock)
        .expect("CLK signal");
    assert!(clk
        .members
        .iter()
        .any(|member| member.text == "period 5 pulse 2 offset 1"));
    let temp = model
        .nodes
        .iter()
        .find(|node| node.name == "TEMP")
        .expect("analog signal");
    assert_eq!(temp.kind, FamilyNodeKind::TimingRobust);
    assert!(temp
        .members
        .iter()
        .any(|member| member.text == "__timing:analog"));
    assert!(temp
        .members
        .iter()
        .any(|member| member.text == "__timing:analog_between 0 100"));

    let bus = model
        .nodes
        .iter()
        .find(|node| node.name == "BUS" && node.kind == FamilyNodeKind::TimingRobust)
        .expect("BUS signal");
    assert!(bus
        .members
        .iter()
        .any(|member| member.text == "__timing:order:idle,run,fault"));

    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.name == "10"
            && node.alias.as_deref() == Some("REQ")
            && node
                .members
                .iter()
                .any(|member| member.text == "idle #LightGreen : ignored annotation")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.name == "13"
            && node.alias.as_deref() == Some("BUS")
            && node.members.iter().any(|member| member.text == "Run State")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.name == "10"
            && node.alias.as_deref() == Some("FLAG")
            && node.members.iter().any(|member| member.text == "high")
    }));
    assert!(model.nodes.iter().any(|node| {
        node.kind == FamilyNodeKind::TimingEvent
            && node.name == "5"
            && node
                .label
                .as_deref()
                .is_some_and(|label| label.starts_with("range:15:warmup"))
    }));

    let handoff = model
        .relations
        .iter()
        .find(|rel| rel.label.as_deref() == Some("handoff"))
        .expect("timing relation");
    assert_eq!(handoff.from, "REQ@5");
    assert_eq!(handoff.to, "BUS@15");
}

#[test]
fn preproc_builtins_cover_deterministic_stubs_paths_and_state_queries() {
    let labels = msg_labels(
        r#"@startuml
!$path = "/tmp/puml/demo/file.puml"
!$value = "set"
!function Decorate($x)
!return %upper($x)
!endfunction
!procedure $Emit($who)
A -> $who : emitted
!endprocedure
%invoke_procedure("$Emit", "Bob")
A -> B : %dirpath($path)
A -> B : %filename($path)
A -> B : %filenameroot($path)
A -> B : %feature("stdlib")
A -> B : %get_variable_value("value")
A -> B : %variable_exists("value")
A -> B : %function_exists("Decorate")
A -> B : %procedure_exists("$Emit")
A -> B : %call_user_func("Decorate", "ok")
A -> B : %random()/%uuid()/%strlen(%date())/%strlen(%getenv("PUML_TEST_ENV"))
A -> B : %retrieve_procedure_return()/%set_variable_value("value", "ignored")
@enduml
"#,
    );

    assert_eq!(
        labels,
        vec![
            "emitted",
            "/tmp/puml/demo",
            "file.puml",
            "file",
            "false",
            "\"set\"",
            "true",
            "true",
            "true",
            "OK",
            "0/00000000-0000-0000-0000-000000000000/10/0",
            "/",
        ]
    );
}

#[test]
fn preproc_includes_cover_many_glob_subblocks_import_and_unsafe_builtin_errors() {
    use puml::parser::{parse_with_options, ParseOptions};
    use std::fs;

    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    fs::write(root.join("part_a.puml"), "Alice -> Bob : from-a\n").expect("write a");
    fs::write(root.join("part_b.puml"), "Alice -> Bob : from-b\n").expect("write b");
    fs::write(
        root.join("tagged.puml"),
        "@startuml\n!startsub FLOW\nAlice -> Bob : from-sub\n!endsub\n@enduml\n",
    )
    .expect("write tagged");
    let stdlib = root.join("stdlib");
    fs::create_dir_all(&stdlib).expect("create stdlib");
    fs::write(
        stdlib.join("lib.puml"),
        "!procedure Lib($x)\nA -> $x : imported\n!endprocedure\n",
    )
    .expect("write import");

    let options = ParseOptions {
        include_root: Some(root.to_path_buf()),
        ..ParseOptions::default()
    };
    let document = parse_with_options(
        "@startuml\n!include_many part_*.puml\n!includesub tagged.puml!FLOW\n!import lib\n!Lib(Bob)\n@enduml\n",
        &options,
    )
    .expect("include/import diagram should parse");
    let labels = document
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::Message(message) => message.label.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["from-a", "from-b", "from-sub", "imported"]);

    let err = puml::parse("@startuml\nA -> B : %load_json(\"local.json\")\n@enduml\n")
        .expect_err("unsafe IO builtin should be rejected");
    assert!(err.message.contains("E_PREPROC_UNSAFE_BUILTIN"));
}

#[test]
fn preproc_errors_cover_include_directive_and_callable_signature_edges() {
    use puml::parser::{parse_with_options, ParseOptions};
    use std::fs;

    fn assert_parse_error(src: &str, code: &str) {
        let err = puml::parse(src).expect_err("source should fail");
        assert!(
            err.message.contains(code),
            "expected {code}, got {:?}",
            err.message
        );
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    fs::write(
        root.join("tagged.puml"),
        "!startsub FLOW\nAlice -> Bob : from-sub\n!endsub\n",
    )
    .expect("write tagged");
    let options = ParseOptions {
        include_root: Some(root.to_path_buf()),
        ..ParseOptions::default()
    };

    for (src, code) in [
        (
            "@startuml\n!include_many\n@enduml\n",
            "E_INCLUDE_PATH_REQUIRED",
        ),
        (
            "@startuml\n!include_many /tmp/nope.puml\n@enduml\n",
            "E_INCLUDE_ABSOLUTE_PATH",
        ),
        (
            "@startuml\n!includesub tagged.puml\n@enduml\n",
            "E_INCLUDESUB_TAG_REQUIRED",
        ),
        (
            "@startuml\n!include tagged.puml!MISSING\n@enduml\n",
            "E_INCLUDE_TAG_NOT_FOUND",
        ),
    ] {
        let err = parse_with_options(src, &options).expect_err("source should fail");
        assert!(
            err.message.contains(code),
            "expected {code}, got {:?}",
            err.message
        );
    }

    assert_parse_error(
        "@startuml\n!import /tmp/nope.puml\n@enduml\n",
        "E_IMPORT_ABSOLUTE_PATH",
    );

    for (src, code) in [
        (
            "@startuml\n!function Broken\n!return 1\n!endfunction\n@enduml\n",
            "E_PREPROC_SIGNATURE",
        ),
        (
            "@startuml\n!function ($x)\n!return $x\n!endfunction\n@enduml\n",
            "E_PREPROC_SIGNATURE",
        ),
        (
            "@startuml\n!function Broken(=1)\n!return 1\n!endfunction\n@enduml\n",
            "E_PREPROC_SIGNATURE",
        ),
    ] {
        assert_parse_error(src, code);
    }
}

#[test]
fn preproc_builtins_cover_legacy_json_path_fallbacks() {
    let labels = msg_labels(
        r#"@startuml
!$legacy = {"users": ["ann", {"name":"bob"}], "flag": true, }
A -> B : %get_json_attribute($legacy, "users[1].name")
A -> B : %json_key_exists($legacy, "users")
A -> B : %json_contains_value("[one,two,three]", "two")
@enduml
"#,
    );

    assert_eq!(labels, vec!["bob", "true", "true"]);
}

#[test]
fn metadata_extracts_sequence_family_pages_timeline_state_and_simple_families() {
    fn metadata_for(src: &str) -> puml::metadata::DiagramMetadata {
        let document = puml::parse(src).expect("metadata source should parse");
        let model = puml::normalize_family(document.clone()).expect("metadata source normalizes");
        extract_metadata(&document, &model)
    }

    let sequence = metadata_for(
        r##"@startuml
title Sequence metadata
skinparam responseMessageBelowArrow true
skinparam unknownSequenceKey nope
!theme plain
Alice -> Bob : hello
note right: note
alt ok
Alice -> Bob : grouped
end
newpage Next
Bob --> Alice : reply
@enduml
"##,
    );
    assert_eq!(sequence.family, "sequence");
    assert_eq!(sequence.title.as_deref(), Some("Sequence metadata"));
    assert_eq!(sequence.counts["participants"], 2);
    assert_eq!(sequence.counts["messages"], 3);
    assert_eq!(sequence.counts["notes"], 1);
    assert_eq!(sequence.counts["groups"], 1);
    assert_eq!(sequence.counts["pages"], 2);
    assert_eq!(sequence.pages[1].title.as_deref(), Some("Next"));
    assert_eq!(sequence.themes, vec!["plain"]);
    assert!(sequence
        .skinparams
        .iter()
        .any(|param| param.key == "responseMessageBelowArrow"));

    let family_pages =
        metadata_for("@startuml\nclass A\nnewpage Page 2\nclass B\nA --> B\n@enduml\n");
    assert_eq!(family_pages.family, "class");
    assert_eq!(family_pages.counts["pages"], 2);
    assert_eq!(family_pages.counts["nodes"], 2);
    assert_eq!(family_pages.pages[1].title.as_deref(), Some("Page 2"));

    let timeline = metadata_for(
        "@startgantt\nProject starts 2026-05-01\n[Build] lasts 3 days\n[Launch] happens on 2026-05-10\n@endgantt\n",
    );
    assert_eq!(timeline.family, "gantt");
    assert_eq!(timeline.counts["tasks"], 1);
    assert_eq!(timeline.counts["milestones"], 1);

    let state = metadata_for("@startuml\nstate A\n[*] --> A\n@enduml\n");
    assert_eq!(state.family, "state");
    assert_eq!(state.counts["nodes"], 2);
    assert_eq!(state.counts["transitions"], 1);

    let cases = [
        ("@startjson\n{\"a\":1}\n@endjson\n", "json", "nodes"),
        ("@startyaml\na: 1\n@endyaml\n", "yaml", "nodes"),
        (
            "@startnwdiag\nnetwork dmz {\n  address = \"10.0.0.0/24\"\n  web01 [address = \"10.0.0.10\"]\n}\n@endnwdiag\n",
            "nwdiag",
            "networks",
        ),
        (
            "@startarchimate\narchimate \"Capability\" as cap <<strategy>>\n@endarchimate\n",
            "archimate",
            "elements",
        ),
        ("@startregex\n^foo$\n@endregex\n", "regex", "patterns"),
        ("@startebnf\nexpr = \"id\" ;\n@endebnf\n", "ebnf", "rules"),
        ("@startmath\nx^2\n@endmath\n", "math", "body_bytes"),
        (
            "@startsdl\nstate Start <<start>>\n@endsdl\n",
            "sdl",
            "states",
        ),
        (
            "@startditaa\n+---+\n| A |\n+---+\n@endditaa\n",
            "ditaa",
            "body_bytes",
        ),
        (
            "@startchart\npie chart\nA : 10\n@endchart\n",
            "chart",
            "data_points",
        ),
        (
            "@startwire\ncomponent Panel [80x60] right:P\n@endwire\n",
            "wire",
            "components",
        ),
    ];
    for (src, family, count_key) in cases {
        let metadata = metadata_for(src);
        assert_eq!(metadata.family, family);
        assert!(
            metadata.counts[count_key] > 0,
            "expected {family}.{count_key} to be counted: {metadata:?}"
        );
    }
}
